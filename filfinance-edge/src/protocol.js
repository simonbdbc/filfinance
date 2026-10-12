export const EDGE_PROTOCOL_VERSION = 1;
export const AUDIO_EVENT_HISTORY_LIMIT = 20;
export const TRANSLATED_NEWS_ITEM_HISTORY_LIMIT = 50;
export const PUBLISH_SIGNATURE_MAX_SKEW_MS = 120_000;

export const PUBLISHABLE_MESSAGE_TYPES = new Set([
  "news_card",
  "translated_news_item",
  "now_playing",
  "audio_state",
  "audio_ready",
  "audio_failed",
  "audio_finalized",
  "calendar_alert_ready",
  "system_notice",
  "source_status",
]);

const FORBIDDEN_AUDIO_KEYS = new Set([
  "audio_chunk",
  "audio_start",
  "audio_end",
  "audio_error",
  "audio_base64",
  "audio_bytes",
  "bytes",
  "chunk",
  "chunks",
]);

export function nowIso() {
  return new Date().toISOString();
}

export function jsonResponse(payload, init = {}) {
  return new Response(JSON.stringify(payload), {
    ...init,
    headers: {
      "Content-Type": "application/json; charset=utf-8",
      ...init.headers,
    },
  });
}

export async function assertPublishAuthorized(
  request,
  env,
  rawBody,
  nowEpochMs = Date.now()
) {
  const expectedSecret = String(env.FILFINANCE_EDGE_PUBLISH_SECRET ?? "").trim();

  if (expectedSecret.length < 32) {
    return { ok: false, status: 500, error: "publish secret is not configured" };
  }

  const timestamp = request.headers.get("X-FilFinance-Timestamp")?.trim() ?? "";
  const providedSignature = request.headers.get("X-FilFinance-Signature")?.trim() ?? "";

  if (!timestamp) {
    return { ok: false, status: 401, error: "missing publish timestamp" };
  }

  if (!providedSignature) {
    return { ok: false, status: 401, error: "missing publish signature" };
  }

  const timestampEpochMs = Date.parse(timestamp);
  if (!Number.isFinite(timestampEpochMs)) {
    return { ok: false, status: 401, error: "invalid publish timestamp" };
  }

  if (Math.abs(nowEpochMs - timestampEpochMs) > PUBLISH_SIGNATURE_MAX_SKEW_MS) {
    return { ok: false, status: 401, error: "stale publish signature" };
  }

  const expectedSignature = await buildPublishSignature(expectedSecret, timestamp, rawBody);
  if (!constantTimeEqual(expectedSignature, providedSignature)) {
    return { ok: false, status: 401, error: "invalid publish signature" };
  }

  return { ok: true };
}

export async function buildPublishSignature(secret, timestamp, rawBody) {
  return `sha256=${await signHs256Base64Url(`${timestamp}.${rawBody}`, secret)}`;
}

export async function assertWebSocketAuthorized(request, env) {
  const expectedSecret = env.FILFINANCE_EDGE_TICKET_SECRET;

  if (!expectedSecret) {
    return { ok: false, status: 500, error: "edge ticket secret is not configured" };
  }

  const ticket = readEdgeLiveTicket(request);
  if (!ticket) {
    return { ok: false, status: 401, error: "missing edge live ticket" };
  }

  const verification = await verifyEdgeLiveTicket(ticket, expectedSecret);
  if (!verification.ok) {
    return { ok: false, status: 401, error: verification.error };
  }

  return { ok: true, claims: verification.claims };
}

export function readEdgeLiveTicket(request) {
  const url = new URL(request.url);
  return url.searchParams.get("ticket")?.trim() ?? "";
}

export async function verifyEdgeLiveTicket(ticket, secret, nowEpochSeconds = Math.floor(Date.now() / 1000)) {
  if (!secret) {
    return { ok: false, error: "edge ticket secret is not configured" };
  }

  const parts = ticket.split(".");
  if (parts.length !== 3 || parts.some((part) => part.length === 0)) {
    return { ok: false, error: "invalid edge live ticket format" };
  }

  let header;
  let claims;

  try {
    header = JSON.parse(textFromBase64Url(parts[0]));
    claims = JSON.parse(textFromBase64Url(parts[1]));
  } catch {
    return { ok: false, error: "invalid edge live ticket payload" };
  }

  if (header.alg !== "HS256") {
    return { ok: false, error: "unsupported edge live ticket algorithm" };
  }

  const expectedSignature = await signHs256Base64Url(`${parts[0]}.${parts[1]}`, secret);
  if (!constantTimeEqual(expectedSignature, parts[2])) {
    return { ok: false, error: "invalid edge live ticket signature" };
  }

  if (typeof claims.exp !== "number" || claims.exp <= nowEpochSeconds) {
    return { ok: false, error: "expired edge live ticket" };
  }

  if (typeof claims.iat !== "number" || claims.iat > nowEpochSeconds + 30) {
    return { ok: false, error: "invalid edge live ticket issued-at" };
  }

  if (claims.scope !== "live_metadata") {
    return { ok: false, error: "invalid edge live ticket scope" };
  }

  if (!isNonEmptyString(claims.sub) || !isNonEmptyString(claims.role)) {
    return { ok: false, error: "incomplete edge live ticket claims" };
  }

  return { ok: true, claims };
}

