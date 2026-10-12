#!/usr/bin/env node

import { existsSync, readFileSync } from "node:fs";
import { performance } from "node:perf_hooks";

const DEFAULT_WS_URL = "wss://api.filfinance.fr/ws/news";
const DEFAULT_TIERS = "50,100,200,500";

loadEnvFile(process.env.FILFINANCE_ENV_FILE || ".env.local");

const config = {
  wsUrl: envString("FILFINANCE_LOAD_TEST_WS_URL", DEFAULT_WS_URL),
  apiBaseUrl: envString("FILFINANCE_LOAD_TEST_API_BASE_URL", ""),
  hlsBaseUrl: envString("FILFINANCE_LOAD_TEST_HLS_BASE_URL", ""),
  tiers: envIntList("FILFINANCE_LOAD_TEST_TIERS", DEFAULT_TIERS),
  durationMs: envInt("FILFINANCE_LOAD_TEST_DURATION_MS", 30_000),
  connectBatchSize: envInt("FILFINANCE_LOAD_TEST_CONNECT_BATCH_SIZE", 10),
  connectBatchDelayMs: envInt("FILFINANCE_LOAD_TEST_CONNECT_BATCH_DELAY_MS", 100),
  connectTimeoutMs: envInt("FILFINANCE_LOAD_TEST_CONNECT_TIMEOUT_MS", 10_000),
  hlsPollIntervalMs: envInt("FILFINANCE_LOAD_TEST_HLS_POLL_INTERVAL_MS", 4_000),
  hlsMaxSegmentsPerPoll: envInt("FILFINANCE_LOAD_TEST_HLS_MAX_SEGMENTS_PER_POLL", 3),
  hlsClipPlaylistUrl: envString("FILFINANCE_LOAD_TEST_HLS_CLIP_PLAYLIST_URL", ""),
  httpRetryAttempts: envInt("FILFINANCE_LOAD_TEST_HTTP_RETRY_ATTEMPTS", 2),
  httpRetryBackoffMs: envInt("FILFINANCE_LOAD_TEST_HTTP_RETRY_BACKOFF_MS", 250),
  skipHls: envBool("FILFINANCE_LOAD_TEST_SKIP_HLS", false),
  skipWs: envBool("FILFINANCE_LOAD_TEST_SKIP_WS", false),
};

config.apiBaseUrl ||= deriveApiBaseUrl(config.wsUrl);
config.hlsBaseUrl = normalizeOptionalBaseUrl(config.hlsBaseUrl);
assertAllowedHlsLoadTestTarget(config.hlsBaseUrl || config.apiBaseUrl);
config.replayPlaylistUrl = resolveHlsTargetUrl("/audio/replay/today/playlist.m3u8");
config.hlsTicketUrl = new URL("/api/hls-ticket", `${config.apiBaseUrl}/`).toString();
config.hlsTargets = buildHlsTargets();

if (envBool("FILFINANCE_LOAD_TEST_VALIDATE_TARGET_ONLY", false)) {
  console.log(
    JSON.stringify({
      stage: "hls_load_test_target_allowed",
      hls_base_url: config.hlsBaseUrl || config.apiBaseUrl,
      replay_playlist_url: config.replayPlaylistUrl,
    }),
  );
  process.exit(0);
}

async function main() {
  const accessToken = config.skipWs && config.skipHls ? null : await resolveAccessToken();
  const runStartedAt = new Date().toISOString();
  const reports = [];

  console.log(
    JSON.stringify({
      stage: "load_test_start",
      started_at: runStartedAt,
      ws_url: redactUrl(config.wsUrl),
      api_base_url: config.apiBaseUrl,
      hls_base_url: config.hlsBaseUrl || config.apiBaseUrl,
      tiers: config.tiers,
      duration_ms: config.durationMs,
      hls_targets: config.skipHls
        ? []
        : config.hlsTargets.map((target) => ({
            name: target.name,
            playlist_url: target.url,
          })),
    }),
  );

  await assertBackendReachable(config.apiBaseUrl);
  await assertHlsTicketReachable(accessToken);

  for (const clients of config.tiers) {
    const report = await runTier(clients, accessToken);
    reports.push(report);
    console.log(JSON.stringify({ stage: "tier_report", report }));
  }

  const finalReport = {
    started_at: runStartedAt,
    finished_at: new Date().toISOString(),
    tiers: reports,
    ok: reports.every((report) => report.ok),
  };

  console.log(JSON.stringify({ stage: "load_test_done", report: finalReport }, null, 2));

  if (!finalReport.ok) {
    process.exitCode = 1;
  }
}

