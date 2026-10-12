const fs = require("node:fs");
const path = require("node:path");

const projectRoot = path.join(__dirname, "..");
const srcRoot = path.join(projectRoot, "src");
const appRoot = path.join(projectRoot, "app");
const applicationRoot = path.join(srcRoot, "application");
const featuresRoot = path.join(srcRoot, "features");
const removedAppShellFeatureRoot = path.join(featuresRoot, "app-shell");
const allowedApplicationRootEntries = new Set([
  "components",
  "hooks",
  "index.ts",
  "layouts",
  "navigation",
  "providers",
]);
const allowedFeatureRootEntries = new Set([
  "components",
  "hooks",
  "index.ts",
  "lib",
  "providers",
  "screens",
  "services",
  "types",
]);

function walk(dir) {
  if (!fs.existsSync(dir)) {
    return [];
  }

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

function relativePath(filePath) {
  return path.relative(projectRoot, filePath).split(path.sep).join("/");
}

function isInside(childPath, parentPath) {
  const relative = path.relative(parentPath, childPath);
  return Boolean(relative) && !relative.startsWith("..") && !path.isAbsolute(relative);
}

function featureNameForPath(filePath) {
  if (!isInside(filePath, featuresRoot)) {
    return null;
  }

  return path.relative(featuresRoot, filePath).split(path.sep)[0] ?? null;
}

function resolveRelativeImport(sourceFile, specifier) {
  if (!specifier.startsWith(".")) {
    return null;
  }

  return path.resolve(path.dirname(sourceFile), specifier);
}

function collectImportSpecifiers(source) {
  const specifiers = [];
  const fromPattern = /\b(?:import|export)\b[\s\S]*?\bfrom\s*["']([^"']+)["']/g;
  const dynamicPattern = /\bimport\s*\(\s*["']([^"']+)["']\s*\)/g;

  for (const match of source.matchAll(fromPattern)) {
    specifiers.push(match[1]);
  }
  for (const match of source.matchAll(dynamicPattern)) {
    specifiers.push(match[1]);
  }

  return specifiers;
}

const offenders = [];

if (!fs.existsSync(applicationRoot)) {
  offenders.push("src/application: missing application composition layer");
}

if (fs.existsSync(removedAppShellFeatureRoot)) {
  offenders.push("src/features/app-shell: app-shell must live in src/application");
}

if (fs.existsSync(applicationRoot)) {
  const publicApplicationIndexPath = path.join(applicationRoot, "index.ts");
  if (!fs.existsSync(publicApplicationIndexPath)) {
    offenders.push("src/application: missing public index.ts");
  }

  for (const entry of fs.readdirSync(applicationRoot, { withFileTypes: true })) {
    if (!allowedApplicationRootEntries.has(entry.name)) {
      offenders.push(
        `src/application/${entry.name}: application root entry must be one of ${[...allowedApplicationRootEntries].sort().join(", ")}`,
      );
      continue;
    }

    if (entry.name === "index.ts" && !entry.isFile()) {
      offenders.push("src/application/index.ts: public index must be a file");
    }
    if (entry.name !== "index.ts" && !entry.isDirectory()) {
      offenders.push(`src/application/${entry.name}: application section must be a directory`);
    }
  }
}

for (const featureEntry of fs.readdirSync(featuresRoot, { withFileTypes: true })) {
  if (!featureEntry.isDirectory()) {
    offenders.push(`src/features/${featureEntry.name}: feature root must only contain feature directories`);
    continue;
  }

  const featurePath = path.join(featuresRoot, featureEntry.name);
  const publicIndexPath = path.join(featurePath, "index.ts");
  if (!fs.existsSync(publicIndexPath)) {
    offenders.push(`src/features/${featureEntry.name}: missing public index.ts`);
  }

  for (const entry of fs.readdirSync(featurePath, { withFileTypes: true })) {
    if (!allowedFeatureRootEntries.has(entry.name)) {
      offenders.push(
        `src/features/${featureEntry.name}/${entry.name}: feature root entry must be one of ${[...allowedFeatureRootEntries].sort().join(", ")}`,
      );
      continue;
    }

    if (entry.name === "index.ts" && !entry.isFile()) {
      offenders.push(`src/features/${featureEntry.name}/index.ts: public index must be a file`);
    }
    if (entry.name !== "index.ts" && !entry.isDirectory()) {
      offenders.push(`src/features/${featureEntry.name}/${entry.name}: feature section must be a directory`);
    }
  }
}

for (const filePath of [...walk(srcRoot), ...walk(appRoot)]) {
  const source = fs.readFileSync(filePath, "utf8");
  const sourceFeatureName = featureNameForPath(filePath);
  const isApplicationFile = isInside(filePath, applicationRoot);

  for (const specifier of collectImportSpecifiers(source)) {
    if (specifier === "@app" || specifier.startsWith("@app/")) {
      offenders.push(
        `${relativePath(filePath)}: import ${specifier} uses removed @app alias; use @application or @shared/ui`,
      );
    }

    if (specifier === "@features/app-shell" || specifier.startsWith("@features/app-shell/")) {
      offenders.push(
        `${relativePath(filePath)}: import ${specifier} uses removed app-shell feature; use @application`,
      );
    }

    if (!isApplicationFile && specifier.startsWith("@application/")) {
      offenders.push(
        `${relativePath(filePath)}: import ${specifier} bypasses the application public index`,
      );
    }

    if (specifier.startsWith("@features/") && specifier.split("/").length > 2) {
      offenders.push(
        `${relativePath(filePath)}: import ${specifier} bypasses the feature public index`,
      );
    }

    const resolvedPath = resolveRelativeImport(filePath, specifier);
    if (!resolvedPath) {
      continue;
    }

    if (!isApplicationFile && isInside(resolvedPath, applicationRoot)) {
      offenders.push(
        `${relativePath(filePath)}: relative import ${specifier} crosses into application; use @application`,
      );
    }

    if (!isInside(resolvedPath, featuresRoot)) {
      continue;
    }

    const targetFeatureName = featureNameForPath(resolvedPath);
    if (targetFeatureName && targetFeatureName !== sourceFeatureName) {
      offenders.push(
        `${relativePath(filePath)}: relative import ${specifier} crosses into feature ${targetFeatureName}; use @features/${targetFeatureName}`,
      );
    }
  }
}

if (offenders.length > 0) {
  throw new Error(`Feature architecture contract failed:\n${offenders.join("\n")}`);
}

console.log("[feature-architecture] public feature imports and root layout OK");
