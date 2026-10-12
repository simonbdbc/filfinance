import { readFile, rm, writeFile } from "node:fs/promises";

const baseUrl = (process.env.FILFINANCE_AUTH_BASE_URL || "http://127.0.0.1:3000").replace(/\/$/, "");
const requestOrigin = (
  process.env.FILFINANCE_AUTH_SMOKE_ORIGIN ||
  process.env.BETTER_AUTH_URL ||
  baseUrl
).replace(/\/$/, "");
const expectedCallbackOrigin = (process.env.APP_WEB_URL || "").replace(/\/$/, "");
const emailFile = process.env.FILFINANCE_AUTH_SMOKE_EMAIL_FILE || "";
const email = process.env.FILFINANCE_AUTH_SMOKE_EMAIL || `smoke-${Date.now()}@example.test`;
const password = process.env.FILFINANCE_AUTH_SMOKE_PASSWORD || "Abcd1234!x";
const tokenFile = process.env.FILFINANCE_AUTH_SMOKE_TOKEN_FILE || "";
const expectedFromEmail = process.env.FILFINANCE_AUTH_SMOKE_FROM_EMAIL || "";
const expectedFromName = process.env.FILFINANCE_AUTH_SMOKE_FROM_NAME || "";

if (!emailFile) {
  throw new Error("FILFINANCE_AUTH_SMOKE_EMAIL_FILE is required for the email-verification smoke");
}
if (!expectedCallbackOrigin) {
  throw new Error("APP_WEB_URL is required for the email-verification smoke");
}
if (!expectedFromEmail || !expectedFromName) {
  throw new Error("FILFINANCE_AUTH_SMOKE_FROM_EMAIL and FILFINANCE_AUTH_SMOKE_FROM_NAME are required");
}

async function request(path, init = {}, expected = [200]) {
  const response = await fetch(`${baseUrl}${path}`, {
    ...init,
    headers: {
      origin: requestOrigin,
      ...(init.body ? { "content-type": "application/json" } : {}),
      ...init.headers
    }
  });
  const text = await response.text();
  if (!expected.includes(response.status)) {
    throw new Error(`${init.method || "GET"} ${path}: expected ${expected.join("/")}, got ${response.status}: ${text}`);
  }
  return { response, body: text ? JSON.parse(text) : null };
}

function sessionCookie(response) {
  return response.headers
    .getSetCookie()
    .filter((value) => /(?:^|\.)(?:session_token|session_data)=/.test(value.split(";", 1)[0]))
    .map((value) => value.split(";", 1)[0])
    .join("; ");
}

function decodeQuotedPrintable(value) {
  return value
    .replace(/=\r?\n/g, "")
    .replace(/=([0-9a-f]{2})/gi, (_match, hex) => String.fromCharCode(Number.parseInt(hex, 16)));
}

function decodeMimeBody(headers, body) {
  const encoding = headers.match(/^content-transfer-encoding:\s*([^\s;]+)/im)?.[1]?.toLowerCase();
  if (encoding === "quoted-printable") return decodeQuotedPrintable(body);
  if (encoding === "base64") return Buffer.from(body.replace(/\s/g, ""), "base64").toString("utf8");
  return body;
}

function textPlainPart(message) {
  const normalized = message.replaceAll("\r\n", "\n");
  const headerEnd = normalized.indexOf("\n\n");
  if (headerEnd < 0) throw new Error("SMTP capture is missing MIME headers");
  const messageHeaders = normalized.slice(0, headerEnd);
  const messageBody = normalized.slice(headerEnd + 2);
  const boundary = messageHeaders.match(/boundary=(?:"([^"]+)"|([^;\s]+))/i)?.slice(1).find(Boolean);

  if (!boundary) {
    if (!/^content-type:\s*text\/plain/im.test(messageHeaders)) {
      throw new Error("SMTP capture does not contain a text/plain part");
    }
    return decodeMimeBody(messageHeaders, messageBody);
  }

  for (const rawPart of messageBody.split(`--${boundary}`)) {
    const part = rawPart.replace(/^\n/, "");
    const partHeaderEnd = part.indexOf("\n\n");
    if (partHeaderEnd < 0) continue;
    const partHeaders = part.slice(0, partHeaderEnd);
    if (!/^content-type:\s*text\/plain(?:;|\s|$)/im.test(partHeaders)) continue;
    return decodeMimeBody(partHeaders, part.slice(partHeaderEnd + 2));
  }
  throw new Error("SMTP capture does not contain a text/plain part");
}

