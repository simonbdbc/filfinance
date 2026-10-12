import assert from "node:assert/strict";
import test from "node:test";

import {
  emailVerificationReturnUrl,
  passwordResetRedirectUrl,
} from "../src/features/auth/lib/authRedirects.ts";

test("web callbacks preserve the deployed PWA origin", () => {
  assert.equal(
    emailVerificationReturnUrl("web", "https://app.dev.filfinance.fr", ""),
    "https://app.dev.filfinance.fr/email-verification?status=success",
  );
  assert.equal(
    passwordResetRedirectUrl("web", "https://app.filfinance.fr", ""),
    "https://app.filfinance.fr/login?mode=reset-password",
  );
});

test("native callbacks use the configured HTTPS PWA and never a custom scheme", () => {
  for (const platform of ["ios", "android"]) {
    assert.equal(
      emailVerificationReturnUrl(
        platform,
        "http://192.168.1.20:8081",
        "https://app.dev.filfinance.fr",
      ),
      "https://app.dev.filfinance.fr/email-verification?status=success",
    );
    assert.equal(
      passwordResetRedirectUrl(
        platform,
        "http://192.168.1.20:8081",
        "https://app.dev.filfinance.fr",
      ),
      "https://app.dev.filfinance.fr/login?mode=reset-password",
    );
  }
});

test("callbacks fail closed when no safe HTTP origin exists", () => {
  assert.equal(
    emailVerificationReturnUrl("web", null, ""),
    null,
  );
  assert.equal(
    passwordResetRedirectUrl("ios", "not a URL", "filfinance://"),
    null,
  );
  assert.equal(
    emailVerificationReturnUrl("android", null, "http://app.dev.filfinance.fr"),
    null,
  );
});

test("local web development may use only a loopback HTTP origin", () => {
  assert.equal(
    emailVerificationReturnUrl("web", "http://localhost:8052", ""),
    "http://localhost:8052/email-verification?status=success",
  );
  assert.equal(
    emailVerificationReturnUrl("web", "http://192.168.1.20:8081", ""),
    null,
  );
});
