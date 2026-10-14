const fs = require("node:fs");
const path = require("node:path");

const projectRoot = path.join(__dirname, "..");
const repoRoot = path.join(projectRoot, "..", "..");
const urlsPath = path.join(projectRoot, "src", "shared", "api", "urls.ts");

const urlsSource = fs.readFileSync(urlsPath, "utf8");

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

console.log("[hls-url-contract] generic local HLS base keeps /audio paths; application CI is not exported");
