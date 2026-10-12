import assert from "node:assert/strict";
import test from "node:test";

process.env.BETTER_AUTH_SECRET = "auth-config-test-secret-with-at-least-thirty-two-characters";

const { auth } = await import("./auth.js");

test("sign-up never backgrounds verification-email delivery", () => {
  assert.equal(auth.options.emailVerification?.sendOnSignUp, false);
});
