const fs = require("node:fs");
const path = require("node:path");
const assert = require("node:assert/strict");
const root = path.resolve(__dirname, "..");
const read = (f) => fs.readFileSync(path.join(root, f), "utf8");
const css = read("global.css");
for (const [theme, values] of Object.entries({
  light: ["#F1E5D3", "#FBF5EB", "#382E26", "#796B5D", "#7B5E45"],
  dark: ["#111827", "#192535", "#FBF5EB", "#F1E5D3", "#8B5E3C", "#714426"],
})) {
  const block = css.match(new RegExp(`@variant ${theme}\\s*\\{([^}]+)`))[1];
  for (const value of values)
    assert(block.toLowerCase().includes(value.toLowerCase()), `${theme}: ${value}`);
  for (const token of [
    "background",
    "surface",
    "foreground",
    "secondary",
    "decorative",
    "primary",
    "primary-text",
    "border",
    "border-soft",
    "error",
    "success",
    "warning",
    "overlay",
    "shadow",
  ])
    assert(block.includes(`--ff-${token}:`), `${theme}: ${token}`);
}
assert.match(read("src/shared/theme/tokens.ts"), /useCSSVariable/);
assert.doesNotMatch(read("src/shared/theme/tokens.ts"), /#[a-f0-9]{3,8}\b/i);
assert.match(css, /Georgia/);
assert.match(css, /Arial/);
assert.match(css, /:focus-visible/);
assert.match(read("src/shared/theme/preference.ts"), /filfinance\.theme/);
assert.match(
  read("src/shared/theme/theme.tsx"),
  /Uniwind\.setTheme\(initialTheme\)/,
);
for (const file of [
  "src/features/profile/components/ProfilePage.tsx",
  "src/features/profile/hooks/useUserProfileRuntime.ts",
  "src/application/hooks/useAppRuntimeValue.ts",
])
  assert.doesNotMatch(
    read(file),
    /preferences\.theme|persistPreferredTheme|applyThemeName/,
  );
console.log(
  "[theme-contract] palettes CSS uniques, typographie et préférence locale OK",
);

// Les tokens textuels doivent rester lisibles sans dépendre de la maquette.
function luminance(hex) {
  return [1, 3, 5].map((offset) => {
    const channel = parseInt(hex.slice(offset, offset + 2), 16) / 255;
    return channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4;
  }).reduce((sum, channel, index) => sum + channel * [0.2126, 0.7152, 0.0722][index], 0);
}
for (const theme of ["light", "dark"]) {
  const block = css.match(new RegExp(`@variant ${theme}\\s*\\{([^}]+)`))[1];
  const colors = Object.fromEntries([...block.matchAll(/--ff-([\w-]+):\s*(#[\da-f]{6})/gi)].map((match) => [match[1], match[2]]));
  for (const [foreground, background] of [["foreground", "background"], ["foreground", "surface"], ["secondary", "surface"], ["primary-text", "primary"], ["error", "surface"], ["success", "surface"], ["warning", "surface"]]) {
    const [low, high] = [luminance(colors[foreground]), luminance(colors[background])].sort((a, b) => a - b);
    assert((high + 0.05) / (low + 0.05) >= 4.5, `${theme}: contraste ${foreground}/${background}`);
  }
}
console.log("[theme-contract] contrastes des textes ≥ 4,5:1");
