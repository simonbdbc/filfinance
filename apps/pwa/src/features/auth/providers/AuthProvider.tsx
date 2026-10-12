import {
  AuthSessionClient,
  SessionExpiredError,
} from "@shared/api/authSession";
import {
  createContext,
  type PropsWithChildren,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { Platform } from "react-native";

import {
  betterAuthClient,
  isBetterAuthConfigured,
} from "@shared/api/betterAuthClient";
import {
  emailVerificationReturnUrl,
  passwordResetRedirectUrl,
} from "../lib/authRedirects";
import { readJwtMetadata } from "../lib/jwtMetadata";
import { readPublicEnv } from "@shared/config/runtimeEnv";

type AuthStatus = "loading" | "authenticated" | "anonymous";

type AuthUser = {
  id: string;
  email?: string | null;
  name?: string | null;
  username?: string | null;
};

type AuthContextValue = {
  isConfigured: boolean;
  isLoading: boolean;
  isSubmitting: boolean;
  session: AuthSessionClient | null;
  authStatus: AuthStatus;
  user: AuthUser | null;
  authSession: AuthSessionClient | null;
  error: string | null;
  notice: string | null;
  signIn: (email: string, password: string) => Promise<void>;
  signUp: (email: string, password: string, name?: string) => Promise<void>;
  requestPasswordReset: (email: string) => Promise<void>;
  resetPassword: (newPassword: string, token: string) => Promise<boolean>;
  signOut: () => Promise<void>;
  refreshSession: () => Promise<string | null>;
  clearError: () => void;
  clearNotice: () => void;
};

const AuthContext = createContext<AuthContextValue | null>(null);

export function AuthProvider({ children }: PropsWithChildren) {
  if (!isBetterAuthConfigured) {
    return <UnconfiguredAuthProvider>{children}</UnconfiguredAuthProvider>;
  }

  return <BetterAuthProvider>{children}</BetterAuthProvider>;
}

function UnconfiguredAuthProvider({ children }: PropsWithChildren) {
  const [error, setError] = useState<string | null>(null);

  const value = useMemo<AuthContextValue>(
    () => ({
      isConfigured: false,
      isLoading: false,
      isSubmitting: false,
      session: null,
      user: null,
      authSession: null,
      authStatus: "anonymous",
      error,
      notice: null,
      signIn: async () => {
        setError("Configuration Better Auth manquante.");
      },
      signUp: async () => {
        setError("Configuration Better Auth manquante.");
      },
      requestPasswordReset: async () => {
        setError("Configuration Better Auth manquante.");
      },
      resetPassword: async () => {
        setError("Configuration Better Auth manquante.");
        return false;
      },
      signOut: async () => undefined,
      refreshSession: async () => null,
      clearError: () => setError(null),
      clearNotice: () => undefined,
    }),
    [error],
  );

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}

type BetterAuthUser = {
  id: string;
  email?: string | null;
  name?: string | null;
  username?: string | null;
};

type BetterAuthSessionData = {
  user?: BetterAuthUser | null;
};

type BetterAuthSessionState = {
  data?: BetterAuthSessionData | null;
  isPending?: boolean;
  refetch?: () => Promise<unknown>;
};

type AccessTokenStatus = "idle" | "loading" | "ready" | "error";

function BetterAuthProvider({ children }: PropsWithChildren) {
  const sessionState = betterAuthClient.useSession() as BetterAuthSessionState;
  const [authSession, setAuthSession] = useState<AuthSessionClient | null>(
    null,
  );
  const clientRef = useRef<AuthSessionClient | null>(null);
  const [accessTokenStatus, setAccessTokenStatus] =
    useState<AccessTokenStatus>("idle");
  const [sessionRestart, setSessionRestart] = useState(0);
  const [isSessionRevoked, setIsSessionRevoked] = useState(false);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const isSigningOutRef = useRef(false);
  const configuredPwaOrigin = readPublicEnv("EXPO_PUBLIC_FILFINANCE_PWA_URL");

  const sessionUser = sessionState.data?.user ?? null;
  const user = useMemo(
    () => (sessionUser ? userFromBetterAuth(sessionUser) : null),
    [sessionUser],
  );

  const refetchSessionRef = useRef(sessionState.refetch);
  refetchSessionRef.current = sessionState.refetch;
  const refreshSession = useCallback(async () => {
    await refetchSessionRef.current?.();
    try {
      return (await clientRef.current?.refresh()) ?? null;
    } catch {
      return null;
    }
  }, []);

  useEffect(() => {
    clientRef.current?.close();
    clientRef.current = null;
    setAuthSession(null);
    setAccessTokenStatus("idle");
    if (!sessionUser || isSigningOutRef.current) return;
    setIsSessionRevoked(false);
    const client = new AuthSessionClient(
      sessionUser.id,
      async () => {
        for (let attempt = 1; attempt <= MAX_ACCESS_TOKEN_ATTEMPTS; attempt++) {
          client.assertActive();
          try {
            const result = await betterAuthClient.token();
            if (result.error?.status === 401) throw new SessionExpiredError();
            if (!result.error && result.data?.token) {
              const metadata = readJwtMetadata(result.data.token);
              if (!metadata?.subject || !metadata.expiresAt)
                throw new Error("Invalid token metadata");
              return {
                token: result.data.token,
                subject: metadata.subject,
                expiresAt: metadata.expiresAt,
              };
            }
          } catch (error) {
            if (error instanceof SessionExpiredError) throw error;
          }
          if (attempt < MAX_ACCESS_TOKEN_ATTEMPTS)
            await wait(ACCESS_TOKEN_RETRY_DELAYS_MS[attempt - 1]);
        }
        throw new Error("AUTH_TEMPORARILY_UNAVAILABLE");
      },
      (state) => {
        if (clientRef.current !== client) return;
        if (state === "ready") {
          setAuthSession(client);
          setAccessTokenStatus("ready");
          setError(null);
        } else if (state === "expired") {
          setIsSessionRevoked(true);
          setAuthSession(null);
          setAccessTokenStatus("error");
        } else {
          setAccessTokenStatus("error");
          setError("Authentification temporairement indisponible. Réessayez.");
        }
      },
    );
    clientRef.current = client;
    setAccessTokenStatus("loading");
    void client.refresh().catch(() => {});
    return () => {
      client.close();
      if (clientRef.current === client) clientRef.current = null;
    };
  }, [sessionUser?.id, sessionRestart]);

  const signIn = useCallback(
    async (email: string, password: string) => {
      isSigningOutRef.current = false;
      setIsSessionRevoked(false);
      setIsSubmitting(true);
      setError(null);
      setNotice(null);

      try {
        const result = await betterAuthClient.signIn.email({
          email,
          password,
        });
        if (result.error) {
          if (authErrorCode(result.error) === "EMAIL_NOT_VERIFIED") {
            const callbackURL = emailVerificationReturnUrl(
              Platform.OS,
              currentWebOrigin(),
              configuredPwaOrigin,
            );
            if (!callbackURL) {
              setError(
                "URL HTTPS de la PWA manquante pour envoyer le lien de confirmation.",
              );
              return;
            }
            const verificationResult =
              await betterAuthClient.sendVerificationEmail({
                email,
                callbackURL,
              });
            if (verificationResult.error) {
              setError(
                authErrorMessage(
                  verificationResult.error,
                  "Impossible d’envoyer le lien de confirmation.",
                ),
              );
              return;
            }
            setNotice(
              "Adresse non vérifiée. Un nouveau lien de confirmation a été envoyé.",
            );
            return;
          }
          setError(authErrorMessage(result.error, "Connexion refusée."));
          return;
        }
        await refreshSession();
      } catch {
        setError("Impossible d’ouvrir la session.");
      } finally {
        setIsSubmitting(false);
      }
    },
    [configuredPwaOrigin, refreshSession],
  );

  const signUp = useCallback(
    async (email: string, password: string, name?: string) => {
      isSigningOutRef.current = false;
      setIsSessionRevoked(false);
      setIsSubmitting(true);
      setError(null);
      setNotice(null);

      try {
        const callbackURL = emailVerificationReturnUrl(
          Platform.OS,
          currentWebOrigin(),
          configuredPwaOrigin,
        );
        if (!callbackURL) {
          setError(
            "URL HTTPS de la PWA manquante pour envoyer le lien de confirmation.",
          );
          return;
        }
        const result = await betterAuthClient.signUp.email({
          email,
          password,
          name: name?.trim() || email,
          callbackURL,
        });
        if (result.error) {
          setError(authErrorMessage(result.error, "Inscription refusée."));
          return;
        }
        const verificationResult = await betterAuthClient.sendVerificationEmail(
          {
            email,
            callbackURL,
          },
        );
        if (verificationResult.error) {
          setError(
            authErrorMessage(
              verificationResult.error,
              "Compte enregistré, mais le lien de confirmation n’a pas pu être envoyé.",
            ),
          );
          return;
        }
        setNotice(
          "Si ce compte doit être vérifié, un lien de confirmation a été envoyé.",
        );
      } catch {
        setError("Impossible de créer le compte.");
      } finally {
        setIsSubmitting(false);
      }
    },
    [configuredPwaOrigin],
  );

  const requestPasswordReset = useCallback(
    async (email: string) => {
      setIsSubmitting(true);
      setError(null);
      setNotice(null);

      try {
        const redirectTo = passwordResetRedirectUrl(
          Platform.OS,
          currentWebOrigin(),
          configuredPwaOrigin,
        );
        if (!redirectTo) {
          setError(
            "URL HTTPS de la PWA manquante pour réinitialiser le mot de passe.",
          );
          return;
        }
        const result = await betterAuthClient.requestPasswordReset({
          email,
          redirectTo,
        });
        if (result.error) {
          setError(
            result.error.message ?? "Demande de réinitialisation refusée.",
          );
          return;
        }
        setNotice(
          "Si ce compte existe, un lien de réinitialisation a été envoyé.",
        );
      } catch {
        setError("Impossible de demander la réinitialisation du mot de passe.");
      } finally {
        setIsSubmitting(false);
      }
    },
    [configuredPwaOrigin],
  );

  const resetPassword = useCallback(
    async (newPassword: string, token: string) => {
      setIsSubmitting(true);
      setError(null);
      setNotice(null);

      try {
        const result = await betterAuthClient.resetPassword({
          newPassword,
          token,
        });
        if (result.error) {
          setError(
            authErrorMessage(
              result.error,
              "Lien de réinitialisation invalide.",
            ),
          );
          return false;
        }
        setNotice("Mot de passe modifié. Vous pouvez maintenant vous connecter.");
        return true;
      } catch {
        setError("Impossible de modifier le mot de passe.");
        return false;
      } finally {
        setIsSubmitting(false);
      }
    },
    [],
  );

  const signOut = useCallback(async () => {
    isSigningOutRef.current = true;
    setIsSubmitting(true);
    setError(null);
    setNotice(null);
    clientRef.current?.close();
    clientRef.current = null;
    setIsSessionRevoked(true);
    setAuthSession(null);
    setAccessTokenStatus("idle");

    try {
      const result = await betterAuthClient.signOut();
      if (result.error) {
        isSigningOutRef.current = false;
        setIsSessionRevoked(false);
        setError(result.error.message ?? "Déconnexion refusée.");
        setSessionRestart((value) => value + 1);
        return;
      }
    } catch {
      isSigningOutRef.current = false;
      setIsSessionRevoked(false);
      setError("Impossible de fermer la session.");
      setSessionRestart((value) => value + 1);
    } finally {
      setIsSubmitting(false);
    }
  }, []);

  const clearError = useCallback(() => setError(null), []);
  const clearNotice = useCallback(() => setNotice(null), []);
  const effectiveUser = isSessionRevoked ? null : user;
  const effectiveAuthSession =
    effectiveUser && authSession?.userId === effectiveUser.id
      ? authSession
      : null;
  const isLoading =
    !effectiveAuthSession &&
    (Boolean(sessionState.isPending) ||
      (Boolean(effectiveUser) &&
        (accessTokenStatus === "idle" || accessTokenStatus === "loading")));
  const authStatus: AuthStatus = effectiveAuthSession
    ? "authenticated"
    : isLoading
      ? "loading"
      : "anonymous";

  const value = useMemo<AuthContextValue>(
    () => ({
      isConfigured: isBetterAuthConfigured,
      isLoading,
      isSubmitting,
      session: effectiveAuthSession,
      authStatus,
      user: effectiveUser,
      authSession: effectiveAuthSession,
      error,
      notice,
      signIn,
      signUp,
      requestPasswordReset,
      resetPassword,
      signOut,
      refreshSession,
      clearError,
      clearNotice,
    }),
    [
      effectiveAuthSession,
      authStatus,
      isLoading,
      clearError,
      clearNotice,
      error,
      effectiveUser,
      isSubmitting,
      notice,
      refreshSession,
      requestPasswordReset,
      resetPassword,
      signIn,
      signOut,
      signUp,
      user,
    ],
  );

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}

function userFromBetterAuth(claims: BetterAuthUser): AuthUser {
  return {
    id: claims.id,
    email: claims.email,
    name: claims.name,
    username: claims.username,
  };
}

function currentWebOrigin() {
  if (
    Platform.OS === "web" &&
    typeof window !== "undefined" &&
    window.location?.origin
  ) {
    return window.location.origin;
  }
  return null;
}

function authErrorCode(error: unknown): string | null {
  if (typeof error !== "object" || error === null || !("code" in error))
    return null;
  return typeof error.code === "string" ? error.code.toUpperCase() : null;
}

function authErrorMessage(error: unknown, fallback: string): string {
  const code = authErrorCode(error);
  if (code === "PASSWORD_TOO_SHORT") {
    return "Le mot de passe doit contenir au moins 10 caractères.";
  }
  if (code === "PASSWORD_TOO_LONG") {
    return "Le mot de passe ne doit pas dépasser 128 caractères.";
  }
  if (typeof error === "object" && error !== null && "message" in error) {
    return typeof error.message === "string" && error.message
      ? error.message
      : fallback;
  }
  return fallback;
}

const MAX_ACCESS_TOKEN_ATTEMPTS = 3;
const ACCESS_TOKEN_RETRY_DELAYS_MS = [250, 750] as const;

function wait(delayMs: number) {
  return new Promise<void>((resolve) => {
    setTimeout(resolve, delayMs);
  });
}

export function useAuth() {
  const context = useContext(AuthContext);
  if (!context) {
    throw new Error("useAuth doit être utilisé dans AuthProvider");
  }

  return context;
}
