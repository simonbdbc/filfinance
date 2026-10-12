import { TextInput } from "@shared/ui/TextInput";
import { Text } from "@shared/ui/Text";
import {
  type ComponentProps,
  type PropsWithChildren,
  useEffect,
  useState,
} from "react";
import { useLocalSearchParams } from "expo-router";
import { View } from "react-native";

import { AuthBrandHeader, AuthPageShell } from "./AuthPageShell";
import { AuthTokenRecoveryScreen } from "./AuthTokenRecoveryScreen";
import { isValidNewPassword, passwordValidity } from "../lib/passwordPolicy";
import {
  clearResetCallbackFromBrowserUrl,
  readResetCallback,
} from "../lib/resetLink";
import { useAuth } from "../providers/AuthProvider";
import { useThemeTokens } from "@shared/theme/theme";
import { Button } from "@shared/ui/Button";
import { Surface, SectionHeader } from "@shared/ui/Surface";
import { SubtleText } from "@shared/ui/Typography";
import { LegalLinks } from "@shared/legal/LegalLinks";

export function AuthGate({ children }: PropsWithChildren) {
  const {
    isConfigured,
    isLoading,
    isSubmitting,
    session,
    user,
    error,
    notice,
    signIn,
    signUp,
    requestPasswordReset,
    resetPassword,
    clearError,
    clearNotice,
  } = useAuth();
  const params = useLocalSearchParams<{
    error?: string | string[];
    mode?: string | string[];
    token?: string | string[];
  }>();
  const callbackToken = firstParam(params.token);
  const callbackError = firstParam(params.error);
  const initialResetCallback = readResetCallback(callbackToken, callbackError);
  const [resetToken, setResetToken] = useState<string | undefined>(
    initialResetCallback.resetToken,
  );
  const [resetLinkError, setResetLinkError] = useState<string | null>(
    initialResetCallback.resetLinkError,
  );
  const [authMode, setAuthMode] = useState<AuthMode>(
    initialResetCallback.authMode,
  );
  const [email, setEmail] = useState("");
  const [name, setName] = useState("");
  const [password, setPassword] = useState("");
  const tokens = useThemeTokens();
  const requiresNewPassword =
    authMode === "sign-up" || authMode === "reset-password";
  const currentPasswordValidity = passwordValidity(password);
  const hasValidPassword = requiresNewPassword
    ? isValidNewPassword(password)
    : authMode === "forgot-password" || password.length > 0;

  useEffect(() => {
    if (!callbackToken && !callbackError) return;

    if (callbackError) {
      setResetToken(undefined);
      setResetLinkError(
        readResetCallback(undefined, callbackError).resetLinkError,
      );
      setAuthMode("sign-in");
    } else if (callbackToken) {
      setResetToken(callbackToken);
      setResetLinkError(null);
      setAuthMode("reset-password");
    }
    clearResetCallbackFromBrowserUrl();
    // Expo peut réécrire son URL initiale après cet effet. Nettoyer aussi après
    // le montage, sans navigation avant que son conteneur soit prêt.
    if (
      typeof window !== "undefined" &&
      typeof requestAnimationFrame === "function"
    ) {
      const frame = requestAnimationFrame(() =>
        clearResetCallbackFromBrowserUrl(),
      );
      return () => cancelAnimationFrame(frame);
    }
  }, [callbackError, callbackToken, isLoading]);

  if (isLoading) {
    return (
      <AuthPageShell>
        <AuthBrandHeader />
        <Surface>
          <SectionHeader
            title="Fil Finance"
            description="Restauration de la session."
          />
        </Surface>
      </AuthPageShell>
    );
  }

  if (session) {
    return <>{children}</>;
  }

  if (user) {
    return <AuthTokenRecoveryScreen />;
  }

  return (
    <AuthPageShell>
      <View className="w-full max-w-full min-w-0 gap-3">
        <AuthBrandHeader />

        {!isConfigured ? (
          <Surface variant="danger">
            <SectionHeader
              title="Configuration Better Auth absente"
              description="Renseigner EXPO_PUBLIC_BETTER_AUTH_URL."
            />
          </Surface>
        ) : null}

        <Surface>
          <SectionHeader
            title={authTitle(authMode)}
            description={authDescription(authMode)}
          />

          {notice ? <NoticeText>{notice}</NoticeText> : null}
          {resetLinkError ? <ErrorText>{resetLinkError}</ErrorText> : null}
          {error ? <ErrorText>{error}</ErrorText> : null}

          <View className="w-full max-w-full min-w-0 gap-3 pt-3">
            {authMode === "sign-up" ? (
              <AuthInput
                autoCapitalize="words"
                onChangeText={setName}
                placeholder="Nom"
                tokens={tokens}
                value={name}
              />
            ) : null}
            {authMode !== "reset-password" ? (
              <AuthInput
                autoCapitalize="none"
                autoComplete="email"
                keyboardType="email-address"
                onChangeText={setEmail}
                placeholder="Adresse e-mail"
                tokens={tokens}
                value={email}
              />
            ) : null}
            {authMode !== "forgot-password" ? (
              <>
                <AuthInput
                  accessibilityHint={
                    requiresNewPassword
                      ? "Le mot de passe doit contenir entre 10 et 128 caractères."
                      : undefined
                  }
                  autoCapitalize="none"
                  autoComplete={
                    authMode === "sign-in" ? "current-password" : "new-password"
                  }
                  onChangeText={setPassword}
                  placeholder={
                    authMode === "reset-password"
                      ? "Nouveau mot de passe"
                      : "Mot de passe"
                  }
                  secureTextEntry
                  style={
                    requiresNewPassword &&
                    currentPasswordValidity !== "empty" &&
                    currentPasswordValidity !== "valid"
                      ? { borderColor: tokens.colors.dangerText }
                      : undefined
                  }
                  tokens={tokens}
                  value={password}
                />
                {requiresNewPassword ? (
                  <PasswordRequirement validity={currentPasswordValidity} />
                ) : null}
              </>
            ) : null}
            {authMode === "sign-up" ? <LegalLinks registration /> : null}
            <Button
              disabled={
                !isConfigured ||
                isSubmitting ||
                (authMode !== "reset-password" && !email.trim()) ||
                !hasValidPassword ||
                (authMode === "reset-password" && !resetToken)
              }
              onPress={() => {
                clearError();
                clearNotice();
                if (authMode === "sign-up") {
                  void signUp(email.trim(), password, name);
                  return;
                }
                if (authMode === "forgot-password") {
                  void requestPasswordReset(email.trim());
                  return;
                }
                if (authMode === "reset-password" && resetToken) {
                  void resetPassword(password, resetToken).then((success) => {
                    if (success) {
                      setPassword("");
                      setResetToken(undefined);
                      setAuthMode("sign-in");
                    }
                  });
                  return;
                }
                void signIn(email.trim(), password);
              }}
              variant="primary"
            >
              {isSubmitting ? "Traitement..." : authSubmitLabel(authMode)}
            </Button>
            <Button
              disabled={isSubmitting}
              onPress={() => {
                clearError();
                clearNotice();
                setAuthMode(authMode === "sign-up" ? "sign-in" : "sign-up");
              }}
            >
              {authMode === "sign-up"
                ? "J’ai déjà un compte"
                : "Créer un compte"}
            </Button>
            {authMode === "sign-in" ? (
              <Button
                disabled={isSubmitting}
                onPress={() => {
                  clearError();
                  clearNotice();
                  setAuthMode("forgot-password");
                }}
              >
                Mot de passe oublié
              </Button>
            ) : null}
            {authMode === "forgot-password" || authMode === "reset-password" ? (
              <Button
                disabled={isSubmitting}
                onPress={() => {
                  clearError();
                  clearNotice();
                  setResetToken(undefined);
                  setAuthMode("sign-in");
                }}
              >
                Retour à la connexion
              </Button>
            ) : null}
          </View>
        </Surface>
      </View>
    </AuthPageShell>
  );
}

