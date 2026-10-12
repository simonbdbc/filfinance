const fs = require("node:fs"),
  path = require("node:path"),
  assert = require("node:assert/strict");
const root = path.join(__dirname, "..");
const read = (f) => fs.readFileSync(path.join(root, f), "utf8");
const shell = read("src/application/layouts/AppLayout.tsx"),
  viewport = read("src/shared/ui/PageViewport.tsx"),
  audio = read("src/shared/ui/AudioPlayerView.tsx");
for (const fragment of [
  'maxHeight: "100vh"',
  '"fixed"',
  "<ScrollView",
  "width >= 1024",
  "<AppNavigation",
  "compact",
  "<DialogPanel",
  "onThemeNameChange: (themeName: ThemeName) => void",
  'testID="page-scroll"',
  'scrollTo({ y: 0, animated: false })',
  '}, [pageKey])',
  'accessibilityLabel="Basculer entre les thèmes clair et sombre"',
])
  assert(shell.includes(fragment), fragment);
assert.match(shell, /<ScrollView\s+ref=\{contentRef\}\s+testID="page-scroll"/);
assert.match(
  read("src/application/layouts/ProtectedAppLayout.tsx"),
  /pageKey=\{pathname\}/,
);
for (const fragment of ["ScrollView", 'height: "100vh"', 'overflowX: "hidden"'])
  assert(viewport.includes(fragment), fragment);
assert(!viewport.includes('overflowY: "hidden"'));
for (const label of [
  "Résumé audio du jour",
  "Résumé en préparation",
  "Résumé indisponible",
  "Arrêter la préparation",
  "Arrêter la lecture",
])
  assert(audio.includes(label), label);
assert.doesNotMatch(audio, /new Audio|useAudio|WebSocket|volume/i);
assert.match(read("src/shared/ui/DetailPanel.tsx"), /width >= 1280/);
assert.doesNotMatch(
  read("src/features/auth/components/AuthPageShell.tsx"),
  /AudioPlayerView|AudioControl/,
);
assert.doesNotMatch(
  read("src/features/feed/components/NewsFeedCard.tsx"),
  /Calendar|Calendrier/,
);
assert.match(
  read("src/application/components/AppNavigation.tsx"),
  /canAccessAdmin\(role\)/,
);
console.log(
  "[shell-contract] seuils responsive, navigation, commandes audio et séparation des pages OK",
);
