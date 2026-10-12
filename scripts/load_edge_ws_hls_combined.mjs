#!/usr/bin/env node

import { spawn } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(scriptDir, "..");
const edgeDir = resolve(repoRoot, "filfinance-edge");

loadEnvFile(process.env.FILFINANCE_ENV_FILE || resolve(repoRoot, ".env.local"));

const config = {
  apiBaseUrl: envString("FILFINANCE_LOAD_TEST_API_BASE_URL", "https://api.filfinance.fr"),
  tiers: envString("FILFINANCE_COMBINED_LOAD_TEST_TIERS", "500"),
  durationMs: envInt("FILFINANCE_COMBINED_LOAD_TEST_DURATION_MS", 60_000),
  edgePublishIntervalMs: envInt("FILFINANCE_EDGE_LOAD_TEST_PUBLISH_INTERVAL_MS", 500),
  useManualEdgeTicket: envBool("FILFINANCE_EDGE_LOAD_TEST_USE_MANUAL_TICKET", false),
};

if (config.tiers.split(",").filter(Boolean).length !== 1) {
  console.error(
    JSON.stringify({
      stage: "combined_load_test_config_error",
      error: "combined load test expects exactly one tier per run",
      value: config.tiers,
    }),
  );
  process.exit(1);
}

const edgeMessages = envInt(
  "FILFINANCE_EDGE_LOAD_TEST_MESSAGES",
  Math.max(12, Math.ceil(config.durationMs / config.edgePublishIntervalMs)),
);

const missingEdgeEnv = [
  "FILFINANCE_EDGE_WS_URL",
  "FILFINANCE_EDGE_PUBLISH_URL",
  "FILFINANCE_EDGE_PUBLISH_SECRET",
].filter((key) => !process.env[key]);

if (missingEdgeEnv.length > 0) {
  console.error(
    JSON.stringify({
      stage: "combined_load_test_config_error",
      error: "missing required Edge load test environment variables",
      missing: missingEdgeEnv,
    }),
  );
  process.exit(1);
}

const configuredManualEdgeTicket = process.env.FILFINANCE_EDGE_LOAD_TEST_TICKET?.trim() || "";
if (configuredManualEdgeTicket && !config.useManualEdgeTicket) {
  console.log(
    JSON.stringify({
      stage: "edge_ticket_env_ignored",
      reason: "manual edge tickets are ignored unless FILFINANCE_EDGE_LOAD_TEST_USE_MANUAL_TICKET=true",
    }),
  );
}

const manualEdgeTicket = config.useManualEdgeTicket ? configuredManualEdgeTicket : "";
const accessToken =
  process.env.FILFINANCE_LOAD_TEST_ACCESS_TOKEN?.trim() ||
  (manualEdgeTicket ? "" : await resolveAccessToken());
const edgeTicket = manualEdgeTicket || (await fetchEdgeLiveTicket(accessToken));
const edgeTicketSource = manualEdgeTicket ? "manual" : "api";
const startedAt = new Date().toISOString();

console.log(
  JSON.stringify({
    stage: "combined_load_test_start",
    started_at: startedAt,
    tiers: config.tiers,
    duration_ms: config.durationMs,
    edge_messages: edgeMessages,
    edge_publish_interval_ms: config.edgePublishIntervalMs,
    edge_ticket_source: edgeTicketSource,
  }),
);

const hlsEnv = {
  ...process.env,
  ...(accessToken ? { FILFINANCE_LOAD_TEST_ACCESS_TOKEN: accessToken } : {}),
  FILFINANCE_LOAD_TEST_TIERS:
    process.env.FILFINANCE_LOAD_TEST_TIERS ?? config.tiers,
  FILFINANCE_LOAD_TEST_DURATION_MS:
    process.env.FILFINANCE_LOAD_TEST_DURATION_MS ?? String(config.durationMs),
  FILFINANCE_LOAD_TEST_SKIP_WS: process.env.FILFINANCE_LOAD_TEST_SKIP_WS ?? "true",
};

const edgeEnv = {
  ...process.env,
  FILFINANCE_EDGE_LOAD_TEST_TICKET: edgeTicket,
  ...(accessToken
    ? {
        FILFINANCE_EDGE_LOAD_TEST_ACCESS_TOKEN: accessToken,
        FILFINANCE_EDGE_LOAD_TEST_TICKET_URL: new URL(
          "/api/edge/live-ticket",
          `${config.apiBaseUrl}/`,
        ).toString(),
      }
    : {}),
  FILFINANCE_EDGE_LOAD_TEST_TIERS:
    process.env.FILFINANCE_EDGE_LOAD_TEST_TIERS ?? config.tiers,
  FILFINANCE_EDGE_LOAD_TEST_MESSAGES: String(edgeMessages),
  FILFINANCE_EDGE_LOAD_TEST_PUBLISH_INTERVAL_MS: String(config.edgePublishIntervalMs),
  FILFINANCE_EDGE_LOAD_TEST_TIMEOUT_MS:
    process.env.FILFINANCE_EDGE_LOAD_TEST_TIMEOUT_MS ??
    String(config.durationMs + 30_000),
};

const [hlsResult, edgeResult] = await Promise.all([
  runChild("hls", repoRoot, ["scripts/load_backend_ws_hls.mjs"], hlsEnv),
  runChild("edge", edgeDir, ["scripts/load-test-live.js"], edgeEnv),
]);