function headerValue(message, name) {
  const normalized = message.replaceAll("\r\n", "\n");
  const headerEnd = normalized.indexOf("\n\n");
  if (headerEnd < 0) throw new Error("SMTP capture is missing MIME headers");
  const headers = normalized.slice(0, headerEnd);
  const match = headers.match(new RegExp(`^${name}:\\s*(.+(?:\\n[ \\t].+)*)$`, "im"));
  if (!match) throw new Error(`SMTP capture is missing the ${name} header`);
  return match[1].replace(/\n[ \t]+/g, " ");
}

async function capturedMessage() {
  const message = await readFile(emailFile, "utf8");
  const from = headerValue(message, "From");
  if (!from.includes(expectedFromEmail) || !from.includes(expectedFromName)) {
    throw new Error("SMTP capture sender does not match the configured SMTP_FROM_EMAIL and SMTP_FROM_NAME");
  }
  return message;
}

async function capturedVerificationUrl() {
  const message = textPlainPart(await capturedMessage());
  const matches = [
    ...new Set(message.match(/https?:\/\/[^\s<>"']+\/api\/auth\/verify-email\?[^\s<>"']+/g) || [])
  ];
  if (matches.length !== 1) {
    throw new Error("SMTP capture must contain exactly one verification link");
  }

  let verificationUrl;
  try {
    verificationUrl = new URL(matches[0]);
  } catch {
    throw new Error("SMTP capture contains an invalid verification link");
  }
  if (verificationUrl.origin !== new URL(process.env.BETTER_AUTH_URL || baseUrl).origin
    || verificationUrl.pathname !== "/api/auth/verify-email"
    || !verificationUrl.searchParams.get("token")) {
    throw new Error("SMTP capture verification link targets an unexpected endpoint");
  }
  return verificationUrl;
}

async function capturedPasswordResetUrl() {
  const message = textPlainPart(await capturedMessage());
  const matches = [
    ...new Set(message.match(/https?:\/\/[^\s<>"']+\/api\/auth\/reset-password\/[^\s<>"'?]+\?[^\s<>"']+/g) || [])
  ];
  if (matches.length !== 1) {
    throw new Error("SMTP capture must contain exactly one password-reset link");
  }

  let resetUrl;
  try {
    resetUrl = new URL(matches[0]);
  } catch {
    throw new Error("SMTP capture contains an invalid password-reset link");
  }
  if (resetUrl.origin !== new URL(process.env.BETTER_AUTH_URL || baseUrl).origin
    || !resetUrl.pathname.startsWith("/api/auth/reset-password/")
    || resetUrl.pathname === "/api/auth/reset-password/"
    || !resetUrl.searchParams.get("callbackURL")) {
    throw new Error("SMTP capture password-reset link targets an unexpected endpoint");
  }
  return resetUrl;
}

const escalatedSignup = await request("/api/auth/sign-up/email", {
  method: "POST",
  body: JSON.stringify({ email, password, name: "FilFinance Smoke", role: "admin" })
}, [400]);
if (escalatedSignup.body?.code !== "FIELD_NOT_ALLOWED") {
  throw new Error("sign-up did not reject the caller-controlled role");
}

const shortPasswordSignup = await request("/api/auth/sign-up/email", {
  method: "POST",
  body: JSON.stringify({ email, password: "123456789", name: "FilFinance Smoke" })
}, [400, 422]);
if (shortPasswordSignup.body?.code !== "PASSWORD_TOO_SHORT") {
  throw new Error("sign-up did not reject a nine-character password with PASSWORD_TOO_SHORT");
}

const signup = await request("/api/auth/sign-up/email", {
  method: "POST",
  body: JSON.stringify({ email, password, name: "FilFinance Smoke" })
});
if (signup.body?.token !== null || signup.body?.user?.emailVerified !== false) {
  throw new Error("sign-up must remain unverified and sessionless until email activation");
}
if (sessionCookie(signup.response)) {
  throw new Error("sign-up returned a session cookie before email activation");
}

const sendVerification = await request("/api/auth/send-verification-email", {
  method: "POST",
  body: JSON.stringify({
    email,
    callbackURL: `${expectedCallbackOrigin}/email-verification?status=success`
  })
});
if (sendVerification.body?.status !== true) {
  throw new Error("explicit verification-email delivery did not report success");
}

const sessionBeforeActivation = await request("/api/auth/get-session");
if (sessionBeforeActivation.body !== null) {
  throw new Error("get-session returned a session before email activation");
}

const signInBeforeActivation = await request("/api/auth/sign-in/email", {
  method: "POST",
  body: JSON.stringify({ email, password })
}, [403]);
if (signInBeforeActivation.body?.code !== "EMAIL_NOT_VERIFIED") {
  throw new Error("sign-in before activation did not return EMAIL_NOT_VERIFIED");
}

const verificationUrl = await capturedVerificationUrl();
const verificationResponse = await fetch(verificationUrl, { redirect: "manual" });
if (verificationResponse.status !== 302) {
  throw new Error(`email verification expected HTTP 302, got ${verificationResponse.status}`);
}
const expectedLocation = `${expectedCallbackOrigin}/email-verification?status=success`;
if (verificationResponse.headers.get("location") !== expectedLocation) {
  throw new Error("email verification redirected to an unexpected callback");
}
const cookie = sessionCookie(verificationResponse);
if (!cookie) {
  throw new Error("email activation did not return a session cookie");
}

const session = await request("/api/auth/get-session", { headers: { cookie } });
if (session.body?.user?.email !== email || session.body?.user?.emailVerified !== true) {
  throw new Error("restored session does not match the activated user");
}

const tokenResponse = await request("/api/auth/token", { headers: { cookie } });
const accessToken = tokenResponse.body?.token;
if (typeof accessToken !== "string" || accessToken.split(".").length !== 3) {
  throw new Error("Better Auth did not return a signed JWT after activation");
}

const restoredSession = await request("/api/auth/get-session", { headers: { cookie } });
if (restoredSession.body?.user?.email !== email) {
  throw new Error("session was not restored on a subsequent request");
}

const jwks = await request("/api/auth/jwks");
if (!Array.isArray(jwks.body?.keys) || jwks.body.keys.length === 0) {
  throw new Error("JWKS endpoint did not return a public key");
}

const authHeaders = { authorization: `Bearer ${accessToken}` };
const profile = await request("/api/me", { headers: authHeaders });
if (profile.body?.email !== email || profile.body?.role !== "listener") {
  throw new Error("profile identity or default role does not match the JWT");
}

const updatedProfile = await request("/api/me", {
  method: "PATCH",
  headers: authHeaders,
  body: JSON.stringify({ preferred_tts_voice: "fr_FR-upmc-medium-jessica", theme: "dark" })
});
if (updatedProfile.body?.preferences?.theme !== "dark") {
  throw new Error("profile preferences were not persisted");
}

await request("/api/me", {
  method: "PATCH",
  headers: authHeaders,
  body: JSON.stringify({ role: "admin" })
}, [400]);

if (tokenFile) {
  await writeFile(tokenFile, accessToken, { encoding: "utf8", mode: 0o600 });
}

await rm(emailFile, { force: true });
const resetRequest = await request("/api/auth/request-password-reset", {
  method: "POST",
  body: JSON.stringify({
    email,
    redirectTo: `${expectedCallbackOrigin}/login?mode=reset-password`
  })
});
if (resetRequest.body?.status !== true) {
  throw new Error("password-reset request did not return its anti-enumeration success response");
}

const resetUrl = await capturedPasswordResetUrl();
const resetCallbackResponse = await fetch(resetUrl, { redirect: "manual" });
if (resetCallbackResponse.status !== 302) {
  throw new Error(`password-reset callback expected HTTP 302, got ${resetCallbackResponse.status}`);
}
const resetLocation = new URL(resetCallbackResponse.headers.get("location") || "", expectedCallbackOrigin);
const resetToken = resetLocation.searchParams.get("token");
if (resetLocation.origin !== expectedCallbackOrigin
  || resetLocation.pathname !== "/login"
  || resetLocation.searchParams.get("mode") !== "reset-password"
  || !resetToken) {
  throw new Error("password-reset callback did not target the safe PWA route with a token");
}

const newPassword = "NewAbcd1234!";
await request("/api/auth/reset-password", {
  method: "POST",
  body: JSON.stringify({ newPassword, token: resetToken })
});
const reusedReset = await request("/api/auth/reset-password", {
  method: "POST",
  body: JSON.stringify({ newPassword: "OtherAbcd1234!", token: resetToken })
}, [400]);
if (reusedReset.body?.code !== "INVALID_TOKEN") {
  throw new Error("password-reset token was not single-use");
}

const sessionAfterReset = await request("/api/auth/get-session", { headers: { cookie } });
if (sessionAfterReset.body !== null) {
  throw new Error("password reset did not revoke the pre-existing session");
}
await request("/api/auth/sign-in/email", {
  method: "POST",
  body: JSON.stringify({ email, password })
}, [400, 401]);
const signInAfterReset = await request("/api/auth/sign-in/email", {
  method: "POST",
  body: JSON.stringify({ email, password: newPassword })
});
const renewedCookie = sessionCookie(signInAfterReset.response);
if (!renewedCookie) {
  throw new Error("new password did not create a fresh verified session");
}

await request("/api/auth/sign-out", {
  method: "POST",
  headers: { cookie: renewedCookie },
  body: JSON.stringify({})
});
console.log(`Better Auth email-verification integration smoke passed for ${email}`);
