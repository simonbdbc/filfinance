import assert from "node:assert/strict";
import test from "node:test";
import { legalLinksForOrigin, parseLegalOrigin } from "../src/shared/legal/legalOrigin.ts";
import { readPublicEnv } from "../src/shared/config/runtimeEnv.ts";

for (const origin of ["http://localhost:4321", "https://dev.filfinance.fr", "https://filfinance.fr"]) {
  test(`quatre documents publics sur ${origin}`, () => {
    const links = legalLinksForOrigin(origin + "/");
    assert.equal(links.length, 4);
    assert.deepEqual(links.map(link => new URL(link.href).pathname), ["/mentions-legales/", "/politique-de-confidentialite/", "/conditions-generales-utilisation/", "/cookies/"]);
    assert(links.every(link => new URL(link.href).origin === origin && !new URL(link.href).search));
  });
}
for (const value of [undefined, "", "http://filfinance.fr", "javascript:alert(1)", "https://filfinance.fr/login", "https://user:secret@filfinance.fr", "https://filfinance.fr/?token=secret", "https://filfinance.fr/#section"]) {
  test(`origine refusée : ${value ?? "absente"}`, () => assert.throws(() => parseLegalOrigin(value)));
}
test("la surcharge Web de l'origine est conservée", () => {
  const previous = globalThis.window;
  try {
    globalThis.window = { __FILFINANCE_ENV__: { EXPO_PUBLIC_LANDING_ORIGIN: "https://dev.filfinance.fr" } };
    assert.equal(readPublicEnv("EXPO_PUBLIC_LANDING_ORIGIN"), "https://dev.filfinance.fr");
  } finally { if (previous === undefined) delete globalThis.window; else globalThis.window = previous; }
});