const report = {
  started_at: startedAt,
  finished_at: new Date().toISOString(),
  ok: hlsResult.code === 0 && edgeResult.code === 0,
  children: {
    hls: hlsResult,
    edge: edgeResult,
  },
};

console.log(JSON.stringify({ stage: "combined_load_test_done", report }, null, 2));

if (!report.ok) {
  process.exitCode = 1;
}

function runChild(name, cwd, args, env) {
  const startedAtMs = Date.now();
  const child = spawn(process.execPath, args, {
    cwd,
    env,
    stdio: ["ignore", "pipe", "pipe"],
  });

  pipeLines(name, "stdout", child.stdout);
  pipeLines(name, "stderr", child.stderr);

  return new Promise((resolve) => {
    child.on("close", (code, signal) => {
      resolve({
        code,
        signal,
        duration_ms: Date.now() - startedAtMs,
      });
    });
  });
}

function pipeLines(childName, streamName, stream) {
  let buffer = "";
  stream.setEncoding("utf8");

  stream.on("data", (chunk) => {
    buffer += chunk;
    let newlineIndex = buffer.indexOf("\n");
    while (newlineIndex >= 0) {
      emitChildLine(childName, streamName, buffer.slice(0, newlineIndex));
      buffer = buffer.slice(newlineIndex + 1);
      newlineIndex = buffer.indexOf("\n");
    }
  });

  stream.on("end", () => {
    if (buffer.length > 0) {
      emitChildLine(childName, streamName, buffer);
      buffer = "";
    }
  });
}

function emitChildLine(childName, streamName, line) {
  const trimmed = line.trimEnd();
  if (!trimmed) {
    return;
  }

  let payload = trimmed;
  try {
    payload = JSON.parse(trimmed);
  } catch {
    // Keep non-JSON output readable while preserving a machine-readable envelope.
  }

  console.log(
    JSON.stringify({
      stage: "combined_child_output",
      child: childName,
      stream: streamName,
      payload,
    }),
  );
}

async function fetchEdgeLiveTicket(token) {
  if (!token) {
    throw new Error("edge ticket auto-fetch requires a Better Auth JWT");
  }

  const ticketUrl = new URL("/api/edge/live-ticket", `${config.apiBaseUrl}/`).toString();
  let response;
  try {
    response = await fetch(ticketUrl, {
      headers: {
        Authorization: `Bearer ${token}`,
      },
    });
  } catch (error) {
    throw new Error(`edge live ticket endpoint probe failed: ${errorMessage(error)}`);
  }

  const payload = await responseJsonOrNull(response);
  console.log(
    JSON.stringify({
      stage: "edge_ticket_probe",
      status: response.status,
      ttl_secs: payload?.ttl_secs ?? null,
      expires_at: payload?.expires_at ?? null,
      has_ticket: Boolean(payload?.ticket),
    }),
  );

  if (response.status === 401 || response.status === 403) {
    throw new Error(
      [
        `edge live ticket endpoint rejected the load-test bearer with HTTP ${response.status}.`,
        "Unset stale FILFINANCE_LOAD_TEST_ACCESS_TOKEN, or provide a fresh Better Auth JWT.",
      ].join(" "),
    );
  }

  if (!response.ok) {
    throw new Error(`edge live ticket endpoint probe failed with HTTP ${response.status}`);
  }

  if (!payload?.ticket) {
    throw new Error("edge live ticket endpoint did not return a ticket");
  }

  return payload.ticket;
}

async function resolveAccessToken() {
  const directToken = process.env.FILFINANCE_LOAD_TEST_ACCESS_TOKEN?.trim();
  if (directToken) {
    return directToken;
  }

  throw new Error(
    [
      "Combined load test requires auth to fetch a fresh edge ticket.",
      "Set FILFINANCE_EDGE_LOAD_TEST_TICKET manually, or set",
      "FILFINANCE_LOAD_TEST_ACCESS_TOKEN to a Better Auth JWT.",
    ].join(" "),
  );
}

async function responseJsonOrNull(response) {
  try {
    return await response.json();
  } catch {
    return null;
  }
}

function errorMessage(error) {
  return error instanceof Error ? error.message : String(error);
}

function envString(name, fallback) {
  const value = process.env[name];
  return value === undefined || value === "" ? fallback : value;
}

function envInt(name, fallback) {
  const value = process.env[name];
  if (value === undefined || value === "") {
    return fallback;
  }

  const parsed = Number.parseInt(value, 10);
  return Number.isFinite(parsed) && parsed > 0 ? parsed : fallback;
}

function envBool(name, fallback) {
  const value = process.env[name]?.trim().toLowerCase();
  if (!value) {
    return fallback;
  }
  return ["1", "true", "yes", "on"].includes(value);
}

function loadEnvFile(path) {
  if (!path || !existsSync(path)) {
    return;
  }

  const text = readFileSync(path, "utf8");
  for (const rawLine of text.split(/\r?\n/u)) {
    const line = rawLine.trim();
    if (!line || line.startsWith("#")) {
      continue;
    }
    const match = /^([A-Za-z_][A-Za-z0-9_]*)=(.*)$/u.exec(line);
    if (!match || process.env[match[1]] !== undefined) {
      continue;
    }
    process.env[match[1]] = unquoteEnvValue(match[2]);
  }
}

function unquoteEnvValue(value) {
  const trimmed = value.trim();
  if (
    (trimmed.startsWith('"') && trimmed.endsWith('"')) ||
    (trimmed.startsWith("'") && trimmed.endsWith("'"))
  ) {
    return trimmed.slice(1, -1);
  }
  return trimmed;
}
