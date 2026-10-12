export function emailVerificationCallbackUrl(appWebUrl: string): string {
  const callbackUrl = new URL("/email-verification", appWebUrl);
  callbackUrl.searchParams.set("status", "success");
  return callbackUrl.toString();
}

export function passwordResetCallbackUrl(appWebUrl: string): string {
  const callbackUrl = new URL("/login", appWebUrl);
  callbackUrl.searchParams.set("mode", "reset-password");
  return callbackUrl.toString();
}

export function withSafeAuthCallbacks(
  request: Request,
  appWebUrl: string,
): Request {
  const url = new URL(request.url);
  if (request.method !== "GET") {
    return request;
  }

  if (url.pathname === "/api/auth/verify-email") {
    url.searchParams.set("callbackURL", emailVerificationCallbackUrl(appWebUrl));
  } else if (/^\/api\/auth\/reset-password\/[^/]+\/?$/u.test(url.pathname)) {
    url.searchParams.set("callbackURL", passwordResetCallbackUrl(appWebUrl));
  } else {
    return request;
  }

  return new Request(url, request);
}
