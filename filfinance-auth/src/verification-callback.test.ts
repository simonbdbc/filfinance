import assert from "node:assert/strict";
import test from "node:test";

import {
  emailVerificationCallbackUrl,
  passwordResetCallbackUrl,
  withSafeAuthCallbacks,
} from "./verification-callback.js";

const appWebUrl = "https://app.filfinance.fr";

test("builds the public email verification callback", () => {
  assert.equal(
    emailVerificationCallbackUrl(appWebUrl),
    "https://app.filfinance.fr/email-verification?status=success",
  );
  assert.equal(
    passwordResetCallbackUrl(appWebUrl),
    "https://app.filfinance.fr/login?mode=reset-password",
  );
});

test("adds a callback to legacy verification links without losing the token", () => {
  const request = new Request(
    "https://api.filfinance.fr/api/auth/verify-email?token=legacy-token",
  );
  const normalized = withSafeAuthCallbacks(request, appWebUrl);
  const normalizedUrl = new URL(normalized.url);

  assert.equal(normalizedUrl.searchParams.get("token"), "legacy-token");
  assert.equal(
    normalizedUrl.searchParams.get("callbackURL"),
    "https://app.filfinance.fr/email-verification?status=success",
  );
});

test("replaces every verification callback with the configured HTTPS PWA", () => {
  const rootCallbackRequest = new Request(
    "https://api.filfinance.fr/api/auth/verify-email?token=token&callbackURL=%2F",
  );
  const explicitCallbackRequest = new Request(
    "https://api.filfinance.fr/api/auth/verify-email?token=token&callbackURL=filfinance%3A%2F%2Femail-verification%3Fstatus%3Dsuccess",
  );

  const normalizedRoot = withSafeAuthCallbacks(rootCallbackRequest, appWebUrl);
  const normalizedExplicit = withSafeAuthCallbacks(explicitCallbackRequest, appWebUrl);

  assert.equal(
    new URL(normalizedRoot.url).searchParams.get("callbackURL"),
    "https://app.filfinance.fr/email-verification?status=success",
  );
  assert.equal(
    new URL(normalizedExplicit.url).searchParams.get("callbackURL"),
    "https://app.filfinance.fr/email-verification?status=success",
  );
  assert.equal(new URL(normalizedExplicit.url).searchParams.get("token"), "token");
});

test("replaces custom-scheme reset callbacks without losing the path token", () => {
  const request = new Request(
    "https://api.filfinance.fr/api/auth/reset-password/reset-secret?callbackURL=filfinance%3A%2F%2Flogin%3Fmode%3Dreset-password",
  );
  const normalized = withSafeAuthCallbacks(request, appWebUrl);
  const normalizedUrl = new URL(normalized.url);

  assert.equal(normalizedUrl.pathname, "/api/auth/reset-password/reset-secret");
  assert.equal(
    normalizedUrl.searchParams.get("callbackURL"),
    "https://app.filfinance.fr/login?mode=reset-password",
  );
});

test("does not alter unrelated auth requests", () => {
  const request = new Request("https://api.filfinance.fr/api/auth/get-session");

  assert.equal(withSafeAuthCallbacks(request, appWebUrl), request);
});
