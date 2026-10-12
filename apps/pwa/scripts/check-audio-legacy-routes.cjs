const fs = require("node:fs");
const path = require("node:path");

const root = path.join(__dirname, "..", "src");
const forbidden = ["/ws/audio", "/audio/live"];

function walk(dir) {
  return fs.readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const fullPath = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      return walk(fullPath);
    }
    if (/\.(ts|tsx)$/.test(entry.name)) {
      return [fullPath];
    }
    return [];
  });
}

const offenders = [];
for (const filePath of walk(root)) {
  const source = fs.readFileSync(filePath, "utf8");
  for (const pattern of forbidden) {
    if (source.includes(pattern)) {
      offenders.push(`${path.relative(path.join(__dirname, ".."), filePath)}: ${pattern}`);
    }
  }
}

if (offenders.length > 0) {
  throw new Error(`Legacy audio route reference detected:\n${offenders.join("\n")}`);
}

console.log("[audio-legacy] no /ws/audio or /audio/live references in src");
