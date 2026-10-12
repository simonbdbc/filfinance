import assert from "node:assert/strict";
import test from "node:test";

import { shouldNavigateToHome } from "../src/session.js";

const homeUrl = "https://www.financialjuice.com/home";

test("session navigation opens home only when outside FinancialJuice", () => {
  assert.equal(shouldNavigateToHome("about:blank", homeUrl), true);
  assert.equal(shouldNavigateToHome("https://example.com/", homeUrl), true);
  assert.equal(shouldNavigateToHome("https://www.financialjuice.com/home", homeUrl), false);
  assert.equal(shouldNavigateToHome("https://www.financialjuice.com/login", homeUrl), false);
  assert.equal(shouldNavigateToHome("https://www.financialjuice.com/cdn-cgi/challenge-platform/h/b/orchestrate/chl_page/v1", homeUrl), false);
});
