import assert from "node:assert/strict";
import test from "node:test";
import { developmentMailLog } from "./mailer.js";

test("development mail fallback never logs recipients, links, or tokens", () => {
  const output = developmentMailLog();

  assert.match(output, /SMTP transport is not configured/);
  assert.doesNotMatch(output, /user@example\.test/);
  assert.doesNotMatch(output, /verify-email|token=/);
});
