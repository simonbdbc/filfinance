const fs = require("node:fs");
const path = require("node:path");

const projectRoot = path.join(__dirname, "..");
const blacklistScreenPath = path.join(
  projectRoot,
  "src",
  "features",
  "admin",
  "screens",
  "AdminBlacklistScreen.tsx",
);
const contractsPath = path.join(projectRoot, "src", "shared", "api", "contracts.ts");
const urlsPath = path.join(projectRoot, "src", "shared", "api", "urls.ts");
const hookPath = path.join(
  projectRoot,
  "src",
  "features",
  "admin",
  "hooks",
  "useNewsBlacklist.ts",
);
const apiPath = path.join(
  projectRoot,
  "src",
  "features",
  "admin",
  "services",
  "newsBlacklistApi.ts",
);
const blacklistScreen = fs.readFileSync(blacklistScreenPath, "utf8");
const contracts = fs.readFileSync(contractsPath, "utf8");
const urls = fs.readFileSync(urlsPath, "utf8");
const hook = fs.readFileSync(hookPath, "utf8");
const api = fs.readFileSync(apiPath, "utf8");

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

assertIncludes(contracts, 'NewsBlacklistTargetScope = "title" | "content" | "title_content"', "blacklist target scope type");
assertIncludes(contracts, 'NewsBlacklistAction = "reject_news" | "strip_content"', "blacklist action type");
assertIncludes(blacklistScreen, "Champ filtré", "blacklist target scope selector label");
assertIncludes(blacklistScreen, "Titre + contenu", "blacklist title and content option");
assertIncludes(blacklistScreen, "Contenu", "blacklist content label");
assertIncludes(blacklistScreen, "celui-ci est retiré de l’actualité en direct, mais son titre est conservé", "blacklist content-only help");
assertIncludes(blacklistScreen, "Dernières actions de blocage", "blacklist actions audit title");
assertIncludes(blacklistScreen, "contenu retiré", "blacklist strip content action label");
assertIncludes(blacklistScreen, "actualité rejetée", "blacklist reject action label");
assertIncludes(blacklistScreen, "Actualiser la liste de blocage", "blacklist reapply button");
assertIncludes(blacklistScreen, "actualisation", "blacklist reapply loading badge");
assertIncludes(contracts, "NewsBlacklistReapplyResponse", "blacklist reapply response type");
assertIncludes(urls, "/api/news-blacklist/reapply", "blacklist reapply URL");
assertIncludes(api, "reapplyNewsBlacklist", "blacklist reapply API call");
assertIncludes(hook, "isReapplying", "blacklist reapply hook state");
assertIncludes(hook, "reapplyBlacklist", "blacklist reapply hook action");
assertExcludes(blacklistScreen.toLowerCase(), "corps", "blacklist visible body wording");

console.log("[blacklist-contract] target scope, content wording and reapply action OK");