export function validatePublishMessage(message) {
  if (!isRecord(message)) {
    return { ok: false, error: "message must be a JSON object" };
  }

  if (!PUBLISHABLE_MESSAGE_TYPES.has(message.type)) {
    return { ok: false, error: `unsupported live message type: ${String(message.type)}` };
  }

  if (!isRecord(message.payload)) {
    return { ok: false, error: "message payload must be a JSON object" };
  }

  if (containsForbiddenAudioKey(message)) {
    return { ok: false, error: "metadata live messages must not contain audio chunks" };
  }

  if (!isNonEmptyString(message.payload.event_id)) {
    return { ok: false, error: "payload.event_id is required" };
  }

  if (!isNonEmptyString(message.payload.server_time)) {
    return { ok: false, error: "payload.server_time is required" };
  }

  const typedError = validateTypedPayload(message);
  if (typedError) {
    return { ok: false, error: typedError };
  }

  return { ok: true, message };
}

export function buildHelloMessage(snapshot, serverTime = nowIso()) {
  return {
    type: "hello",
    payload: {
      protocol_version: EDGE_PROTOCOL_VERSION,
      server_time: serverTime,
      snapshot: normalizeSnapshot(snapshot),
    },
  };
}

export function buildHeartbeatMessage(
  sourceStatuses = [],
  calendarDaySummary = null,
  serverTime = nowIso()
) {
  return {
    type: "heartbeat",
    payload: {
      server_time: serverTime,
      sources: normalizeSourceStatuses(sourceStatuses),
      calendar_day_summary: normalizeCalendarDaySummary(calendarDaySummary),
    },
  };
}

export function emptySnapshot() {
  return {
    latest_event_id: null,
    now_playing: null,
    audio_state: null,
    translated_news_items: [],
    audio_events: [],
    source_statuses: [],
    calendar_day_summary: null,
  };
}

export function applySnapshotFromMessage(snapshot, message) {
  const nextSnapshot = normalizeSnapshot(snapshot);
  const payload = message.payload;

  nextSnapshot.latest_event_id = payload.event_id;

  if (message.type === "now_playing") {
    nextSnapshot.now_playing = payload.now_playing ?? null;
  }

  if (message.type === "audio_state") {
    nextSnapshot.audio_state = payload.state ?? null;
  }

  if (isTranslatedNewsItemMessage(message)) {
    nextSnapshot.translated_news_items = appendTranslatedNewsItem(
      nextSnapshot.translated_news_items,
      message
    );
  }

  if (isAudioEventMessage(message)) {
    nextSnapshot.audio_events = appendAudioEvent(nextSnapshot.audio_events, message);
  }

  if (message.type === "source_status") {
    nextSnapshot.source_statuses = normalizeSourceStatuses(payload.sources);
    nextSnapshot.calendar_day_summary = normalizeCalendarDaySummary(payload.calendar_day_summary);
  }

  return nextSnapshot;
}

export function normalizeSnapshot(snapshot) {
  return {
    latest_event_id: snapshot?.latest_event_id ?? null,
    now_playing: snapshot?.now_playing ?? null,
    audio_state: snapshot?.audio_state ?? null,
    translated_news_items: normalizeTranslatedNewsItems(snapshot?.translated_news_items),
    audio_events: normalizeAudioEvents(snapshot?.audio_events),
    source_statuses: normalizeSourceStatuses(snapshot?.source_statuses),
    calendar_day_summary: normalizeCalendarDaySummary(snapshot?.calendar_day_summary),
  };
}

