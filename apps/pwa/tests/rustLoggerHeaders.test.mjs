import assert from "node:assert/strict";
import test from "node:test";

import { mergeReaderHeaders } from "../src/features/admin/lib/rustLogger/headers.ts";

test("le lecteur conserve l'identité conditionnelle des opérations sur fichier", () => {
  const headers = mergeReaderHeaders(
    { "If-Match": "device:inode:revision" },
    {
      Authorization: "Bearer access-token",
      "Content-Type": "application/json",
      "x-local-session": "reader-session",
    },
  );

  assert.equal(headers.get("if-match"), "device:inode:revision");
  assert.equal(headers.get("authorization"), "Bearer access-token");
  assert.equal(headers.get("content-type"), "application/json");
  assert.equal(headers.get("x-local-session"), "reader-session");
});

test("les en-têtes d'authentification du lecteur ne peuvent pas être remplacés", () => {
  const headers = mergeReaderHeaders(
    {
      Authorization: "Bearer foreign-token",
      "x-local-session": "foreign-session",
    },
    {
      Authorization: "Bearer access-token",
      "x-local-session": "reader-session",
    },
  );

  assert.equal(headers.get("authorization"), "Bearer access-token");
  assert.equal(headers.get("x-local-session"), "reader-session");
});
