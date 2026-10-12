import { existsSync, readFileSync } from "node:fs";

export function loadLocalEnvFile(path = ".env", targetEnv = process.env) {
  if (!existsSync(path)) {
    return false;
  }

  const content = readFileSync(path, "utf8");
  for (const line of content.split(/\r?\n/u)) {
    const entry = parseEnvLine(line);
    if (!entry) {
      continue;
    }
    const [key, value] = entry;
    if (targetEnv[key] === undefined) {
      targetEnv[key] = value;
    }
  }

  return true;
}

export function loadLocalEnvFilePath(targetEnv = process.env) {
  return loadLocalEnvFile(targetEnv.FILFINANCE_ENV_FILE ?? ".env", targetEnv);
}

export function parseEnvLine(line) {
  const trimmed = line.trim();
  if (!trimmed || trimmed.startsWith("#")) {
    return null;
  }

  const assignment = trimmed.startsWith("export ") ? trimmed.slice("export ".length).trim() : trimmed;
  const separator = assignment.indexOf("=");
  if (separator <= 0) {
    return null;
  }

  const key = assignment.slice(0, separator).trim();
  const value = unquoteValue(assignment.slice(separator + 1).trim());
  if (!/^[A-Za-z_][A-Za-z0-9_]*$/u.test(key)) {
    return null;
  }

  return [key, value];
}

function unquoteValue(value) {
  if (
    (value.startsWith("\"") && value.endsWith("\"")) ||
    (value.startsWith("'") && value.endsWith("'"))
  ) {
    return value.slice(1, -1);
  }

  return value;
}
