import assert from "node:assert/strict";
import test from "node:test";

import { verificationEmailContent } from "./email-templates.js";

test("verification email preserves the complete callback URL in text and HTML", () => {
  const url = "https://api.filfinance.fr/api/auth/verify-email?token=token-value&callbackURL=https%3A%2F%2Fapp.filfinance.fr%2Femail-verification%3Fstatus%3Dsuccess";
  const content = verificationEmailContent(url);

  assert.match(content.text, new RegExp(`<${url.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}>`));
  assert.ok(content.html.includes("token=token-value&amp;callbackURL="));
  assert.ok(content.html.includes("Verifier mon adresse"));
  assert.ok(!content.html.includes("token=token-value&callbackURL="));
});

test("verification email escapes unsafe HTML attribute characters", () => {
  const content = verificationEmailContent('https://example.test/verify?value="<unsafe>"');

  assert.ok(content.html.includes("&quot;&lt;unsafe&gt;&quot;"));
  assert.ok(!content.html.includes('href="https://example.test/verify?value="<unsafe>"'));
});
