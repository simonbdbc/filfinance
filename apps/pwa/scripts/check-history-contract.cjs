const fs = require("node:fs");
const path = require("node:path");

const projectRoot = path.join(__dirname, "..");
const routesPath = path.join(
  projectRoot,
  "src",
  "application",
  "navigation",
  "routes.ts",
);
const historyCardPath = path.join(
  projectRoot,
  "src",
  "features",
  "history",
  "components",
  "AdminFeedDiagnosticsCard.tsx",
);

const routesSource = fs.readFileSync(routesPath, "utf8");
const historyCardSource = fs.readFileSync(historyCardPath, "utf8");

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
  routesSource,
  'calendar: "Calendrier économique"',
  "calendar navigation label",
);
assertExcludes(
  routesSource,
  "Calendrier hebdomadaire",
  "calendar navigation label",
);

assertIncludes(
  historyCardSource,
  "active -> Élevée",
  "history active level badge",
);
assertIncludes(
  historyCardSource,
  "news-general -> Général",
  "history news-general level badge",
);
assertIncludes(
  historyCardSource,
  "none -> Faible",
  "history fallback level badge",
);
assertIncludes(
  historyCardSource,
  "Contenu anglais n/a",
  "history English empty content label",
);
assertIncludes(
  historyCardSource,
  "Contenu français n/a",
  "history French empty content label",
);
assertExcludes(
  historyCardSource,
  "Corps anglais n/a",
  "history English empty content label",
);
assertExcludes(
  historyCardSource,
  "Corps français n/a",
  "history French empty content label",
);

console.log(
  "[history-contract] labels, level badges and empty content labels OK",
);
