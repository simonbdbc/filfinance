import assert from "node:assert/strict";
import test from "node:test";

import {
  clearResetCallbackFromBrowserUrl,
  readResetCallback,
  resetLinkErrorMessage,
} from "../src/features/auth/lib/resetLink.ts";

test("a reset link captures its token in memory and opens the new-password form", () => {
  const callback = readResetCallback("synthetic-test-token", undefined);

  assert.deepEqual(callback, {
    authMode: "reset-password",
    resetLinkError: null,
    resetToken: "synthetic-test-token",
  });
});

test("a reset callback error does not retain a token and returns to sign-in", () => {
  const callback = readResetCallback("synthetic-test-token", "TOKEN_EXPIRED");

  assert.equal(callback.authMode, "sign-in");
  assert.equal(callback.resetToken, undefined);
  assert.match(callback.resetLinkError, /invalide ou expiré/);
});

test("a reset callback is removed from the visible browser URL after memory capture", () => {
  const originalWindow = Object.getOwnPropertyDescriptor(globalThis, "window");
  let redactedUrl;
  Object.defineProperty(globalThis, "window", {
    configurable: true,
    value: {
      history: {
        state: { navigation: "test" },
        replaceState(_state, _title, nextUrl) {
          redactedUrl = nextUrl;
        },
      },
      location: {
        href: "https://app.filfinance.fr/login?mode=reset-password&token=synthetic-test-token&source=email#reset",
      },
    },
  });

  try {
    clearResetCallbackFromBrowserUrl();
  } finally {
    if (originalWindow) {
      Object.defineProperty(globalThis, "window", originalWindow);
    } else {
      Reflect.deleteProperty(globalThis, "window");
    }
  }

  assert.equal(redactedUrl, "/login?source=email#reset");
});

test("reset callback errors explain invalid and expired links", () => {
  assert.match(resetLinkErrorMessage("INVALID_TOKEN"), /invalide ou expiré/);
  assert.match(resetLinkErrorMessage("TOKEN_EXPIRED"), /invalide ou expiré/);
});

test("unknown reset callback errors remain understandable", () => {
  assert.match(resetLinkErrorMessage("unexpected"), /nouveau lien/);
  assert.equal(resetLinkErrorMessage(null), null);
});
