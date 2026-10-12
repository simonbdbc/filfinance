import { buildPublishSignature } from "../src/protocol.js";

const DEFAULT_CLIENTS = 1000;
const DEFAULT_MESSAGES = 12;
const DEFAULT_CONNECT_BATCH_SIZE = 50;
const DEFAULT_CONNECT_DELAY_MS = 50;
const DEFAULT_PUBLISH_INTERVAL_MS = 250;
const DEFAULT_TIMEOUT_MS = 30_000;

const config = {
  wsUrl: requiredEnv("FILFINANCE_EDGE_WS_URL"),
  publishUrl: requiredEnv("FILFINANCE_EDGE_PUBLISH_URL"),
  publishSecret: requiredEnv("FILFINANCE_EDGE_PUBLISH_SECRET"),
  ticket: optionalEnv("FILFINANCE_EDGE_LOAD_TEST_TICKET"),
  ticketUrl: optionalEnv("FILFINANCE_EDGE_LOAD_TEST_TICKET_URL"),
  accessToken: optionalEnv("FILFINANCE_EDGE_LOAD_TEST_ACCESS_TOKEN"),
  authBaseUrl: optionalEnv("FILFINANCE_LOAD_TEST_BETTER_AUTH_URL"),
  authEmail: optionalEnv("FILFINANCE_LOAD_TEST_EMAIL"),
  authPassword: optionalEnv("FILFINANCE_LOAD_TEST_PASSWORD"),
  clients: envInt("FILFINANCE_EDGE_LOAD_TEST_CLIENTS", DEFAULT_CLIENTS),
  tiers: envIntList("FILFINANCE_EDGE_LOAD_TEST_TIERS"),
  messages: envInt("FILFINANCE_EDGE_LOAD_TEST_MESSAGES", DEFAULT_MESSAGES),
  connectBatchSize: envInt(
    "FILFINANCE_EDGE_LOAD_TEST_CONNECT_BATCH_SIZE",
    DEFAULT_CONNECT_BATCH_SIZE
  ),
  connectDelayMs: envInt(
    "FILFINANCE_EDGE_LOAD_TEST_CONNECT_DELAY_MS",
    DEFAULT_CONNECT_DELAY_MS
  ),
  publishIntervalMs: envInt(
    "FILFINANCE_EDGE_LOAD_TEST_PUBLISH_INTERVAL_MS",
    DEFAULT_PUBLISH_INTERVAL_MS
  ),
  timeoutMs: envInt("FILFINANCE_EDGE_LOAD_TEST_TIMEOUT_MS", DEFAULT_TIMEOUT_MS),
};

if (!config.ticket && (!config.ticketUrl || (!config.accessToken && !hasPasswordAuth()))) {
  throw new Error(
    [
      "Edge load test requires either FILFINANCE_EDGE_LOAD_TEST_TICKET,",
      "or FILFINANCE_EDGE_LOAD_TEST_TICKET_URL plus FILFINANCE_EDGE_LOAD_TEST_ACCESS_TOKEN,",
      "or email/password credentials to refresh access tokens during long runs.",
    ].join(" ")
  );
}

const tiers = config.tiers.length > 0 ? config.tiers : [config.clients];
const reports = [];

try {
  for (const clients of tiers) {
    const report = await runTier(clients);
    reports.push(report);
    console.log(JSON.stringify({ stage: "tier_report", report }));

    if (!report.ok) {
      process.exitCode = 1;
      break;
    }
  }
} finally {
  console.log(
    JSON.stringify(
      {
        stage: "load_test_done",
        report: {
          started_at: reports[0]?.started_at ?? new Date().toISOString(),
          finished_at: new Date().toISOString(),
          tiers: reports,
          ok: reports.length > 0 && reports.every((report) => report.ok),
        },
      },
      null,
      2
    )
  );
}

