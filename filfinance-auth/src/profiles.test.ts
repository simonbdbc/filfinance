import assert from "node:assert/strict";
import test from "node:test";
import { parsePreferencePatch } from "./preferences.js";

function patchRequest(body: string): Request {
  return new Request("http://localhost/api/me", {
    method: "PATCH",
    headers: { "content-type": "application/json" },
    body
  });
}

test("parsePreferencePatch normalizes supported preferences", async () => {
  const result = await parsePreferencePatch(
    patchRequest(JSON.stringify({ preferred_tts_voice: " voice-one ", theme: "dark" }))
  );

  assert(!(result instanceof Response));
  assert.deepEqual(result, { preferred_tts_voice: "voice-one", theme: "dark" });
});

test("parsePreferencePatch rejects unknown fields", async () => {
  const result = await parsePreferencePatch(patchRequest(JSON.stringify({ role: "admin" })));

  assert(result instanceof Response);
  assert.equal(result.status, 400);
  assert.deepEqual(await result.json(), { error: "unknown_preference" });
});

test("parsePreferencePatch rejects invalid JSON and invalid themes", async () => {
  const invalidJson = await parsePreferencePatch(patchRequest("{"));
  const invalidTheme = await parsePreferencePatch(
    patchRequest(JSON.stringify({ theme: "system" }))
  );

  assert(invalidJson instanceof Response);
  assert.equal(invalidJson.status, 400);
  assert(invalidTheme instanceof Response);
  assert.equal(invalidTheme.status, 400);
});
