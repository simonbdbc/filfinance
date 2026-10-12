const DEFAULT_HOME_URL = "https://www.financialjuice.com/home";
const DEFAULT_BACKEND_URL = "http://127.0.0.1:8088";
const DEFAULT_TARGET_NAME = "default";
const TARGET_NAME_PATTERN = /^[A-Za-z0-9_]+$/u;

export function loadConfig(env = process.env) {
  const enabled = readBoolean(env.NEWS_FINANCIAL_JUICE_BROWSER_ENABLED, false);
  const targets = loadBackendTargets(env);
  const primaryTarget = targets[0];

  return {
    enabled,
    homeUrl: trimOrDefault(env.NEWS_FINANCIAL_JUICE_BROWSER_HOME_URL, DEFAULT_HOME_URL),
    targets,
    backendUrl: primaryTarget.backendUrl,
    newsEndpoint: primaryTarget.newsEndpoint,
    statusEndpoint: primaryTarget.statusEndpoint,
    controlEndpoint: primaryTarget.controlEndpoint,
    ingestToken: primaryTarget.ingestToken,
    headless: readBoolean(env.NEWS_FINANCIAL_JUICE_BROWSER_HEADLESS, false),
    pollIntervalMs: readInteger(env.NEWS_FINANCIAL_JUICE_BROWSER_POLL_INTERVAL_MS, 5_000),
    requestTimeoutMs: readInteger(env.NEWS_FINANCIAL_JUICE_BROWSER_REQUEST_TIMEOUT_MS, 15_000),
    rateLimitBackoffMs: readInteger(
      env.NEWS_FINANCIAL_JUICE_BROWSER_RATE_LIMIT_BACKOFF_MS,
      60_000,
    ),
    maxBatchSize: readInteger(env.NEWS_FINANCIAL_JUICE_BROWSER_MAX_BATCH_SIZE, 100),
    historyPageLimit: readInteger(
      env.NEWS_FINANCIAL_JUICE_BROWSER_HISTORY_PAGE_LIMIT,
      120,
    ),
    historyRefreshMs: readInteger(
      env.NEWS_FINANCIAL_JUICE_BROWSER_HISTORY_REFRESH_MS,
      10 * 60_000,
    ),
    historyTimezone: trimOrDefault(
      env.NEWS_FINANCIAL_JUICE_BROWSER_HISTORY_TIMEZONE,
      "Europe/Paris",
    ),
    userDataDir: trimOrDefault(
      env.NEWS_FINANCIAL_JUICE_BROWSER_USER_DATA_DIR,
      "data/financialjuice-browser-profile",
    ),
    email: trimOptional(env.NEWS_FINANCIAL_JUICE_EMAIL),
    password: trimOptional(env.NEWS_FINANCIAL_JUICE_PASSWORD),
  };
}

export function validateConfig(config) {
  const errors = [];

  if (!config.enabled) {
    return errors;
  }
  if (config.targets.length === 0) {
    errors.push("at least one FinancialJuice browser backend target is required when the browser collector is enabled");
  }
  for (const target of config.targets) {
    if (!target.backendUrl) {
      errors.push(`${target.backendUrlEnvKey} is required when the browser collector target "${target.name}" is enabled`);
    }
    if (!target.ingestToken || target.ingestToken === "your-browser-collector-token") {
      errors.push(`${target.ingestTokenEnvKey} is required when the browser collector target "${target.name}" is enabled`);
    }
  }
  if (!config.email) {
    errors.push("NEWS_FINANCIAL_JUICE_EMAIL is required when the browser collector is enabled");
  }
  if (!config.password) {
    errors.push("NEWS_FINANCIAL_JUICE_PASSWORD is required when the browser collector is enabled");
  }
  if (config.maxBatchSize < 1 || config.maxBatchSize > 500) {
    errors.push("NEWS_FINANCIAL_JUICE_BROWSER_MAX_BATCH_SIZE must be between 1 and 500");
  }
  if (config.historyPageLimit < 1 || config.historyPageLimit > 500) {
    errors.push("NEWS_FINANCIAL_JUICE_BROWSER_HISTORY_PAGE_LIMIT must be between 1 and 500");
  }
  if (config.historyRefreshMs < 60_000) {
    errors.push("NEWS_FINANCIAL_JUICE_BROWSER_HISTORY_REFRESH_MS must be at least 60000");
  }

  return errors;
}