async function runTier(clientCount, token) {
  const metrics = createMetrics(clientCount);
  const clients = Array.from(
    { length: clientCount },
    (_, index) => new LoadClient(index, token, metrics),
  );
  const hlsLoops = [];
  const tierStartedAt = new Date().toISOString();

  for (let index = 0; index < clients.length; index += config.connectBatchSize) {
    const batch = clients.slice(index, index + config.connectBatchSize);
    await Promise.all(batch.map((client) => client.connect()));
    if (!config.skipHls) {
      hlsLoops.push(...batch.map((client) => client.startHlsLoop()));
    }
    await sleep(config.connectBatchDelayMs);
  }

  await sleep(config.durationMs);

  for (const client of clients) {
    client.stop();
  }
  await Promise.allSettled(hlsLoops);

  const report = buildTierReport(tierStartedAt, metrics);
  return report;
}

class LoadClient {
  constructor(index, token, metrics) {
    this.index = index;
    this.token = token;
    this.metrics = metrics;
    this.seenSegmentsByTarget = new Map();
    this.socket = null;
    this.stopped = false;
    this.authenticated = false;
    this.hlsAccessTicket = null;
  }

  connect() {
    if (config.skipWs) {
      return Promise.resolve();
    }

    const startedAt = performance.now();
    return new Promise((resolve) => {
      const socket = new WebSocket(config.wsUrl);
      this.socket = socket;
      let settled = false;
      let opened = false;

      const settle = (result) => {
        if (settled) {
          return;
        }
        settled = true;
        clearTimeout(timeout);
        resolve(result);
      };

      const timeout = setTimeout(() => {
        this.metrics.ws.connectionTimeouts += 1;
        socket.close();
        settle(null);
      }, config.connectTimeoutMs);

      socket.addEventListener("open", () => {
        opened = true;
        this.metrics.ws.opened += 1;
        this.metrics.ws.connectLatenciesMs.push(performance.now() - startedAt);
        socket.send(
          JSON.stringify({
            type: "auth",
            access_token: this.token,
          }),
        );
      });

      socket.addEventListener("message", (event) => {
        const message = parseJson(event.data);
        if (!message) {
          this.metrics.ws.invalidMessages += 1;
          return;
        }

        const messageType = String(message.type ?? "unknown");
        this.metrics.ws.messageTypes.set(
          messageType,
          (this.metrics.ws.messageTypes.get(messageType) ?? 0) + 1,
        );

        if (!this.authenticated && messageType === "hello") {
          this.authenticated = true;
          this.metrics.ws.authenticated += 1;
          this.metrics.ws.helloLatenciesMs.push(performance.now() - startedAt);
          settle(socket);
        }
      });

      socket.addEventListener("close", (event) => {
        if (!this.authenticated) {
          this.metrics.ws.authFailures += 1;
          this.metrics.ws.closeCodes.set(
            event.code,
            (this.metrics.ws.closeCodes.get(event.code) ?? 0) + 1,
          );
          settle(null);
          return;
        }

        if (!this.stopped) {
          this.metrics.ws.unexpectedCloses += 1;
          this.metrics.ws.closeCodes.set(
            event.code,
            (this.metrics.ws.closeCodes.get(event.code) ?? 0) + 1,
          );
        }
      });

      socket.addEventListener("error", () => {
        if (!opened || !this.authenticated) {
          this.metrics.ws.connectionErrors += 1;
          settle(null);
        } else {
          this.metrics.ws.socketErrors += 1;
        }
      });
    });
  }

