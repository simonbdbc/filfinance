export type ResetCallbackState = {
  authMode: "sign-in" | "reset-password";
  resetLinkError: string | null;
  resetToken: string | undefined;
};

export function readResetCallback(
  token: string | undefined,
  errorCode: string | undefined
): ResetCallbackState {
  if (errorCode) {
    return {
      authMode: "sign-in",
      resetLinkError: resetLinkErrorMessage(errorCode),
      resetToken: undefined,
    };
  }

  if (token) {
    return {
      authMode: "reset-password",
      resetLinkError: null,
      resetToken: token,
    };
  }

  return {
    authMode: "sign-in",
    resetLinkError: null,
    resetToken: undefined,
  };
}

export function clearResetCallbackFromBrowserUrl() {
  if (typeof window === "undefined" || !window.location || !window.history) return;

  try {
    const url = new URL(window.location.href);
    const hasResetCallback = url.searchParams.has("token") || url.searchParams.has("error");
    if (!hasResetCallback) return;

    url.searchParams.delete("error");
    url.searchParams.delete("mode");
    url.searchParams.delete("token");
    window.history.replaceState(window.history.state, "", `${url.pathname}${url.search}${url.hash}`);
  } catch {
    // URL redaction is best effort. The captured callback state remains usable.
  }
}

export function resetLinkErrorMessage(errorCode: string | null) {
  if (!errorCode) return null;

  switch (errorCode.trim().toUpperCase()) {
    case "INVALID_TOKEN":
    case "TOKEN_EXPIRED":
      return "Ce lien de réinitialisation est invalide ou expiré. Demandez un nouveau lien.";
    default:
      return "Ce lien de réinitialisation ne peut pas être utilisé. Demandez un nouveau lien.";
  }
}