async function runTier(clients) {
  const context = createContext(clients);

  try {
    console.log(
      JSON.stringify({
        stage: "connect_start",
        started_at: context.startedAt,
        clients,
        messages: config.messages,
      })
    );

    for (let index = 0; index < clients; index += config.connectBatchSize) {
      const batch = Array.from(
        { length: Math.min(config.connectBatchSize, clients - index) },
        (_, offset) => index + offset
      );
      const ticket = await resolveEdgeTicket();
      await Promise.all(batch.map((clientIndex) => connectClient(clientIndex, context, ticket)));
      await sleep(config.connectDelayMs);
    }

    console.log(
      JSON.stringify({
        stage: "connect_done",
        clients,
        connected: context.connected,
        connection_failures: context.connectionFailures,
      })
    );

    for (let index = 0; index < config.messages; index += 1) {
      const message = buildLoadTestMessage(index + 1);
      const eventId = message.payload.event_id;
      context.sentAtByEventId.set(eventId, performance.now());
      context.deliveriesByEventId.set(eventId, 0);
      context.publishedEventIds.push(eventId);
      context.publishedMessageTypes[message.type] =
        (context.publishedMessageTypes[message.type] ?? 0) + 1;
      await publishMessage(message);
      await sleep(config.publishIntervalMs);
    }

    await waitForDeliveries(context);
    context.snapshotProbe = await probeSnapshot(context);
    context.heartbeatProbe = await probeHeartbeat(context);
    return buildReport(context);
  } finally {
    closeContext(context);
  }
}

function createContext(clients) {
  return {
    startedAt: new Date().toISOString(),
    clients,
    closing: false,
    sockets: [],
    sentAtByEventId: new Map(),
    deliveriesByEventId: new Map(),
    publishedEventIds: [],
    publishedMessageTypes: {},
    receivedMessageTypes: {},
    latenciesMs: [],
    connected: 0,
    connectionFailures: 0,
    unexpectedCloses: 0,
    invalidMessages: 0,
    snapshotProbe: null,
    heartbeatProbe: null,
  };
}

function connectClient(index, context, ticket) {
  return new Promise((resolve) => {
    const socket = new WebSocket(appendTicket(config.wsUrl, ticket));
    let settled = false;
    const timeout = setTimeout(() => {
      if (settled) {
        return;
      }
      settled = true;
      context.connectionFailures += 1;
      socket.close();
      resolve(null);
    }, config.timeoutMs);

    socket.addEventListener("open", () => {
      if (settled) {
        return;
      }
      settled = true;
      clearTimeout(timeout);
      context.connected += 1;
      context.sockets.push(socket);
      resolve(socket);
    });

    socket.addEventListener("message", (event) => {
      recordMessage(context, event.data);
    });

    socket.addEventListener("close", () => {
      if (!settled) {
        settled = true;
        clearTimeout(timeout);
        context.connectionFailures += 1;
        resolve(null);
        return;
      }

      if (!context.closing) {
        context.unexpectedCloses += 1;
      }
    });

    socket.addEventListener("error", () => {
      if (!settled) {
        settled = true;
        clearTimeout(timeout);
        context.connectionFailures += 1;
        resolve(null);
      }
    });
  });
}

async function publishMessage(message) {
  const body = JSON.stringify(message);
  const timestamp = new Date().toISOString();
  const signature = await buildPublishSignature(config.publishSecret, timestamp, body);
  const response = await fetch(config.publishUrl, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      "X-FilFinance-Timestamp": timestamp,
      "X-FilFinance-Signature": signature,
    },
    body,
  });

  if (!response.ok) {
    throw new Error(`publish failed with HTTP ${response.status}: ${await response.text()}`);
  }
}

