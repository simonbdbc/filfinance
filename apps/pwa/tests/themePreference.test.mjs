import test from "node:test";
import assert from "node:assert/strict";
import {
  readWebTheme,
  isThemeName,
  themeStorageKey,
} from "../src/shared/theme/preference.ts";
for (const [value, expected] of [
  [null, "light"],
  ["system", "light"],
  ["LIGHT", "light"],
  ["dark", "dark"],
  ["light", "light"],
])
  test(`préférence ${value}: ${expected}`, () => {
    globalThis.window = {
      localStorage: {
        getItem(key) {
          assert.equal(key, themeStorageKey);
          return value;
        },
      },
    };
    assert.equal(readWebTheme(), expected);
    delete globalThis.window;
  });
test("stockage refusé et rendu hors navigateur démarrent en clair", () => {
  globalThis.window = {
    get localStorage() {
      throw new Error("SecurityError");
    },
  };
  assert.equal(readWebTheme(), "light");
  delete globalThis.window;
  assert.equal(readWebTheme(), "light");
});
test("validation fermée des thèmes", () => {
  assert.equal(isThemeName("dark"), true);
  for (const v of ["system", "", undefined, {}, null])
    assert.equal(isThemeName(v), false);
});
