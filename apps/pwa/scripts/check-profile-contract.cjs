const fs = require("node:fs");
const path = require("node:path");

const projectRoot = path.join(__dirname, "..");
const profilePagePath = path.join(
  projectRoot,
  "src",
  "features",
  "profile",
  "components",
  "ProfilePage.tsx",
);

const source = fs.readFileSync(profilePagePath, "utf8");

function assertIncludes(expected, label) {
  if (!source.includes(expected)) {
    throw new Error(`${label}: missing ${expected}`);
  }
}

function assertExcludes(unexpected, label) {
  if (source.includes(unexpected)) {
    throw new Error(`${label}: unexpected ${unexpected}`);
  }
}

assertIncludes("Voix TTS préférée", "profile preferred voice label");
assertIncludes("Thème", "profile theme label");
assertIncludes("Mise à jour", "profile updated label");
assertIncludes("Création", "profile created label");

assertExcludes("ID profil", "profile technical id");
assertExcludes("Voix TTS preferee", "profile preferred voice label");
assertExcludes("Theme:", "profile theme label");
assertExcludes("Mise a jour", "profile updated label");
assertExcludes("Creation:", "profile created label");

console.log("[profile-contract] labels are accented and technical profile id is hidden");