function recordMessage(context, rawData) {
  if (typeof rawData !== "string") {
    return;
  }

  let message;
  try {
    message = JSON.parse(rawData);
  } catch {
    context.invalidMessages += 1;
    return;
  }

  context.receivedMessageTypes[message.type] =
    (context.receivedMessageTypes[message.type] ?? 0) + 1;

  const eventId = message?.payload?.event_id;
  if (!context.sentAtByEventId.has(eventId)) {
    return;
  }

  context.deliveriesByEventId.set(
    eventId,
    (context.deliveriesByEventId.get(eventId) ?? 0) + 1
  );
  context.latenciesMs.push(performance.now() - context.sentAtByEventId.get(eventId));
}

async function waitForDeliveries(context) {
  const expectedDeliveries = context.connected * config.messages;
  const deadline = Date.now() + config.timeoutMs;

  while (Date.now() < deadline) {
    if (totalDeliveries(context) >= expectedDeliveries) {
      return;
    }

    await sleep(100);
  }
}

async function probeSnapshot(context) {
  const hello = await openProbeAndReadHello();
  const snapshot = hello.message?.payload?.snapshot ?? {};
  hello.socket?.close();
  const serializedSnapshot = JSON.stringify(snapshot);
  const latestEventId = lastPublishedEventId(context);
  const observedLatestEventId = snapshot.latest_event_id ?? null;
  const latestEventIdMatchesExpected = observedLatestEventId === latestEventId;
  const latestEventIdCompatible =
    latestEventIdMatchesExpected ||
    (isNonEmptyString(observedLatestEventId) && !isLoadTestEventId(observedLatestEventId));
  const hasSourceStatus = context.publishedMessageTypes.source_status > 0;
  const hasTranslatedNewsItem = context.publishedMessageTypes.translated_news_item > 0;
  const hasAudioEvent =
    (context.publishedMessageTypes.audio_ready ?? 0) +
      (context.publishedMessageTypes.audio_failed ?? 0) +
      (context.publishedMessageTypes.audio_finalized ?? 0) >
    0;

  return {
    ok:
      hello.ok &&
      latestEventIdCompatible &&
      (!hasSourceStatus || snapshot.source_statuses?.length > 0) &&
      (!hasTranslatedNewsItem || snapshot.translated_news_items?.length > 0) &&
      (!hasAudioEvent || snapshot.audio_events?.length > 0),
    latest_event_id: observedLatestEventId,
    expected_latest_event_id: latestEventId,
    latest_event_id_matches_expected: latestEventIdMatchesExpected,
    latest_event_id_compatible: latestEventIdCompatible,
    translated_news_item_count: snapshot.translated_news_items?.length ?? 0,
    audio_event_count: snapshot.audio_events?.length ?? 0,
    source_status_count: snapshot.source_statuses?.length ?? 0,
    snapshot_bytes: serializedSnapshot.length,
    error: hello.error,
  };
}

async function probeHeartbeat(context) {
  const probe = await openProbeAndReadHello();
  if (!probe.ok) {
    return { ok: false, sources_count: 0, error: probe.error };
  }

  const heartbeat = await readMessageAfter(
    probe.socket,
    () => {
      probe.socket.send("heartbeat");
    },
    (message) => message.type === "heartbeat"
  );
  probe.socket.close();

  const sources = heartbeat.message?.payload?.sources;
  const hasSourceStatus = context.publishedMessageTypes.source_status > 0;
  return {
    ok:
      heartbeat.ok &&
      heartbeat.message?.type === "heartbeat" &&
      Array.isArray(sources) &&
      (!hasSourceStatus || sources.length > 0),
    sources_count: Array.isArray(sources) ? sources.length : 0,
    ignored_messages: heartbeat.ignored_messages ?? 0,
    error: heartbeat.error,
  };
}