function normalizeSourceStatuses(value) {
  if (!Array.isArray(value)) {
    return [];
  }

  const allowedStatuses = new Set(["active", "disabled", "blocked", "degraded", "stale"]);
  const nextStatuses = [];
  const seenSources = new Set();

  for (const item of value) {
    if (!isRecord(item) || !isNonEmptyString(item.source) || seenSources.has(item.source)) {
      continue;
    }

    const status = allowedStatuses.has(item.status) ? item.status : "disabled";
    nextStatuses.push({
      source: item.source,
      enabled: item.enabled === true,
      status,
      updated_at: isNonEmptyString(item.updated_at) ? item.updated_at : nowIso(),
    });
    seenSources.add(item.source);
  }

  return nextStatuses;
}

function normalizeCalendarDaySummary(value) {
  if (!isRecord(value) || !isNonEmptyString(value.local_day)) {
    return null;
  }

  const allowedStatuses = new Set([
    "missing",
    "pending",
    "summarized",
    "audio_generating",
    "audio_ready",
    "error",
  ]);
  const status = allowedStatuses.has(value.status) ? value.status : "missing";

  return {
    local_day: value.local_day,
    status,
    audio_available: status === "audio_ready" && value.audio_available === true,
    updated_at: isNonEmptyString(value.updated_at) ? value.updated_at : nowIso(),
  };
}

function appendTranslatedNewsItem(translatedNewsItems, message) {
  const currentItems = normalizeTranslatedNewsItems(translatedNewsItems);
  const messageKey = translatedNewsItemKey(message);
  const nextItems = currentItems.filter((item) => translatedNewsItemKey(item) !== messageKey);
  nextItems.push(message);
  return nextItems.slice(-TRANSLATED_NEWS_ITEM_HISTORY_LIMIT);
}

function appendAudioEvent(audioEvents, message) {
  const currentEvents = normalizeAudioEvents(audioEvents);
  const nextEvents = currentEvents.filter((event) => event.payload.event_id !== message.payload.event_id);
  nextEvents.push(message);
  return nextEvents.slice(-AUDIO_EVENT_HISTORY_LIMIT);
}

function normalizeTranslatedNewsItems(value) {
  if (!Array.isArray(value)) {
    return [];
  }

  const nextItems = [];
  const seenKeys = new Set();

  for (const message of value.filter(isTranslatedNewsItemMessage).slice(-TRANSLATED_NEWS_ITEM_HISTORY_LIMIT)) {
    const key = translatedNewsItemKey(message);
    if (seenKeys.has(key)) {
      continue;
    }
    seenKeys.add(key);
    nextItems.push(message);
  }

  return nextItems;
}

function normalizeAudioEvents(value) {
  if (!Array.isArray(value)) {
    return [];
  }

  return value.filter(isAudioEventMessage).slice(-AUDIO_EVENT_HISTORY_LIMIT);
}

function isTranslatedNewsItemMessage(message) {
  return (
    isRecord(message) &&
    message.type === "translated_news_item" &&
    isRecord(message.payload) &&
    isNonEmptyString(message.payload.event_id) &&
    isRecord(message.payload.item)
  );
}

function translatedNewsItemKey(message) {
  const itemId = message?.payload?.item?.id;
  if (Number.isFinite(itemId)) {
    return `translation:${itemId}`;
  }

  return `event:${message.payload.event_id}`;
}

function isAudioEventMessage(message) {
  return (
    isRecord(message) &&
    ["audio_ready", "audio_failed", "audio_finalized", "calendar_alert_ready"].includes(
      message.type
    ) &&
    isRecord(message.payload) &&
    isNonEmptyString(message.payload.event_id)
  );
}