  async startHlsLoop() {
    while (!this.stopped) {
      await this.pollHlsOnce();
      await sleep(config.hlsPollIntervalMs);
    }
  }

  async pollHlsOnce() {
    for (const target of config.hlsTargets) {
      await this.pollHlsTargetOnce(target);
    }
  }

  async pollHlsTargetOnce(target) {
    const targetMetrics = this.metrics.hls.targets.get(target.name);
    const playlistUrl = await this.resolveHlsPlaylistUrl(target);
    if (!playlistUrl) {
      recordHttpSkippedNoTicket(this.metrics.hls.playlist, targetMetrics.playlist);
      return;
    }

    const playlist = await timedFetchText(
      playlistUrl,
      this.metrics.hls.playlist,
      targetMetrics.playlist,
    );
    if (!playlist.ok) {
      return;
    }

    const segmentUrls = parseHlsSegmentUrls(playlist.text, playlistUrl);
    const seenSegments = this.seenSegmentsForTarget(target.name);
    const unseenSegments = segmentUrls
      .filter((url) => !seenSegments.has(url))
      .slice(-config.hlsMaxSegmentsPerPoll);

    for (const segmentUrl of unseenSegments) {
      seenSegments.add(segmentUrl);
      await timedFetchBytes(segmentUrl, this.metrics.hls.segment, targetMetrics.segment);
    }
  }

  seenSegmentsForTarget(targetName) {
    if (!this.seenSegmentsByTarget.has(targetName)) {
      this.seenSegmentsByTarget.set(targetName, new Set());
    }
    return this.seenSegmentsByTarget.get(targetName);
  }

  stop() {
    this.stopped = true;
    if (this.socket && this.socket.readyState <= WebSocket.OPEN) {
      this.socket.close();
    }
  }

  async resolveHlsPlaylistUrl(target) {
    const ticket = await this.resolveHlsTicket();
    return ticket ? appendQueryParam(target.url, "hls_ticket", ticket) : null;
  }

  async resolveHlsTicket() {
    if (!this.token) {
      return null;
    }

    if (this.hlsAccessTicket && this.hlsAccessTicket.expiresAtMs - Date.now() > 60_000) {
      return this.hlsAccessTicket.ticket;
    }

    this.metrics.hls.ticket.requests += 1;
    const startedAt = performance.now();
    let response;
    try {
      response = await fetch(config.hlsTicketUrl, {
        headers: {
          Authorization: `Bearer ${this.token}`,
        },
      });
      this.metrics.hls.ticket.latenciesMs.push(performance.now() - startedAt);
    } catch {
      this.metrics.hls.ticket.failures += 1;
      this.metrics.hls.ticket.latenciesMs.push(performance.now() - startedAt);
      return null;
    }

    this.metrics.hls.ticket.statusCodes.set(
      response.status,
      (this.metrics.hls.ticket.statusCodes.get(response.status) ?? 0) + 1,
    );
    if (!response.ok) {
      this.metrics.hls.ticket.failures += 1;
      return null;
    }

    const payload = await response.json();
    if (!payload.enabled || !payload.ticket) {
      this.hlsAccessTicket = null;
      return null;
    }

    this.hlsAccessTicket = {
      ticket: payload.ticket,
      expiresAtMs: parseTicketExpiration(payload),
    };
    return payload.ticket;
  }
}

