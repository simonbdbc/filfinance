#!/usr/bin/env node

const fs = require("fs");
const http = require("http");
const https = require("https");
const path = require("path");
const { URL } = require("url");

const LEGACY_AUTH_PROVIDER = ["supa", "base"].join("");
const LEGACY_AUTH_ENV_PREFIX = `EXPO_PUBLIC_${LEGACY_AUTH_PROVIDER.toUpperCase()}`;
const REMOVED_LOGTO_PUBLIC_ENV_KEYS = [
  "EXPO_PUBLIC_LOGTO_ENDPOINT",
  "EXPO_PUBLIC_LOGTO_APP_ID",
  "EXPO_PUBLIC_LOGTO_REDIRECT_URI",
  "EXPO_PUBLIC_LOGTO_API_RESOURCE",
];

function parseArgs(argv) {
  const args = {
    dist: "",
    retryDelayMs: 5000,
    retries: 1,
    target: "dev",
    url: "",
  };

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    const next = argv[index + 1];
    if (arg === "--dist") {
      args.dist = next ?? "";
      index += 1;
      continue;
    }
    if (arg === "--retry-delay-ms") {
      args.retryDelayMs = Number(next ?? args.retryDelayMs);
      index += 1;
      continue;
    }
    if (arg === "--retries") {
      args.retries = Number(next ?? args.retries);
      index += 1;
      continue;
    }
    if (arg === "--target") {
      args.target = next ?? args.target;
      index += 1;
      continue;
    }
    if (arg === "--url") {
      args.url = next ?? "";
      index += 1;
      continue;
    }
    throw new Error(`Unknown argument: ${arg}`);
  }

  if (!args.dist && !args.url) {
    args.dist = "dist";
  }
  if (!["dev", "prod"].includes(args.target)) {
    throw new Error(`Unsupported target: ${args.target}`);
  }
  if (!Number.isInteger(args.retries) || args.retries < 1) {
    throw new Error("--retries must be a positive integer");
  }
  if (!Number.isInteger(args.retryDelayMs) || args.retryDelayMs < 0) {
    throw new Error("--retry-delay-ms must be a positive integer or 0");
  }

  return args;
}

function expectedContract(target) {
  if (target === "prod") {
    return {
      forbidden: [
        `${LEGACY_AUTH_PROVIDER}.co`,
        `${LEGACY_AUTH_ENV_PREFIX}_URL`,
        `${LEGACY_AUTH_ENV_PREFIX}_PUBLISHABLE_KEY`,
        `${LEGACY_AUTH_ENV_PREFIX}_ANON_KEY`,
        "https://filfinancedevhls.b-cdn.net",
        "https://media.dev.filfinance.fr",
        "https://live.filfinance.fr/audio",
        "EXPO_PUBLIC_FILFINANCE_HLS_BASE_URL:\"https://live.filfinance.fr\"",
        "https://tenant.logto.app",
        "logto-native-app-id",
        ...REMOVED_LOGTO_PUBLIC_ENV_KEYS,
      ],
      required: {
        EXPO_PUBLIC_LANDING_ORIGIN: "https://filfinance.fr",
        EXPO_PUBLIC_API_URL:
          process.env.FILFINANCE_EXPECTED_PROD_API_URL ||
          "https://api.filfinance.fr",
        EXPO_PUBLIC_BETTER_AUTH_URL:
          process.env.FILFINANCE_EXPECTED_PROD_BETTER_AUTH_URL ||
          "https://api.filfinance.fr",
        EXPO_PUBLIC_FILFINANCE_EDGE_WS_URL:
          process.env.FILFINANCE_EXPECTED_PROD_EDGE_WS_URL ||
          "wss://live.filfinance.fr/ws/live",
        EXPO_PUBLIC_FILFINANCE_HLS_BASE_URL:
          process.env.FILFINANCE_EXPECTED_PROD_HLS_BASE_URL ||
          "https://media.filfinance.fr",
        EXPO_PUBLIC_FILFINANCE_PWA_URL:
          process.env.FILFINANCE_EXPECTED_PROD_PWA_URL ||
          "https://app.filfinance.fr",
        EXPO_PUBLIC_FILFINANCE_WS_URL:
          process.env.FILFINANCE_EXPECTED_PROD_WS_URL ||
          "wss://api.filfinance.fr/ws/news",
      },
    };
  }

  return {
    forbidden: [
      `${LEGACY_AUTH_PROVIDER}.co`,
      `${LEGACY_AUTH_ENV_PREFIX}_URL`,
      `${LEGACY_AUTH_ENV_PREFIX}_PUBLISHABLE_KEY`,
      `${LEGACY_AUTH_ENV_PREFIX}_ANON_KEY`,
      "https://filfinancedevhls.b-cdn.net",
      "https://live.dev.filfinance.fr/audio",
      "https://tenant.logto.app",
      "logto-native-app-id",
      ...REMOVED_LOGTO_PUBLIC_ENV_KEYS,
    ],
    required: {
      EXPO_PUBLIC_LANDING_ORIGIN: "https://dev.filfinance.fr",
      EXPO_PUBLIC_API_URL:
        process.env.FILFINANCE_EXPECTED_DEV_API_URL ||
        "https://api.dev.filfinance.fr",
      EXPO_PUBLIC_BETTER_AUTH_URL:
        process.env.FILFINANCE_EXPECTED_DEV_BETTER_AUTH_URL ||
        "https://api.dev.filfinance.fr",
      EXPO_PUBLIC_FILFINANCE_EDGE_WS_URL:
        process.env.FILFINANCE_EXPECTED_DEV_EDGE_WS_URL ||
        "wss://live.dev.filfinance.fr/ws/live",
      EXPO_PUBLIC_FILFINANCE_HLS_BASE_URL:
        process.env.FILFINANCE_EXPECTED_DEV_HLS_BASE_URL ||
        "https://media.dev.filfinance.fr",
      EXPO_PUBLIC_FILFINANCE_PWA_URL:
        process.env.FILFINANCE_EXPECTED_DEV_PWA_URL ||
        process.env.FILFINANCE_DEV_PWA_URL ||
        "https://app.dev.filfinance.fr",
      EXPO_PUBLIC_FILFINANCE_WS_URL:
        process.env.FILFINANCE_EXPECTED_DEV_WS_URL ||
        "wss://api.dev.filfinance.fr/ws/news",
    },
  };
}