function loadBackendTargets(env) {
  const targetNames = readList(env.NEWS_FINANCIAL_JUICE_BROWSER_TARGETS);
  if (targetNames.length === 0) {
    return [
      buildBackendTarget({
        name: DEFAULT_TARGET_NAME,
        backendUrl: trimOrDefault(
          env.NEWS_FINANCIAL_JUICE_BROWSER_BACKEND_URL,
          DEFAULT_BACKEND_URL,
        ),
        ingestToken: trimOptional(env.NEWS_FINANCIAL_JUICE_BROWSER_INGEST_TOKEN),
        backendUrlEnvKey: "NEWS_FINANCIAL_JUICE_BROWSER_BACKEND_URL",
        ingestTokenEnvKey: "NEWS_FINANCIAL_JUICE_BROWSER_INGEST_TOKEN",
      }),
    ];
  }

  return targetNames.map((name) => {
    if (!TARGET_NAME_PATTERN.test(name)) {
      throw new Error(
        `invalid NEWS_FINANCIAL_JUICE_BROWSER_TARGETS entry "${name}": only letters, digits and underscores are supported`,
      );
    }
    const suffix = name.toUpperCase();
    const backendUrlEnvKey = `NEWS_FINANCIAL_JUICE_BROWSER_BACKEND_URL_${suffix}`;
    const targetTokenEnvKey = `NEWS_FINANCIAL_JUICE_BROWSER_INGEST_TOKEN_${suffix}`;
    const targetToken = trimOptional(env[targetTokenEnvKey]);
    const fallbackToken = trimOptional(env.NEWS_FINANCIAL_JUICE_BROWSER_INGEST_TOKEN);

    return buildBackendTarget({
      name,
      backendUrl: trimOptional(env[backendUrlEnvKey]),
      ingestToken: targetToken ?? fallbackToken,
      backendUrlEnvKey,
      ingestTokenEnvKey: targetToken || fallbackToken
        ? (targetToken ? targetTokenEnvKey : "NEWS_FINANCIAL_JUICE_BROWSER_INGEST_TOKEN")
        : targetTokenEnvKey,
    });
  });
}

function buildBackendTarget({
  name,
  backendUrl,
  ingestToken,
  backendUrlEnvKey,
  ingestTokenEnvKey,
}) {
  return {
    name,
    backendUrl,
    ingestToken,
    backendUrlEnvKey,
    ingestTokenEnvKey,
    newsEndpoint: endpointFor(backendUrl, "/api/ingest/financial-juice/browser/news"),
    statusEndpoint: endpointFor(backendUrl, "/api/ingest/financial-juice/browser/status"),
    controlEndpoint: endpointFor(backendUrl, "/api/ingest/financial-juice/browser/control"),
  };
}

function endpointFor(backendUrl, pathname) {
  if (!backendUrl) {
    return null;
  }
  return new URL(pathname, `${backendUrl}/`).toString();
}

function readList(value) {
  const normalized = trimOptional(value);
  if (!normalized) {
    return [];
  }
  return normalized
    .split(",")
    .map((entry) => entry.trim())
    .filter((entry) => entry.length > 0);
}

function trimOptional(value) {
  const trimmed = typeof value === "string" ? value.trim() : "";
  return trimmed.length > 0 ? trimmed : null;
}

function trimOrDefault(value, fallback) {
  return trimOptional(value) ?? fallback;
}

function readBoolean(value, fallback) {
  const normalized = trimOptional(value)?.toLowerCase();
  if (!normalized) {
    return fallback;
  }
  if (["1", "true", "yes", "on"].includes(normalized)) {
    return true;
  }
  if (["0", "false", "no", "off"].includes(normalized)) {
    return false;
  }
  throw new Error(`invalid boolean value: ${value}`);
}

function readInteger(value, fallback) {
  const normalized = trimOptional(value);
  if (!normalized) {
    return fallback;
  }
  const parsed = Number.parseInt(normalized, 10);
  if (!Number.isSafeInteger(parsed) || parsed <= 0) {
    throw new Error(`invalid positive integer value: ${value}`);
  }
  return parsed;
}