async function timedFetchText(url, ...metricGroups) {
  const startedAt = performance.now();
  recordHttpRequest(metricGroups);
  for (let attempt = 0; attempt <= config.httpRetryAttempts; attempt += 1) {
    try {
      const response = await fetch(url);
      recordHttpResponse(metricGroups, response, performance.now() - startedAt);
      if (!response.ok) {
        recordHttpFailure(metricGroups);
        return { ok: false, text: "" };
      }
      const text = await response.text();
      if (attempt > 0) {
        recordHttpRetriedSuccess(metricGroups);
      }
      return { ok: true, text };
    } catch {
      if (attempt >= config.httpRetryAttempts) {
        recordHttpFailure(metricGroups);
        recordHttpLatency(metricGroups, performance.now() - startedAt);
        return { ok: false, text: "" };
      }

      recordHttpRetry(metricGroups);
      await sleep(config.httpRetryBackoffMs * (attempt + 1));
    }
  }

  return { ok: false, text: "" };
}

async function timedFetchBytes(url, ...metricGroups) {
  const startedAt = performance.now();
  recordHttpRequest(metricGroups);
  for (let attempt = 0; attempt <= config.httpRetryAttempts; attempt += 1) {
    try {
      const response = await fetch(url);
      recordHttpResponse(metricGroups, response, performance.now() - startedAt);
      if (!response.ok) {
        recordHttpFailure(metricGroups);
        return;
      }
      const bytes = await response.arrayBuffer();
      recordHttpBytes(metricGroups, bytes.byteLength);
      if (attempt > 0) {
        recordHttpRetriedSuccess(metricGroups);
      }
      return;
    } catch {
      if (attempt >= config.httpRetryAttempts) {
        recordHttpFailure(metricGroups);
        recordHttpLatency(metricGroups, performance.now() - startedAt);
        return;
      }

      recordHttpRetry(metricGroups);
      await sleep(config.httpRetryBackoffMs * (attempt + 1));
    }
  }
}

async function assertBackendReachable(apiBaseUrl) {
  const healthUrl = new URL("/health", `${apiBaseUrl}/`).toString();
  const response = await fetch(healthUrl);
  if (!response.ok) {
    throw new Error(`backend health failed with HTTP ${response.status}`);
  }
}

async function assertHlsTicketReachable(token) {
  if (config.skipHls || !token) {
    return;
  }

  let response;
  try {
    response = await fetch(config.hlsTicketUrl, {
      headers: {
        Authorization: `Bearer ${token}`,
      },
    });
  } catch (error) {
    throw new Error(`HLS ticket endpoint probe failed: ${errorMessage(error)}`);
  }

  const payload = await responseJsonOrNull(response);
  console.log(
    JSON.stringify({
      stage: "hls_ticket_probe",
      status: response.status,
      enabled: Boolean(payload?.enabled),
      has_ticket: Boolean(payload?.ticket),
    }),
  );

  if (response.status === 401 || response.status === 403) {
    throw new Error(
      [
        `HLS ticket endpoint rejected the load-test bearer with HTTP ${response.status}.`,
        "Unset FILFINANCE_LOAD_TEST_ACCESS_TOKEN if it is stale, or provide a fresh token.",
        "Use a short-lived JWT from Better Auth.",
      ].join(" "),
    );
  }

  if (!response.ok) {
    throw new Error(`HLS ticket endpoint probe failed with HTTP ${response.status}`);
  }

  if (payload?.enabled && !payload.ticket) {
    throw new Error("HLS ticket endpoint is enabled but did not return a ticket");
  }
}