type AuthMode = "sign-in" | "sign-up" | "forgot-password" | "reset-password";

function firstParam(value: string | string[] | undefined) {
  return Array.isArray(value) ? value[0] : value;
}

function authTitle(mode: AuthMode) {
  if (mode === "sign-up") return "Créer un compte";
  if (mode === "forgot-password") return "Mot de passe oublié";
  if (mode === "reset-password") return "Nouveau mot de passe";
  return "Connexion";
}

function authDescription(mode: AuthMode) {
  if (mode === "forgot-password")
    return "Un lien de réinitialisation sera envoyé par e-mail.";
  if (mode === "reset-password")
    return "Choisissez un mot de passe d’au moins 10 caractères.";
  return "Accès réservé aux utilisateurs Fil Finance.";
}

function authSubmitLabel(mode: AuthMode) {
  if (mode === "sign-up") return "Créer un compte";
  if (mode === "forgot-password") return "Envoyer le lien";
  if (mode === "reset-password") return "Modifier le mot de passe";
  return "Se connecter";
}

function NoticeText({ children }: PropsWithChildren) {
  const tokens = useThemeTokens();
  return (
    <SubtleText style={{ color: tokens.colors.successText }}>
      {children}
    </SubtleText>
  );
}

function ErrorText({ children }: PropsWithChildren) {
  const tokens = useThemeTokens();
  return (
    <SubtleText style={{ color: tokens.colors.dangerText }}>
      {children}
    </SubtleText>
  );
}

function PasswordRequirement({
  validity,
}: {
  validity: ReturnType<typeof passwordValidity>;
}) {
  const tokens = useThemeTokens();
  const isInvalid = validity === "too-short" || validity === "too-long";
  const color =
    validity === "valid"
      ? tokens.colors.successText
      : isInvalid
        ? tokens.colors.dangerText
        : tokens.colors.textPrimary;

  return (
    <Text
      accessibilityLiveRegion="polite"
      className="max-w-full min-w-0 break-words text-sm leading-[19px]"
      style={{ color }}
    >
      {validity === "valid"
        ? "Mot de passe valide."
        : "Le mot de passe doit contenir entre 10 et 128 caractères."}
    </Text>
  );
}

type AuthInputProps = ComponentProps<typeof TextInput> & {
  tokens: ReturnType<typeof useThemeTokens>;
};

function AuthInput({ tokens, style, ...props }: AuthInputProps) {
  return (
    <TextInput
      {...props}
      placeholderTextColor={tokens.colors.textPrimary}
      style={[
        {
          backgroundColor: tokens.colors.background,
          borderColor: tokens.colors.border,
          borderRadius: 8,
          borderWidth: 1,
          color: tokens.colors.textPrimary,
          minHeight: 48,
          paddingHorizontal: 14,
        },
        style,
      ]}
    />
  );
}