function findJavaScriptFiles(rootDir) {
  const files = [];

  function walk(dir) {
    for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
      const filePath = path.join(dir, entry.name);
      if (entry.isDirectory()) {
        walk(filePath);
        continue;
      }
      if (entry.isFile() && entry.name.endsWith(".js")) {
        files.push(filePath);
      }
    }
  }

  walk(rootDir);
  return files;
}

function readDistSource(distDir) {
  const absoluteDist = path.resolve(distDir);
  const jsRoot = path.join(absoluteDist, "_expo", "static", "js", "web");
  const sourceRoot = fs.existsSync(jsRoot) ? jsRoot : absoluteDist;
  const jsFiles = findJavaScriptFiles(sourceRoot);
  if (jsFiles.length === 0) {
    throw new Error(`No JavaScript bundle found under ${sourceRoot}`);
  }
  return {
    label: absoluteDist,
    source: jsFiles.map((file) => fs.readFileSync(file, "utf8")).join("\n"),
  };
}

function fetchText(url, redirectCount = 0) {
  return new Promise((resolve, reject) => {
    const parsedUrl = new URL(url);
    const transport = parsedUrl.protocol === "http:" ? http : https;
    const request = transport.get(parsedUrl, (response) => {
      const statusCode = response.statusCode ?? 0;
      const location = response.headers.location;

      if (
        location &&
        statusCode >= 300 &&
        statusCode < 400 &&
        redirectCount < 5
      ) {
        response.resume();
        resolve(fetchText(new URL(location, url).toString(), redirectCount + 1));
        return;
      }

      let body = "";
      response.setEncoding("utf8");
      response.on("data", (chunk) => {
        body += chunk;
      });
      response.on("end", () => {
        if (statusCode < 200 || statusCode >= 300) {
          reject(new Error(`GET ${url} returned HTTP ${statusCode}`));
          return;
        }
        resolve(body);
      });
    });

    request.setTimeout(15000, () => {
      request.destroy(new Error(`GET ${url} timed out`));
    });
    request.on("error", reject);
  });
}

function resolveScriptUrl(pageUrl, scriptSrc) {
  return new URL(scriptSrc, pageUrl).toString();
}

async function readRemoteSource(pageUrl) {
  const page = await fetchText(pageUrl);
  const scriptUrls = [...page.matchAll(/<script[^>]+src=["']([^"']+\.js)["']/gi)]
    .map((match) => resolveScriptUrl(pageUrl, match[1]));
  const expoScriptUrls = scriptUrls.filter((scriptUrl) =>
    scriptUrl.includes("/_expo/static/js/web/"),
  );
  const selectedScriptUrls = expoScriptUrls.length > 0 ? expoScriptUrls : scriptUrls;
  if (selectedScriptUrls.length === 0) {
    throw new Error(`No JavaScript bundle script found at ${pageUrl}`);
  }

  const bundleSources = await Promise.all(selectedScriptUrls.map(fetchText));
  return {
    label: pageUrl,
    source: bundleSources.join("\n"),
  };
}

function extractPublicEnvValue(source, key) {
  const match = source.match(new RegExp(`${key}:"([^"]*)"`, "u"));
  return match?.[1] ?? null;
}

function validateBundle({ label, source, target }) {
  const contract = expectedContract(target);

  for (const [key, expected] of Object.entries(contract.required)) {
    const actual = extractPublicEnvValue(source, key);
    if (actual !== expected) {
      throw new Error(
        `${label}: ${key} mismatch. Expected ${expected || "<empty>"}, got ${actual || "<missing>"}`,
      );
    }
  }

  for (const forbidden of contract.forbidden) {
    if (source.includes(forbidden)) {
      throw new Error(`${label}: forbidden public env marker found: ${forbidden}`);
    }
  }

  console.log(
    `[web-public-env] ${target} OK for ${label}: ${Object.values(contract.required).join(", ")}`,
  );
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  let lastError = null;

  for (let attempt = 1; attempt <= args.retries; attempt += 1) {
    try {
      const bundle = args.url
        ? await readRemoteSource(args.url)
        : readDistSource(args.dist);
      validateBundle({ ...bundle, target: args.target });
      return;
    } catch (error) {
      lastError = error;
      if (attempt >= args.retries) {
        break;
      }
      console.warn(
        `[web-public-env] attempt ${attempt}/${args.retries} failed: ${error.message}`,
      );
      await sleep(args.retryDelayMs);
    }
  }

  throw lastError;
}

main().catch((error) => {
  console.error(`[web-public-env] ${error.message}`);
  process.exit(1);
});
