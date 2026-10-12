import assert from "node:assert/strict";
import test from "node:test";

import { readJwtMetadata } from "../src/features/auth/lib/jwtMetadata.ts";

function jwtWithPayload(payload) {
  const encoded = Buffer.from(JSON.stringify(payload), "utf8").toString("base64url");
  return `header.${encoded}.signature`;
}

test("JWT metadata decoding does not depend on the browser atob global", () => {
  const originalAtob = globalThis.atob;
  globalThis.atob = undefined;
  try {
    assert.deepEqual(
      readJwtMetadata(jwtWithPayload({ sub: "user-native", exp: 1_800_000_000, label: "verifie" })),
      { subject: "user-native", expiresAt: 1_800_000_000_000 },
    );
  } finally {
    globalThis.atob = originalAtob;
  }
});

test("JWT metadata rejects malformed payloads and missing subjects", () => {
  assert.equal(readJwtMetadata("not-a-jwt"), null);
  assert.equal(readJwtMetadata(jwtWithPayload({ exp: 1_800_000_000 })), null);
  assert.equal(readJwtMetadata("header.!!!!.signature"), null);
});