async function resolveAccessToken() {
  const directToken = process.env.FILFINANCE_LOAD_TEST_ACCESS_TOKEN?.trim();
  if (directToken) {
    return directToken;
  }

  throw new Error(
    [
      "WebSocket load test requires auth.",
      "Set FILFINANCE_LOAD_TEST_ACCESS_TOKEN to a Better Auth JWT.",
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

function buildTierReport(startedAt, metrics) {
  const wsFailures =
    metrics.ws.authFailures +
    metrics.ws.connectionErrors +
    metrics.ws.connectionTimeouts;
  const hlsFailures =
    metrics.hls.ticket.failures +
    metrics.hls.playlist.failures +
    metrics.hls.segment.failures;

  return {
    started_at: startedAt,
    finished_at: new Date().toISOString(),
    clients_requested: metrics.clientsRequested,
    ok:
      (config.skipWs || metrics.ws.authenticated === metrics.clientsRequested) &&
      (config.skipHls || hlsFailures === 0),
    ws: {
      requested: metrics.clientsRequested,
      opened: metrics.ws.opened,
      authenticated: metrics.ws.authenticated,
      failures: wsFailures,
      auth_failures: metrics.ws.authFailures,
      connection_errors: metrics.ws.connectionErrors,
      connection_timeouts: metrics.ws.connectionTimeouts,
      unexpected_closes: metrics.ws.unexpectedCloses,
      invalid_messages: metrics.ws.invalidMessages,
      message_types: objectFromMap(metrics.ws.messageTypes),
      close_codes: objectFromMap(metrics.ws.closeCodes),
      connect_latency_ms: latencyReport(metrics.ws.connectLatenciesMs),
      hello_latency_ms: latencyReport(metrics.ws.helloLatenciesMs),
    },
    hls: config.skipHls
      ? null
      : {
          playlist: httpMetricReport(metrics.hls.playlist),
          segment: httpMetricReport(metrics.hls.segment),
          ticket: httpMetricReport(metrics.hls.ticket),
          total_bytes: metrics.hls.segment.bytes,
          targets: hlsTargetReports(metrics.hls.targets),
        },
  };
}

function createMetrics(clientsRequested) {
  const targets = new Map(
    config.hlsTargets.map((target) => [
      target.name,
      {
        playlist: createHttpMetrics(),
        segment: createHttpMetrics(),
      },
    ]),
  );

  return {
    clientsRequested,
    ws: {
      opened: 0,
      authenticated: 0,
      authFailures: 0,
      connectionErrors: 0,
      connectionTimeouts: 0,
      socketErrors: 0,
      unexpectedCloses: 0,
      invalidMessages: 0,
      closeCodes: new Map(),
      messageTypes: new Map(),
      connectLatenciesMs: [],
      helloLatenciesMs: [],
    },
    hls: {
      ticket: createHttpMetrics(),
      playlist: createHttpMetrics(),
      segment: createHttpMetrics(),
      targets,
    },
  };
}

function createHttpMetrics() {
  return {
    requests: 0,
    failures: 0,
    skippedNoTicket: 0,
    retries: 0,
    retriedSuccesses: 0,
    bytes: 0,
    statusCodes: new Map(),
    cfCacheStatus: new Map(),
    hlsProxyCacheStatus: new Map(),
    latenciesMs: [],
  };
}

function recordHttpRequest(metricGroups) {
  for (const metrics of metricGroups) {
    metrics.requests += 1;
  }
}

function recordHttpResponse(metricGroups, response, latencyMs) {
  for (const metrics of metricGroups) {
    metrics.latenciesMs.push(latencyMs);
    metrics.statusCodes.set(
      response.status,
      (metrics.statusCodes.get(response.status) ?? 0) + 1,
    );
    const cacheStatus = response.headers.get("cf-cache-status") ?? "missing";
    metrics.cfCacheStatus.set(
      cacheStatus,
      (metrics.cfCacheStatus.get(cacheStatus) ?? 0) + 1,
    );
    const hlsProxyCacheStatus =
      response.headers.get("x-filfinance-hls-proxy-cache") ?? "missing";
    metrics.hlsProxyCacheStatus.set(
      hlsProxyCacheStatus,
      (metrics.hlsProxyCacheStatus.get(hlsProxyCacheStatus) ?? 0) + 1,
    );
  }
}

function recordHttpLatency(metricGroups, latencyMs) {
  for (const metrics of metricGroups) {
    metrics.latenciesMs.push(latencyMs);
  }
}

function recordHttpFailure(metricGroups) {
  for (const metrics of metricGroups) {
    metrics.failures += 1;
  }
}

function recordHttpRetry(metricGroups) {
  for (const metrics of metricGroups) {
    metrics.retries += 1;
  }
}

function recordHttpRetriedSuccess(metricGroups) {
  for (const metrics of metricGroups) {
    metrics.retriedSuccesses += 1;
  }
}

function recordHttpSkippedNoTicket(...metricGroups) {
  for (const metrics of metricGroups) {
    metrics.skippedNoTicket += 1;
  }
}

function recordHttpBytes(metricGroups, bytes) {
  for (const metrics of metricGroups) {
    metrics.bytes += bytes;
  }
}

function httpMetricReport(metrics) {
  return {
    requests: metrics.requests,
    failures: metrics.failures,
    skipped_no_ticket: metrics.skippedNoTicket,
    retry_attempts: metrics.retries,
    retried_successes: metrics.retriedSuccesses,
    status_codes: objectFromMap(metrics.statusCodes),
    cf_cache_status: objectFromMap(metrics.cfCacheStatus),
    hls_proxy_cache_status: objectFromMap(metrics.hlsProxyCacheStatus),
    latency_ms: latencyReport(metrics.latenciesMs),
  };
}

function hlsTargetReports(targets) {
  return Object.fromEntries(
    [...targets.entries()]
      .sort(([left], [right]) => left.localeCompare(right))
      .map(([name, metrics]) => [
        name,
        {
          playlist: httpMetricReport(metrics.playlist),
          segment: httpMetricReport(metrics.segment),
          total_bytes: metrics.segment.bytes,
        },
      ]),
  );
}

function latencyReport(values) {
  return {
    p50: percentile(values, 0.5),
    p95: percentile(values, 0.95),
    p99: percentile(values, 0.99),
    max: values.length ? round(Math.max(...values)) : null,
  };
}

function parseHlsSegmentUrls(playlistText, playlistUrl) {
  return String(playlistText)
    .split(/\r?\n/u)
    .map((line) => line.trim())
    .filter((line) => line && !line.startsWith("#"))
    .map((line) => new URL(line, playlistUrl).toString());
}

function buildHlsTargets() {
  const targets = [
    {
      name: "replay",
      url: config.replayPlaylistUrl,
    },
  ];

  if (config.hlsClipPlaylistUrl) {
    targets.push({
      name: "clip",
      url: resolveHlsTargetUrl(config.hlsClipPlaylistUrl),
    });
  }

  return targets;
}

function resolveHlsTargetUrl(resourceUrl) {
  if (!config.hlsBaseUrl) {
    return new URL(resourceUrl, `${config.apiBaseUrl}/`).toString();
  }

  const parsedResource = new URL(resourceUrl, `${config.apiBaseUrl}/`);

  return new URL(
    `${parsedResource.pathname}${parsedResource.search}${parsedResource.hash}`,
    `${config.hlsBaseUrl}/`,
  ).toString();
}

function appendQueryParam(url, name, value) {
  const parsed = new URL(url);
  parsed.searchParams.set(name, value);
  return parsed.toString();
}

function parseTicketExpiration(payload) {
  const expiresAtMs = payload.expires_at ? Date.parse(payload.expires_at) : Number.NaN;
  if (Number.isFinite(expiresAtMs)) {
    return expiresAtMs;
  }

  const ttlMs = Number.isFinite(payload.ttl_secs) ? payload.ttl_secs * 1000 : 60_000;
  return Date.now() + Math.max(ttlMs, 60_000);
}

function parseJson(raw) {
  if (typeof raw !== "string") {
    return null;
  }
  try {
    return JSON.parse(raw);
  } catch {
    return null;
  }
}

function deriveApiBaseUrl(wsUrl) {
  const parsed = new URL(wsUrl);
  parsed.protocol = parsed.protocol === "wss:" ? "https:" : "http:";
  parsed.pathname = "";
  parsed.search = "";
  parsed.hash = "";
  return parsed.toString().replace(/\/$/u, "");
}

function normalizeOptionalBaseUrl(value) {
  const trimmed = String(value || "").trim();
  if (!trimmed) {
    return "";
  }

  const parsed = new URL(trimmed);
  parsed.pathname = parsed.pathname.replace(/\/+$/u, "");
  parsed.search = "";
  parsed.hash = "";
  return parsed.toString().replace(/\/$/u, "");
}

function assertAllowedHlsLoadTestTarget(baseUrl) {
  if (config.skipHls) {
    return;
  }

  const parsed = new URL(baseUrl);
  const hostname = parsed.hostname.toLowerCase();
  const allowedMediaHosts = csvSet("FILFINANCE_MEDIA_LOAD_TEST_ALLOWED_HOSTS");
  if (isBunnyCdnHost(hostname) || allowedMediaHosts.has(hostname)) {
    return;
  }

  const isFilfinanceHost = hostname === "filfinance.fr" || hostname.endsWith(".filfinance.fr");
  const isLiveFilfinanceHost =
    hostname === "live.filfinance.fr" ||
    (hostname.startsWith("live-") && hostname.endsWith(".filfinance.fr"));
  const blockedSuffixes = ["workers.dev", "cloudflarestream.com", "videodelivery.net"];
  const matchedPattern = isFilfinanceHost
    ? "filfinance.fr"
    : blockedSuffixes.find((pattern) => hostname === pattern || hostname.endsWith(`.${pattern}`));
  if (!matchedPattern) {
    return;
  }

  if (
    envBool("FILFINANCE_ALLOW_CLOUDFLARE_MEDIA_LOAD_TEST", false) &&
    (!isFilfinanceHost || isLiveFilfinanceHost)
  ) {
    return;
  }

  console.error(
    JSON.stringify({
      stage: "hls_load_test_target_blocked",
      error: "Cloudflare media load tests are disabled by default",
      matched_pattern: matchedPattern,
      hls_base_url: redactUrl(baseUrl),
      allowed_hosts_env: "FILFINANCE_MEDIA_LOAD_TEST_ALLOWED_HOSTS",
      cloudflare_override_env: "FILFINANCE_ALLOW_CLOUDFLARE_MEDIA_LOAD_TEST",
    }),
  );
  process.exit(1);
}

function isBunnyCdnHost(hostname) {
  return hostname === "b-cdn.net" || hostname.endsWith(".b-cdn.net");
}

function csvSet(name) {
  return new Set(
    String(process.env[name] || "")
      .split(",")
      .map((value) => value.trim().toLowerCase())
      .filter(Boolean),
  );
}

function redactUrl(value) {
  const parsed = new URL(value);
  parsed.username = "";
  parsed.password = "";
  parsed.search = "";
  return parsed.toString();
}

function objectFromMap(map) {
  return Object.fromEntries(
    [...map.entries()].sort(([left], [right]) => String(left).localeCompare(String(right))),
  );
}

function percentile(values, rank) {
  if (!values.length) {
    return null;
  }
  const sorted = [...values].sort((left, right) => left - right);
  const index = Math.min(sorted.length - 1, Math.ceil(sorted.length * rank) - 1);
  return round(sorted[index]);
}

function round(value) {
  return Number(value.toFixed(2));
}

function envString(name, defaultValue) {
  return process.env[name]?.trim() || defaultValue;
}

function envBool(name, defaultValue) {
  const value = process.env[name]?.trim().toLowerCase();
  if (!value) {
    return defaultValue;
  }
  return ["1", "true", "yes", "on"].includes(value);
}

function envInt(name, defaultValue) {
  const value = process.env[name];
  if (!value) {
    return defaultValue;
  }
  const parsed = Number.parseInt(value, 10);
  if (!Number.isFinite(parsed) || parsed <= 0) {
    throw new Error(`${name} must be a positive integer`);
  }
  return parsed;
}

function envIntList(name, defaultValue) {
  const raw = envString(name, defaultValue);
  const values = raw
    .split(",")
    .map((value) => Number.parseInt(value.trim(), 10))
    .filter((value) => Number.isFinite(value) && value > 0);

  if (!values.length) {
    throw new Error(`${name} must contain at least one positive integer`);
  }
  return values;
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

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

await main();
