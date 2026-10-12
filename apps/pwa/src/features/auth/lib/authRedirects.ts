export function emailVerificationReturnUrl(
  platform: string,
  webOrigin: string | null,
  configuredPwaOrigin: string,
): string | null {
  const origin = safeCallbackOrigin(platform, webOrigin, configuredPwaOrigin);
  return origin ? `${origin}/email-verification?status=success` : null;
}

export function passwordResetRedirectUrl(
  platform: string,
  webOrigin: string | null,
  configuredPwaOrigin: string,
): string | null {
  const origin = safeCallbackOrigin(platform, webOrigin, configuredPwaOrigin);
  return origin ? `${origin}/login?mode=reset-password` : null;
}

function safeCallbackOrigin(
  platform: string,
  webOrigin: string | null,
  configuredPwaOrigin: string,
): string | null {
  if (platform === "web") {
    const currentOrigin = httpOrigin(webOrigin, true);
    if (currentOrigin) {
      return currentOrigin;
    }
  }

  return httpOrigin(configuredPwaOrigin, false);
}

function httpOrigin(value: string | null, allowLocalHttp: boolean): string | null {
  if (!value) return null;

  try {
    const url = new URL(value);
    if (url.origin === "null") return null;
    if (url.protocol === "https:") return url.origin;
    if (allowLocalHttp && url.protocol === "http:" && isLocalHostname(url.hostname)) {
      return url.origin;
    }
    return null;
  } catch {
    return null;
  }
}

function isLocalHostname(hostname: string): boolean {
  return hostname === "localhost"
    || hostname === "127.0.0.1"
    || hostname === "[::1]";
}