function validateTypedPayload(message) {
  const payload = message.payload;

  if (message.type === "news_card" && !isRecord(payload.card)) {
    return "news_card payload.card is required";
  }

  if (message.type === "translated_news_item" && !isRecord(payload.item)) {
    return "translated_news_item payload.item is required";
  }

  if (message.type === "now_playing" && payload.now_playing !== null && !isRecord(payload.now_playing)) {
    return "now_playing payload.now_playing must be an object or null";
  }

  if (message.type === "audio_state" && !isRecord(payload.state)) {
    return "audio_state payload.state is required";
  }

  if (message.type === "audio_ready") {
    if (!Number.isFinite(payload.translation_id)) {
      return "audio_ready payload.translation_id is required";
    }

    if (!isNonEmptyString(payload.stream_id)) {
      return "audio_ready payload.stream_id is required";
    }

    if (!isNonEmptyString(payload.playlist_url)) {
      return "audio_ready payload.playlist_url is required";
    }

    if (!isNonEmptyString(payload.published_at)) {
      return "audio_ready payload.published_at is required";
    }

    if (!Number.isFinite(payload.target_duration_secs) || payload.target_duration_secs <= 0) {
      return "audio_ready payload.target_duration_secs must be positive";
    }
  }

  if (message.type === "audio_failed") {
    if (
      payload.translation_id !== null &&
      payload.translation_id !== undefined &&
      !Number.isFinite(payload.translation_id)
    ) {
      return "audio_failed payload.translation_id must be a number or null";
    }

    if (
      payload.stream_id !== null &&
      payload.stream_id !== undefined &&
      !isNonEmptyString(payload.stream_id)
    ) {
      return "audio_failed payload.stream_id must be a string or null";
    }

    if (!isNonEmptyString(payload.error_message)) {
      return "audio_failed payload.error_message is required";
    }

    if (!isNonEmptyString(payload.failed_at)) {
      return "audio_failed payload.failed_at is required";
    }
  }

  if (message.type === "audio_finalized") {
    if (!Number.isFinite(payload.translation_id)) {
      return "audio_finalized payload.translation_id is required";
    }

    if (!isNonEmptyString(payload.stream_id)) {
      return "audio_finalized payload.stream_id is required";
    }

    if (!isNonEmptyString(payload.playlist_url)) {
      return "audio_finalized payload.playlist_url is required";
    }

    if (!isNonEmptyString(payload.finalized_at)) {
      return "audio_finalized payload.finalized_at is required";
    }
  }

  if (message.type === "calendar_alert_ready") {
    if (!Number.isFinite(payload.calendar_item_id)) {
      return "calendar_alert_ready payload.calendar_item_id is required";
    }

    if (!Number.isFinite(payload.alert_offset_secs) || payload.alert_offset_secs < 0) {
      return "calendar_alert_ready payload.alert_offset_secs must be non-negative";
    }

    for (const field of [
      "scheduled_at",
      "stream_id",
      "playlist_url",
      "priority",
      "expires_at",
      "published_at",
    ]) {
      if (!isNonEmptyString(payload[field])) {
        return `calendar_alert_ready payload.${field} is required`;
      }
    }
  }

  if (message.type === "system_notice") {
    if (!["info", "warning", "error"].includes(payload.severity)) {
      return "system_notice payload.severity must be info, warning or error";
    }

    if (!isNonEmptyString(payload.message)) {
      return "system_notice payload.message is required";
    }
  }

  if (message.type === "source_status") {
    if (!Array.isArray(payload.sources)) {
      return "source_status payload.sources is required";
    }

    if (
      payload.calendar_day_summary !== undefined &&
      payload.calendar_day_summary !== null &&
      normalizeCalendarDaySummary(payload.calendar_day_summary) === null
    ) {
      return "source_status payload.calendar_day_summary is invalid";
    }
  }

  return null;
}

function containsForbiddenAudioKey(value) {
  if (Array.isArray(value)) {
    return value.some(containsForbiddenAudioKey);
  }

  if (!isRecord(value)) {
    return false;
  }

  return Object.entries(value).some(([key, child]) => {
    return FORBIDDEN_AUDIO_KEYS.has(key) || containsForbiddenAudioKey(child);
  });
}

function isRecord(value) {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isNonEmptyString(value) {
  return typeof value === "string" && value.trim().length > 0;
}

async function signHs256Base64Url(signingInput, secret) {
  const key = await crypto.subtle.importKey(
    "raw",
    new TextEncoder().encode(secret),
    { name: "HMAC", hash: "SHA-256" },
    false,
    ["sign"]
  );
  const signature = await crypto.subtle.sign("HMAC", key, new TextEncoder().encode(signingInput));
  return bytesToBase64Url(new Uint8Array(signature));
}

function textFromBase64Url(value) {
  return new TextDecoder().decode(bytesFromBase64Url(value));
}

function bytesFromBase64Url(value) {
  const base64 = value.replace(/-/g, "+").replace(/_/g, "/");
  const padded = base64.padEnd(Math.ceil(base64.length / 4) * 4, "=");
  const binary = atob(padded);
  const bytes = new Uint8Array(binary.length);

  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index);
  }

  return bytes;
}

function bytesToBase64Url(bytes) {
  let binary = "";
  for (const byte of bytes) {
    binary += String.fromCharCode(byte);
  }

  return btoa(binary).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/g, "");
}

function constantTimeEqual(left, right) {
  const leftBytes = new TextEncoder().encode(left);
  const rightBytes = new TextEncoder().encode(right);
  let diff = leftBytes.length ^ rightBytes.length;
  const length = Math.max(leftBytes.length, rightBytes.length);

  for (let index = 0; index < length; index += 1) {
    diff |= (leftBytes[index] ?? 0) ^ (rightBytes[index] ?? 0);
  }

  return diff === 0;
}
