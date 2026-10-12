const fs = require("node:fs");
const path = require("node:path");

const projectRoot = path.join(__dirname, "..");
const repoRoot = path.join(projectRoot, "..", "..");
const urlsPath = path.join(projectRoot, "src", "shared", "api", "urls.ts");
const ciPath = path.join(repoRoot, ".gitlab-ci.yml");

const urlsSource = fs.readFileSync(urlsPath, "utf8");
const ciSource = fs.readFileSync(ciPath, "utf8");

function assertIncludes(source, expected, label) {
  if (!source.includes(expected)) {
    throw new Error(`${label}: missing ${expected}`);
  }
}

function assertExcludes(source, unexpected, label) {
  if (source.includes(unexpected)) {
    throw new Error(`${label}: unexpected ${unexpected}`);
  }
}

assertIncludes(
  urlsSource,
  'return resolveApiResourceUrl(wsUrl, resourceUrl);',
  "HLS base vide",
);
assertIncludes(
  urlsSource,
  "`${resource.pathname}${resource.search}${resource.hash}`",
  "Base HLS externe",
);
assertExcludes(
  urlsSource,
  "`/hls${resource.pathname}`",
  "Le resolver HLS client ne doit plus prefixer /audio par /hls",
);
assertExcludes(
  ciSource,
  'EXPO_PUBLIC_FILFINANCE_HLS_BASE_URL="${EXPO_PUBLIC_FILFINANCE_HLS_BASE_URL:-https://live.filfinance.fr}"',
  "La CI main ne doit plus fallback vers live.filfinance.fr",
);
assertExcludes(
  ciSource,
  'EXPO_PUBLIC_FILFINANCE_HLS_BASE_URL="${EXPO_PUBLIC_FILFINANCE_HLS_BASE_URL:-https://live.dev.filfinance.fr}"',
  "La CI dev ne doit plus fallback vers live.dev.filfinance.fr",
);
assertIncludes(
  ciSource,
  'EXPO_PUBLIC_FILFINANCE_HLS_BASE_URL="$FILFINANCE_DEV_HLS_BASE_URL"',
  "La CI dev doit injecter FILFINANCE_DEV_HLS_BASE_URL sans override EXPO_PUBLIC partage",
);
assertIncludes(
  ciSource,
  'EXPO_PUBLIC_FILFINANCE_HLS_BASE_URL="$FILFINANCE_PROD_HLS_BASE_URL"',
  "La CI prod doit injecter FILFINANCE_PROD_HLS_BASE_URL sans override EXPO_PUBLIC partage",
);

console.log("[hls-url-contract] generic HLS base keeps /audio paths and CI has no live fallback");