async function openProbeAndReadHello() {
  let ticket;
  try {
    ticket = await resolveEdgeTicket();
  } catch (error) {
    return {
      ok: false,
      socket: null,
      message: null,
      error: `probe ticket error: ${errorMessage(error)}`,
    };
  }

  return new Promise((resolve) => {
    const socket = new WebSocket(appendTicket(config.wsUrl, ticket));
    const timeout = setTimeout(() => {
      socket.close();
      resolve({ ok: false, socket: null, message: null, error: "probe timeout" });
    }, config.timeoutMs);

    socket.addEventListener(
      "message",
      (event) => {
        clearTimeout(timeout);
        const parsed = parseJsonMessage(event.data);
        if (!parsed.ok) {
          socket.close();
          resolve({ ok: false, socket: null, message: null, error: parsed.error });
          return;
        }

        if (parsed.message.type !== "hello") {
          socket.close();
          resolve({
            ok: false,
            socket: null,
            message: parsed.message,
            error: `expected hello, received ${parsed.message.type}`,
          });
          return;
        }

        resolve({ ok: true, socket, message: parsed.message, error: null });
      },
      { once: true }
    );

    socket.addEventListener("error", () => {
      clearTimeout(timeout);
      resolve({ ok: false, socket: null, message: null, error: "probe websocket error" });
    });
  });
}

function readMessageAfter(socket, trigger, predicate = () => true) {
  return new Promise((resolve) => {
    let ignoredMessages = 0;
    let settled = false;

    const finish = (result) => {
      if (settled) {
        return;
      }

      settled = true;
      clearTimeout(timeout);
      socket.removeEventListener("message", onMessage);
      resolve({ ...result, ignored_messages: ignoredMessages });
    };

    const timeout = setTimeout(() => {
      finish({ ok: false, message: null, error: "probe message timeout" });
    }, config.timeoutMs);

    const onMessage = (event) => {
      const parsed = parseJsonMessage(event.data);
      if (!parsed.ok) {
        finish(parsed);
        return;
      }

      if (!predicate(parsed.message)) {
        ignoredMessages += 1;
        return;
      }

      finish(parsed);
    };

    socket.addEventListener("message", onMessage);

    trigger();
  });
}

function parseJsonMessage(rawData) {
  if (typeof rawData !== "string") {
    return { ok: false, message: null, error: "message is not text" };
  }

  try {
    return { ok: true, message: JSON.parse(rawData), error: null };
  } catch {
    return { ok: false, message: null, error: "invalid json message" };
  }
}

function buildReport(context) {
  const expectedDeliveries = context.connected * config.messages;
  const receivedDeliveries = totalDeliveries(context);
  const report = {
    started_at: context.startedAt,
    finished_at: new Date().toISOString(),
    clients_requested: context.clients,
    ok: true,
    ws: {
      requested: context.clients,
      connected: context.connected,
      connection_failures: context.connectionFailures,
      unexpected_closes: context.unexpectedCloses,
      invalid_messages: context.invalidMessages,
      message_types: context.receivedMessageTypes,
    },
    publish: {
      messages_published: config.messages,
      message_types: context.publishedMessageTypes,
      expected_deliveries: expectedDeliveries,
      received_deliveries: receivedDeliveries,
      missed_deliveries: Math.max(0, expectedDeliveries - receivedDeliveries),
    },
    latency_ms: {
      p50: percentile(context.latenciesMs, 0.5),
      p95: percentile(context.latenciesMs, 0.95),
      p99: percentile(context.latenciesMs, 0.99),
      max: context.latenciesMs.length ? Number(Math.max(...context.latenciesMs).toFixed(2)) : null,
    },
    snapshot: context.snapshotProbe,
    heartbeat: context.heartbeatProbe,
  };

  report.ok =
    report.ws.connection_failures === 0 &&
    report.ws.unexpected_closes === 0 &&
    report.ws.invalid_messages === 0 &&
    report.publish.missed_deliveries === 0 &&
    report.snapshot?.ok === true &&
    report.heartbeat?.ok === true;

  return report;
}

function closeContext(context) {
  context.closing = true;
  for (const socket of context.sockets) {
    socket.close();
  }
}

