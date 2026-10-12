import assert from "node:assert/strict";
import test from "node:test";

import { parseEnvLine } from "../src/env-file.js";

test("parseEnvLine reads quoted and unquoted assignments", () => {
  assert.deepEqual(parseEnvLine("NEWS_FLAG=true"), ["NEWS_FLAG", "true"]);
  assert.deepEqual(parseEnvLine("export NEWS_VALUE=\"hello world\""), ["NEWS_VALUE", "hello world"]);
});

test("parseEnvLine ignores comments and invalid keys", () => {
  assert.equal(parseEnvLine("# comment"), null);
  assert.equal(parseEnvLine("1_BAD=value"), null);
});
