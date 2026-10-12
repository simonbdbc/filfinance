const fs = require("node:fs"),
  path = require("node:path"),
  assert = require("node:assert/strict"),
  { execFileSync } = require("node:child_process");
const root = path.resolve(__dirname, "..");
const read = (f) => fs.readFileSync(path.join(root, f), "utf8");
const pkg = JSON.parse(read("package.json")),
  lock = JSON.parse(read("package-lock.json"));
for (const [name, version] of Object.entries({
  "@gluestack-ui/core": "5.0.15",
  "@gluestack-ui/utils": "5.0.6",
  uniwind: "1.3.0",
  tailwindcss: "4.1.18",
}))
  assert.equal(
    { ...pkg.dependencies, ...pkg.devDependencies }[name],
    version,
    name,
  );
for (const name of Object.keys(lock.packages))
  assert(
    !/(?:^|node_modules\/)(?:nativewind|react-native-css-interop)$/.test(name),
    name,
  );
for (const [name, p] of Object.entries(lock.packages))
  if (name.endsWith("/tailwindcss"))
    assert(p.version.startsWith("4."), `${name}: Tailwind 3 interdit`);
for (const file of [
  "tailwind.config.js",
  "nativewind-env.d.ts",
  "scripts/prepare-nativewind-web-cache.js",
  "src/shared/theme/themes.ts",
  "src/shared/theme/classColor.ts",
  "src/shared/theme/appDesignTheme.ts",
  "src/features/admin/components/dashboard/theme.ts",
])
  assert(!fs.existsSync(path.join(root, file)), `Legacy: ${file}`);
function walk(dir) {
  return fs
    .readdirSync(dir, { withFileTypes: true })
    .flatMap((e) =>
      e.isDirectory() ? walk(path.join(dir, e.name)) : [path.join(dir, e.name)],
    );
}
for (const file of [
  ...walk(path.join(root, "src")),
  ...["metro.config.js", "babel.config.js", "tsconfig.json", "global.css"].map(
    (f) => path.join(root, f),
  ),
]) {
  if (!/\.(?:tsx?|js|json|css)$/.test(file)) continue;
  const text = fs.readFileSync(file, "utf8");
  assert.doesNotMatch(
    text,
    /nativewind|react-native-css-interop|withNativeWind|cssInterop|remapProps|@tailwind (?:base|components|utilities)/i,
    file,
  );
  if (file.includes("/shared/ui/"))
    assert.doesNotMatch(
      text,
      /@features\/|@application|\bfetch\(/,
      `Composant métier interdit: ${file}`,
    );
}
assert.match(read("metro.config.js"), /withUniwindConfig/);
assert.match(read("global.css"), /@import "uniwind"/);
assert.match(
  read("src/shared/ui/primitives/index.tsx"),
  /@gluestack-ui\/core\/button\/creator/,
);
const tree = JSON.parse(
  execFileSync("npm", ["ls", "--all", "--json"], {
    cwd: root,
    encoding: "utf8",
    maxBuffer: 20 * 1024 * 1024,
  }),
);
function check(node) {
  for (const [name, dep] of Object.entries(node.dependencies || {})) {
    assert(!["nativewind", "react-native-css-interop"].includes(name), name);
    check(dep);
  }
}
check(tree);
console.log(
  "[ui-stack] gluestack v5 + UniWind + Tailwind 4 verrouillés ; aucun ancien moteur dans les sources ni les dépendances",
);
