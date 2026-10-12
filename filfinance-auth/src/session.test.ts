import assert from "node:assert/strict";
import test from "node:test";

process.env.BETTER_AUTH_SECRET = "session-test-secret-with-at-least-thirty-two-characters";

const { authenticatedUserFromJwtPayload } = await import("./session.js");

test("authenticatedUserFromJwtPayload accepts an email-verified identity", () => {
  assert.deepEqual(
    authenticatedUserFromJwtPayload({
      sub: "user-1",
      email: "verified@example.test",
      email_verified: true,
      name: "Verified User",
      role: "moderator"
    }),
    {
      id: "user-1",
      email: "verified@example.test",
      name: "Verified User",
      role: "moderator"
    }
  );
});

test("authenticatedUserFromJwtPayload rejects unverified or malformed verification claims", () => {
  const baseClaims = {
    sub: "user-1",
    email: "unverified@example.test"
  };

  for (const emailVerified of [undefined, false, "true", 1, null]) {
    assert.equal(
      authenticatedUserFromJwtPayload({
        ...baseClaims,
        email_verified: emailVerified
      }),
      null
    );
  }
});

test("authenticatedUserFromJwtPayload preserves optional claim defaults", () => {
  assert.deepEqual(
    authenticatedUserFromJwtPayload({
      sub: "user-2",
      email: "listener@example.test",
      email_verified: true
    }),
    {
      id: "user-2",
      email: "listener@example.test",
      name: null,
      role: "listener"
    }
  );
});