function totalDeliveries(context) {
  let total = 0;
  for (const count of context.deliveriesByEventId.values()) {
    total += count;
  }
  return total;
}

function lastPublishedEventId(context) {
  return context.publishedEventIds[context.publishedEventIds.length - 1] ?? null;
}

function appendTicket(wsUrl, ticket) {
  const parsed = new URL(wsUrl);
  parsed.searchParams.set("ticket", ticket);
  return parsed.toString();
}

async function resolveEdgeTicket() {
  if (!config.ticketUrl) {
    return config.ticket;
  }

  let response = await fetchEdgeTicket(await resolveEdgeAccessToken());

  if ((response.status === 401 || response.status === 403) && hasPasswordAuth()) {
    config.accessToken = await fetchBetterAuthAccessToken();
    response = await fetchEdgeTicket(config.accessToken);
  }

  if (!response.ok) {
    throw new Error(`edge ticket refresh failed with HTTP ${response.status}`);
  }

  const payload = await response.json();
  if (!payload?.ticket) {
    throw new Error("edge ticket refresh did not return a ticket");
  }

  return payload.ticket;
}

async function fetchEdgeTicket(token) {
  return fetch(config.ticketUrl, {
    headers: {
      Authorization: `Bearer ${token}`,
    },
  });
}

async function resolveEdgeAccessToken() {
  if (config.accessToken) {
    return config.accessToken;
  }

  config.accessToken = await fetchBetterAuthAccessToken();
  return config.accessToken;
}

async function fetchBetterAuthAccessToken() {
  if (!hasPasswordAuth()) {
    throw new Error(
      "edge ticket refresh requires a Better Auth JWT or email/password credentials"
    );
  }

  const authBaseUrl = config.authBaseUrl || new URL(config.ticketUrl).origin;
  const signInResponse = await fetch(new URL("/api/auth/sign-in/email", `${authBaseUrl}/`), {
    method: "POST",
    headers: {
      Origin: authBaseUrl,
      "Content-Type": "application/json",
    },
    body: JSON.stringify({ email: config.authEmail, password: config.authPassword }),
  });

  if (!signInResponse.ok) {
    throw new Error(`Better Auth password sign-in failed with HTTP ${signInResponse.status}`);
  }

  const cookie = signInResponse.headers
    .getSetCookie()
    .map((value) => value.split(";", 1)[0])
    .join("; ");
  if (!cookie) {
    throw new Error("Better Auth password sign-in did not return a session cookie");
  }

  const tokenResponse = await fetch(new URL("/api/auth/token", `${authBaseUrl}/`), {
    headers: { Cookie: cookie, Origin: authBaseUrl },
  });
  if (!tokenResponse.ok) {
    throw new Error(`Better Auth JWT request failed with HTTP ${tokenResponse.status}`);
  }
  const payload = await tokenResponse.json();
  if (!payload?.token) {
    throw new Error("Better Auth JWT response did not return a token");
  }

  return payload.token;
}

function hasPasswordAuth() {
  return Boolean(
    config.authEmail &&
      config.authPassword &&
      (config.authBaseUrl || config.ticketUrl)
  );
}

function percentile(values, rank) {
  if (!values.length) {
    return null;
  }

  const sorted = [...values].sort((left, right) => left - right);
  const index = Math.min(sorted.length - 1, Math.ceil(sorted.length * rank) - 1);
  return Number(sorted[index].toFixed(2));
}

function buildLoadTestMessage(sequence) {
  const builders = [
    translatedNewsItemMessage,
    audioReadyMessage,
    audioFailedMessage,
    audioFinalizedMessage,
    systemNoticeMessage,
    sourceStatusMessage,
  ];
  return builders[(sequence - 1) % builders.length](sequence);
}

