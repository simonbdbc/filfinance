import { useEffect, useRef, useState } from "react";
import { useLocalSearchParams, useRouter } from "expo-router";
import { View } from "react-native";

import { AuthBrandHeader, AuthPageShell } from "./AuthPageShell";
import { useAuth } from "../providers/AuthProvider";
import { consumeExpoAuthRedirectCookie } from "@shared/api/betterAuthClient";
import { Button } from "@shared/ui/Button";
import { Surface, SectionHeader } from "@shared/ui/Surface";

type VerificationParams = {
  status?: string | string[];
  error?: string | string[];
  cookie?: string | string[];
};

export function EmailVerificationScreen() {
  const params = useLocalSearchParams<VerificationParams>();
  const router = useRouter();
  const { authSession, isConfigured, isLoading, refreshSession } = useAuth();
  const status = firstParam(params.status);
  const errorCode = firstParam(params.error);
  const redirectCookie = firstParam(params.cookie);
  const callbackSucceeded = status === "success" && !errorCode;
  const handledCallbacksRef = useRef(new Set<string>());
  const callbackGenerationRef = useRef(0);
  const [isRestoring, setIsRestoring] = useState(callbackSucceeded);
  const [sessionRestored, setSessionRestored] = useState(Boolean(authSession));

  useEffect(() => {
    const callbackKey = JSON.stringify([
      status ?? "",
      errorCode ?? "",
      redirectCookie ?? "",
    ]);
    if (handledCallbacksRef.current.has(callbackKey)) return;
    handledCallbacksRef.current.add(callbackKey);
    if (redirectCookie) {
      handledCallbacksRef.current.add(
        JSON.stringify([status ?? "", errorCode ?? "", ""]),
      );
    }
    const callbackGeneration = callbackGenerationRef.current + 1;
    callbackGenerationRef.current = callbackGeneration;
    const isCurrentCallback = () =>
      callbackGenerationRef.current === callbackGeneration;
    setIsRestoring(callbackSucceeded);
    setSessionRestored(Boolean(authSession));

    void (async () => {
      let cookieImported = true;
      try {
        if (redirectCookie) {
          router.replace({
            pathname: "/email-verification",
            params: {
              ...(status ? { status } : {}),
              ...(errorCode ? { error: errorCode } : {}),
            },
          });

          if (callbackSucceeded) {
            try {
              cookieImported = await consumeExpoAuthRedirectCookie(redirectCookie);
            } catch {
              cookieImported = false;
              // The cookie has already been removed from the visible URL. The
              // session refresh below will fall back to the sign-in state.
            }
          }
        }

        if (
          callbackSucceeded &&
          isConfigured &&
          cookieImported && isCurrentCallback()
        ) {
          const token = await refreshSession();
          if (isCurrentCallback()) {
            setSessionRestored(Boolean(token));
          }
        }
      } catch {
        if (isCurrentCallback()) {
          setSessionRestored(false);
        }
      } finally {
        if (isCurrentCallback()) {
          setIsRestoring(false);
        }
      }
    })();
  }, [
    authSession,
    callbackSucceeded,
    errorCode,
    isConfigured,
    redirectCookie,
    refreshSession,
    router,
    status,
  ]);

  const hasSession = Boolean(authSession) || sessionRestored;
  const isPending = callbackSucceeded && (isLoading || isRestoring);
  const content = verificationContent({
    callbackSucceeded,
    errorCode,
    hasSession,
    isConfigured,
    isPending,
  });

  return (
    <AuthPageShell>
      <View className="w-full max-w-full min-w-0 gap-3">
        <AuthBrandHeader />
        <Surface variant={content.isError ? "danger" : "default"}>
          <SectionHeader
            title={content.title}
            description={content.description}
          />
          {!isPending ? (
            <Button
              onPress={() => router.replace(hasSession ? "/feed" : "/login")}
              variant="primary"
            >
              {hasSession ? "Accéder à Fil Finance" : "Se connecter"}
            </Button>
          ) : null}
        </Surface>
      </View>
    </AuthPageShell>
  );
}

type VerificationContentInput = {
  callbackSucceeded: boolean;
  errorCode?: string;
  hasSession: boolean;
  isConfigured: boolean;
  isPending: boolean;
};

function verificationContent({
  callbackSucceeded,
  errorCode,
  hasSession,
  isConfigured,
  isPending,
}: VerificationContentInput) {
  if (!isConfigured) {
    return {
      title: "Configuration Better Auth absente",
      description: "La confirmation ne peut pas restaurer la session.",
      isError: true,
    };
  }

  if (errorCode) {
    return {
      title: "Activation impossible",
      description: verificationErrorMessage(errorCode),
      isError: true,
    };
  }

  if (!callbackSucceeded) {
    return {
      title: "Lien de confirmation invalide",
      description: "Utilisez le lien reçu par e-mail pour activer votre compte.",
      isError: true,
    };
  }

  if (isPending) {
    return {
      title: "Activation du compte",
      description:
        "Confirmation de l’adresse e-mail et ouverture de la session.",
      isError: false,
    };
  }

  return {
    title: "Adresse e-mail confirmée",
    description: hasSession
      ? "Votre compte est activé et vous êtes connecté."
      : "Votre compte est activé. Reconnectez-vous pour continuer.",
    isError: false,
  };
}

function verificationErrorMessage(code: string): string {
  if (code.toUpperCase() === "TOKEN_EXPIRED") {
    return "Ce lien de confirmation a expiré. Connectez-vous pour recevoir un nouveau lien.";
  }
  if (code.toUpperCase() === "USER_NOT_FOUND") {
    return "Aucun compte ne correspond à ce lien de confirmation.";
  }
  if (code.toUpperCase() === "INVALID_USER") {
    return "Ce lien ne correspond pas au compte actuellement utilisé.";
  }
  return "Ce lien de confirmation est invalide ou a déjà été utilisé.";
}

function firstParam(value: string | string[] | undefined) {
  return Array.isArray(value) ? value[0] : value;
}
