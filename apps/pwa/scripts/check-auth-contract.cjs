const fs = require("node:fs");
const path = require("node:path");

const root = path.resolve(__dirname, "..");
const read = (relativePath) => fs.readFileSync(path.join(root, relativePath), "utf8");
const fail = (message) => {
  console.error(`[auth-contract] ${message}`);
  process.exit(1);
};

const policy = read("src/features/auth/lib/passwordPolicy.ts");
const provider = read("src/features/auth/providers/AuthProvider.tsx");
const verification = read("src/features/auth/components/EmailVerificationScreen.tsx");
const redirects = read("src/features/auth/lib/authRedirects.ts");
const jwtMetadata = read("src/features/auth/lib/jwtMetadata.ts");
const protectedLayout = read("src/application/layouts/ProtectedAppLayout.tsx");
const authGate = read("src/features/auth/components/AuthGate.tsx");

if (!policy.includes("PASSWORD_MIN_LENGTH = 10")) {
  fail("the client password minimum must remain aligned with Better Auth at 10 characters");
}
if (!provider.includes("Le mot de passe doit contenir au moins 10 caractères.")) {
  fail("the client must expose a clear 10-character validation error");
}
if (!redirects.includes("/email-verification?status=success")
  || redirects.includes("filfinance://")) {
  fail("sensitive email callbacks must target an HTTP PWA route, never a claimable custom scheme");
}
if (!provider.includes("EXPO_PUBLIC_FILFINANCE_PWA_URL")
  || !provider.includes("URL HTTPS de la PWA manquante")) {
  fail("native callbacks must fail closed without an explicit HTTPS PWA origin");
}
const signUpStart = provider.indexOf("const signUp = useCallback");
const passwordResetStart = provider.indexOf("const requestPasswordReset", signUpStart);
if (signUpStart < 0 || passwordResetStart < 0
  || provider.slice(signUpStart, passwordResetStart).includes("refreshSession()")) {
  fail("sign-up must wait for email activation before bootstrapping a JWT");
}
const signUpFlow = provider.slice(signUpStart, passwordResetStart);
if (!signUpFlow.includes("betterAuthClient.sendVerificationEmail")
  || !signUpFlow.includes("verificationResult.error")) {
  fail("sign-up must explicitly await verification-email delivery and surface SMTP failure");
}
if (!provider.includes("MAX_ACCESS_TOKEN_ATTEMPTS = 3")
  || !provider.includes('accessTokenStatus === "idle"')
  || !provider.includes('accessTokenStatus === "loading"')) {
  fail("an existing cookie session must stay pending during bounded JWT bootstrap retries");
}
const transport = read("src/shared/api/authSession.ts");
if (!transport.includes("this.assertActive();")
  || !transport.includes("grant.subject !== this.userId")
  || !transport.includes("this.lifetime.abort()")
  || !provider.includes("clientRef.current !== client")
  || !provider.includes("authSession?.userId === effectiveUser.id")
  || !provider.includes("isSigningOutRef.current")) {
  fail("closed sessions and cross-user token grants must never restore authentication");
}
if (!protectedLayout.includes('authStatus === "loading"')
  || !protectedLayout.includes('authStatus !== "authenticated"')
  || /accessToken/.test(protectedLayout)) {
  fail("the connected runtime must depend on session status, never token rotation");
}
if (provider.includes("globalThis.atob") || jwtMetadata.includes("globalThis.atob")) {
  fail("native JWT bootstrap must not depend on a browser-only atob global");
}
if (!protectedLayout.includes("if (user)")
  || !protectedLayout.includes("<AuthTokenRecoveryScreen />")) {
  fail("a cookie session without a JWT must expose recovery instead of redirecting to login");
}

const replaceIndex = verification.indexOf("router.replace({");
const consumeIndex = verification.indexOf("await consumeExpoAuthRedirectCookie(redirectCookie)");
if (replaceIndex < 0 || consumeIndex < 0 || replaceIndex > consumeIndex) {
  fail("the redirect cookie must be removed from the visible URL before SecureStore consumption");
}
if (!verification.includes("} catch {") || !verification.includes("setSessionRestored(false);")) {
  fail("verification session restoration failures must be handled without an unhandled rejection");
}
if (!verification.includes("handledCallbacksRef") || !verification.includes("new Set<string>()")) {
  fail("successive deep-link callbacks on one mounted route must each be handled exactly once");
}
if (!verification.includes("callbackGenerationRef")
  || !verification.includes("isCurrentCallback()")) {
  fail("an older deep-link callback must not overwrite the state of a newer callback");
}
if (!verification.includes("cookieImported = await consumeExpoAuthRedirectCookie")
  || !verification.includes("cookieImported && isCurrentCallback()")) {
  fail("a failed native cookie import must not start a JWT bootstrap without a session");
}
if (authGate.includes("useRouter") || /router\.(?:setParams|replace|push)\(/.test(authGate)) {
  fail("reset callbacks must not navigate before the Expo router has mounted");
}
if (!authGate.includes("readResetCallback(callbackToken, callbackError)")
  || !authGate.includes("setResetToken(callbackToken)")
  || !authGate.includes("clearResetCallbackFromBrowserUrl()")
  || !authGate.includes("setResetToken(undefined)")) {
  fail("reset callback secrets must be captured only in component memory and cleared after use");
}
if (!authGate.includes("readResetCallback(undefined, callbackError).resetLinkError")) {
  fail("reset callback errors must be explained to the user");
}

console.log("[auth-contract] password, callback, JWT bootstrap and deep-link contracts OK");

const businessRoot = path.join(root, "src/features");
for (const entry of fs.readdirSync(businessRoot, {recursive:true, withFileTypes:true})) {
  if (!entry.isFile() || !/\.tsx?$/.test(entry.name)) continue;
  const file = path.join(entry.parentPath, entry.name);
  if (file.includes(`${path.sep}auth${path.sep}`)) continue;
  if (/\baccessToken\b/.test(fs.readFileSync(file,"utf8"))) {
    fail("business features must use stable authSession, not raw accessToken: " + path.relative(root,file));
  }
}