function translatedNewsItemMessage(sequence) {
  const serverTime = new Date().toISOString();
  const eventId = loadEventId("translated_news_item", sequence);
  return {
    type: "translated_news_item",
    payload: {
      event_id: eventId,
      server_time: serverTime,
      item: {
        id: sequence,
        news_event_id: sequence + 100_000,
        language: "fr",
        status: "ready",
        source: "load_test",
        event_kind: "headline",
        source_title: `Load test source ${sequence}`,
        source_body: null,
        translated_title: `Titre load test ${sequence}`,
        translated_body: null,
        narration_text: `Titre load test ${sequence}`,
        translation_mode: "load_test",
        error_message: null,
        news_level: null,
        url: null,
        timestamp: serverTime,
        created_at: serverTime,
        updated_at: serverTime,
      },
    },
  };
}

function audioReadyMessage(sequence) {
  const serverTime = new Date().toISOString();
  const streamId = `load-test-audio-${sequence}`;
  return {
    type: "audio_ready",
    payload: {
      event_id: loadEventId("audio_ready", sequence),
      server_time: serverTime,
      translation_id: sequence,
      news_event_id: sequence + 100_000,
      timestamp: serverTime,
      stream_id: streamId,
      playlist_url: `/audio/clips/${streamId}/playlist.m3u8`,
      published_at: serverTime,
      target_duration_secs: 2,
    },
  };
}

function audioFailedMessage(sequence) {
  const serverTime = new Date().toISOString();
  return {
    type: "audio_failed",
    payload: {
      event_id: loadEventId("audio_failed", sequence),
      server_time: serverTime,
      translation_id: sequence,
      stream_id: `load-test-audio-${sequence}`,
      error_message: "load test audio failure fixture",
      failed_at: serverTime,
    },
  };
}

function audioFinalizedMessage(sequence) {
  const serverTime = new Date().toISOString();
  const streamId = `load-test-audio-${sequence}`;
  return {
    type: "audio_finalized",
    payload: {
      event_id: loadEventId("audio_finalized", sequence),
      server_time: serverTime,
      translation_id: sequence,
      stream_id: streamId,
      playlist_url: `/audio/clips/${streamId}/playlist.m3u8`,
      finalized_at: serverTime,
    },
  };
}

function systemNoticeMessage(sequence) {
  const serverTime = new Date().toISOString();
  return {
    type: "system_notice",
    payload: {
      event_id: loadEventId("system_notice", sequence),
      server_time: serverTime,
      severity: "info",
      message: `load test notice ${sequence}`,
    },
  };
}

function sourceStatusMessage(sequence) {
  const serverTime = new Date().toISOString();
  return {
    type: "source_status",
    payload: {
      event_id: loadEventId("source_status", sequence),
      server_time: serverTime,
      sources: [
        {
          source: "financial_juice",
          enabled: true,
          status: "active",
          updated_at: serverTime,
        },
        {
          source: "financial_juice_browser",
          enabled: false,
          status: "blocked",
          updated_at: serverTime,
        },
      ],
    },
  };
}

function loadEventId(type, sequence) {
  return `load-test:${type}:${Date.now()}:${String(sequence).padStart(5, "0")}`;
}

function isLoadTestEventId(value) {
  return typeof value === "string" && value.startsWith("load-test:");
}

function isNonEmptyString(value) {
  return typeof value === "string" && value.trim() !== "";
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

function envIntList(name) {
  const value = process.env[name]?.trim();
  if (!value) {
    return [];
  }

  return value.split(",").map((part) => {
    const parsed = Number.parseInt(part.trim(), 10);
    if (!Number.isFinite(parsed) || parsed <= 0) {
      throw new Error(`${name} must be a comma-separated list of positive integers`);
    }
    return parsed;
  });
}

function requiredEnv(name) {
  const value = process.env[name]?.trim();
  if (!value) {
    throw new Error(`${name} is required`);
  }

  return value;
}

function optionalEnv(name) {
  return process.env[name]?.trim() || "";
}

function errorMessage(error) {
  return error instanceof Error ? error.message : String(error);
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
