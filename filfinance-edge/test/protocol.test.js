import assert from "node:assert/strict";
import test from "node:test";
import { randomBytes } from "node:crypto";

import {
  AUDIO_EVENT_HISTORY_LIMIT,
  PUBLISH_SIGNATURE_MAX_SKEW_MS,
  TRANSLATED_NEWS_ITEM_HISTORY_LIMIT,
  applySnapshotFromMessage,
  assertPublishAuthorized,
  buildHeartbeatMessage,
  buildHelloMessage,
  buildPublishSignature,
  emptySnapshot,
  validatePublishMessage,
  verifyEdgeLiveTicket,
} from "../src/protocol.js";

test("validatePublishMessage accepts a metadata news card", () => {
  const message = {
    type: "news_card",
    payload: {
      event_id: "evt-001",
      server_time: "2026-05-03T09:00:00Z",
      card: {
        news_event_id: 7,
        translation_id: 11,
        source: "financial_juice",
        event_kind: "headline",
        source_title: "Fed headline",
        translated_title: "Titre Fed",
        news_level: "active active-critical",
        url: "https://example.test/news",
        timestamp: "2026-05-03T08:59:58Z",
      },
    },
  };

  assert.equal(validatePublishMessage(message).ok, true);
});

test("validatePublishMessage accepts public source status metadata", () => {
  const message = sourceStatusMessage();

  assert.equal(validatePublishMessage(message).ok, true);
});

test("validatePublishMessage rejects malformed calendar day summary metadata", () => {
  const message = sourceStatusMessage();
  message.payload.calendar_day_summary = {
    status: "audio_ready",
    audio_available: true,
    updated_at: "2026-05-09T09:12:00Z",
  };

  const result = validatePublishMessage(message);

  assert.equal(result.ok, false);
  assert.match(result.error, /calendar_day_summary/);
});

test("validatePublishMessage accepts an audio_ready clip metadata event", () => {
  const message = {
    type: "audio_ready",
    payload: {
      event_id: "audio_ready:00000000000000000011",
      server_time: "2026-05-04T08:00:00Z",
      translation_id: 11,
      news_event_id: 7,
      timestamp: "2026-05-04T07:59:58Z",
      stream_id: "clip-20260504-000011",
      playlist_url: "/audio/clips/clip-20260504-000011/playlist.m3u8",
      published_at: "2026-05-04T08:00:00Z",
      target_duration_secs: 2,
    },
  };

  assert.equal(validatePublishMessage(message).ok, true);
});

test("validatePublishMessage rejects incomplete audio_ready events", () => {
  const message = {
    type: "audio_ready",
    payload: {
      event_id: "audio_ready:00000000000000000011",
      server_time: "2026-05-04T08:00:00Z",
      translation_id: 11,
      stream_id: "clip-20260504-000011",
      published_at: "2026-05-04T08:00:00Z",
      target_duration_secs: 2,
    },
  };

  const result = validatePublishMessage(message);

  assert.equal(result.ok, false);
  assert.match(result.error, /playlist_url/);
});

test("validatePublishMessage accepts audio_failed and audio_finalized metadata events", () => {
  const failed = {
    type: "audio_failed",
    payload: {
      event_id: "audio_failed:00000000000000000011",
      server_time: "2026-05-04T08:00:01Z",
      translation_id: 11,
      stream_id: "clip-20260504-000011",
      error_message: "tts timeout",
      failed_at: "2026-05-04T08:00:01Z",
    },
  };
  const finalized = {
    type: "audio_finalized",
    payload: {
      event_id: "audio_finalized:00000000000000000011",
      server_time: "2026-05-04T08:00:02Z",
      translation_id: 11,
      stream_id: "clip-20260504-000011",
      playlist_url: "/audio/clips/clip-20260504-000011/playlist.m3u8",
      finalized_at: "2026-05-04T08:00:02Z",
    },
  };

  assert.equal(validatePublishMessage(failed).ok, true);
  assert.equal(validatePublishMessage(finalized).ok, true);
});

test("validatePublishMessage accepts a calendar alert audio event", () => {
  const message = {
    type: "calendar_alert_ready",
    payload: {
      event_id: "calendar_alert_ready:00000000000000000057",
      server_time: "2026-05-04T08:00:00Z",
      calendar_item_id: 42,
      alert_offset_secs: 300,
      scheduled_at: "2026-05-04T08:05:00Z",
      stream_id: "calendar-alert-42-300",
      playlist_url: "/audio/calendar/alerts/calendar-alert-42-300/playlist.m3u8",
      priority: "high",
      expires_at: "2026-05-04T08:06:00Z",
      published_at: "2026-05-04T08:00:00Z",
    },
  };

  const validation = validatePublishMessage(message);
  const snapshot = applySnapshotFromMessage(emptySnapshot(), message);

  assert.equal(validation.ok, true);
  assert.deepEqual(snapshot.audio_events, [message]);
});

test("validatePublishMessage rejects an incomplete calendar alert audio event", () => {
  const message = {
    type: "calendar_alert_ready",
    payload: {
      event_id: "calendar_alert_ready:00000000000000000057",
      server_time: "2026-05-04T08:00:00Z",
      calendar_item_id: 42,
      alert_offset_secs: 300,
      scheduled_at: "2026-05-04T08:05:00Z",
      stream_id: "calendar-alert-42-300",
      priority: "high",
      expires_at: "2026-05-04T08:06:00Z",
      published_at: "2026-05-04T08:00:00Z",
    },
  };

  const result = validatePublishMessage(message);

  assert.equal(result.ok, false);
  assert.match(result.error, /playlist_url/);
});

test("validatePublishMessage rejects binary audio-shaped payloads", () => {
  const message = {
    type: "news_card",
    payload: {
      event_id: "evt-002",
      server_time: "2026-05-03T09:00:01Z",
      card: {
        news_event_id: 7,
        source: "financial_juice",
        event_kind: "headline",
        source_title: "Fed headline",
        timestamp: "2026-05-03T08:59:58Z",
        bytes: [1, 2, 3],
      },
    },
  };

  const result = validatePublishMessage(message);

  assert.equal(result.ok, false);
  assert.match(result.error, /audio chunks/);
});

test("validatePublishMessage rejects audio_ready with embedded audio bytes", () => {
  const message = {
    type: "audio_ready",
    payload: {
      event_id: "audio_ready:00000000000000000012",
      server_time: "2026-05-04T08:00:00Z",
      translation_id: 12,
      stream_id: "clip-20260504-000012",
      playlist_url: "/audio/clips/clip-20260504-000012/playlist.m3u8",
      published_at: "2026-05-04T08:00:00Z",
      target_duration_secs: 2,
      audio_base64: "AAAA",
    },
  };

  const result = validatePublishMessage(message);

  assert.equal(result.ok, false);
  assert.match(result.error, /audio chunks/);
});

test("applySnapshotFromMessage keeps the minimal reconnect state", () => {
  const message = {
    type: "audio_state",
    payload: {
      event_id: "evt-audio-001",
      server_time: "2026-05-03T09:00:02Z",
      state: {
        status: "playing",
        replay_today_playlist_url: "/audio/replay/today/playlist.m3u8",
        detail: null,
        updated_at: "2026-05-03T09:00:02Z",
      },
    },
  };

  const snapshot = applySnapshotFromMessage(emptySnapshot(), message);

  assert.equal(snapshot.latest_event_id, "evt-audio-001");
  assert.equal(snapshot.audio_state.status, "playing");
  assert.equal(snapshot.now_playing, null);
  assert.deepEqual(snapshot.audio_events, []);
});

test("applySnapshotFromMessage stores source statuses in snapshot", () => {
  const snapshot = applySnapshotFromMessage(emptySnapshot(), sourceStatusMessage());

  assert.equal(snapshot.latest_event_id, "source_status:2026-05-09T09:12:00Z");
  assert.deepEqual(snapshot.source_statuses, [
    {
      source: "financial_juice",
      enabled: true,
      status: "active",
      updated_at: "2026-05-09T09:12:00Z",
    },
    {
      source: "financial_juice_browser",
      enabled: false,
      status: "blocked",
      updated_at: "2026-05-09T09:12:00Z",
    },
  ]);
  assert.deepEqual(snapshot.calendar_day_summary, {
    local_day: "2026-05-09",
    status: "audio_ready",
    audio_available: true,
    updated_at: "2026-05-09T09:12:00Z",
  });
});

test("applySnapshotFromMessage keeps a bounded reconnect audio event history", () => {
  let snapshot = emptySnapshot();

  for (let index = 0; index < AUDIO_EVENT_HISTORY_LIMIT + 5; index += 1) {
    snapshot = applySnapshotFromMessage(snapshot, audioReadyMessage(index));
  }

  assert.equal(snapshot.audio_events.length, AUDIO_EVENT_HISTORY_LIMIT);
  assert.equal(snapshot.audio_events[0].payload.translation_id, 5);

  const duplicated = audioReadyMessage(AUDIO_EVENT_HISTORY_LIMIT + 4);
  snapshot = applySnapshotFromMessage(snapshot, duplicated);

  assert.equal(snapshot.audio_events.length, AUDIO_EVENT_HISTORY_LIMIT);
  assert.equal(
    snapshot.audio_events[snapshot.audio_events.length - 1].payload.event_id,
    duplicated.payload.event_id
  );
});

test("applySnapshotFromMessage keeps a bounded reconnect translated item history", () => {
  let snapshot = emptySnapshot();

  for (let index = 0; index < TRANSLATED_NEWS_ITEM_HISTORY_LIMIT + 5; index += 1) {
    snapshot = applySnapshotFromMessage(snapshot, translatedNewsItemMessage(index));
  }

  assert.equal(snapshot.translated_news_items.length, TRANSLATED_NEWS_ITEM_HISTORY_LIMIT);
  assert.equal(snapshot.translated_news_items[0].payload.item.id, 5);
  assert.deepEqual(snapshot.audio_events, []);

  const duplicated = translatedNewsItemMessage(TRANSLATED_NEWS_ITEM_HISTORY_LIMIT + 4);
  snapshot = applySnapshotFromMessage(snapshot, duplicated);

  assert.equal(snapshot.translated_news_items.length, TRANSLATED_NEWS_ITEM_HISTORY_LIMIT);
  assert.equal(
    snapshot.translated_news_items[snapshot.translated_news_items.length - 1].payload.event_id,
    duplicated.payload.event_id
  );
});

test("buildHelloMessage exposes protocol version and snapshot", () => {
  const hello = buildHelloMessage(
    {
      latest_event_id: "evt-003",
      now_playing: null,
      audio_state: null,
    },
    "2026-05-03T09:00:03Z"
  );

  assert.equal(hello.type, "hello");
  assert.equal(hello.payload.protocol_version, 1);
  assert.equal(hello.payload.server_time, "2026-05-03T09:00:03Z");
  assert.equal(hello.payload.snapshot.latest_event_id, "evt-003");
  assert.deepEqual(hello.payload.snapshot.translated_news_items, []);
  assert.deepEqual(hello.payload.snapshot.audio_events, []);
  assert.deepEqual(hello.payload.snapshot.source_statuses, []);
  assert.equal(hello.payload.snapshot.calendar_day_summary, null);
});

test("buildHeartbeatMessage exposes compact source statuses", () => {
  const heartbeat = buildHeartbeatMessage(
    sourceStatusMessage().payload.sources,
    sourceStatusMessage().payload.calendar_day_summary,
    "2026-05-09T09:13:00Z"
  );

  assert.equal(heartbeat.type, "heartbeat");
  assert.equal(heartbeat.payload.server_time, "2026-05-09T09:13:00Z");
  assert.equal(heartbeat.payload.sources[0].source, "financial_juice");
  assert.equal(heartbeat.payload.calendar_day_summary.status, "audio_ready");
  assert.equal(heartbeat.payload.calendar_day_summary.audio_available, true);
});

test("assertPublishAuthorized accepts a fresh HMAC-signed body", async () => {
  const secret = randomBytes(32).toString("hex");
  const rawBody = JSON.stringify(sourceStatusMessage());
  const timestamp = "2026-05-09T09:12:00.000Z";
  const signature = await buildPublishSignature(secret, timestamp, rawBody);
  const request = publishRequest(rawBody, timestamp, signature);

  const result = await assertPublishAuthorized(
    request,
    { FILFINANCE_EDGE_PUBLISH_SECRET: secret },
    rawBody,
    Date.parse(timestamp)
  );

  assert.equal(result.ok, true);
});

test("assertPublishAuthorized rejects stale publish signatures", async () => {
  const secret = randomBytes(32).toString("hex");
  const rawBody = JSON.stringify(sourceStatusMessage());
  const timestamp = "2026-05-09T09:12:00.000Z";
  const signature = await buildPublishSignature(secret, timestamp, rawBody);
  const request = publishRequest(rawBody, timestamp, signature);

  const result = await assertPublishAuthorized(
    request,
    { FILFINANCE_EDGE_PUBLISH_SECRET: secret },
    rawBody,
    Date.parse(timestamp) + PUBLISH_SIGNATURE_MAX_SKEW_MS + 1
  );

  assert.equal(result.ok, false);
  assert.match(result.error, /stale/);
});

test("assertPublishAuthorized rejects tampered publish bodies", async () => {
  const secret = randomBytes(32).toString("hex");
  const rawBody = JSON.stringify(sourceStatusMessage());
  const timestamp = "2026-05-09T09:12:00.000Z";
  const signature = await buildPublishSignature(secret, timestamp, rawBody);
  const request = publishRequest(rawBody, timestamp, signature);

  const result = await assertPublishAuthorized(
    request,
    { FILFINANCE_EDGE_PUBLISH_SECRET: secret },
    rawBody.replace("financial_juice", "tampered"),
    Date.parse(timestamp)
  );

  assert.equal(result.ok, false);
  assert.match(result.error, /signature/);
});

test("assertPublishAuthorized rejects weak publish secret configuration", async () => {
  const rawBody = JSON.stringify(sourceStatusMessage());
  const timestamp = "2026-05-09T09:12:00.000Z";
  const request = publishRequest(rawBody, timestamp, "sha256=unused");

  const result = await assertPublishAuthorized(
    request,
    { FILFINANCE_EDGE_PUBLISH_SECRET: "short" },
    rawBody,
    Date.parse(timestamp)
  );

  assert.equal(result.ok, false);
  assert.equal(result.status, 500);
});

test("verifyEdgeLiveTicket accepts a valid short-lived ticket", async () => {
  const secret = randomBytes(32).toString("hex");
  const ticket = await signTestTicket(
    {
      sub: "user-1",
      email: "listener@example.test",
      role: "listener",
      scope: "live_metadata",
      iat: 1_777_800_000,
      exp: 1_777_800_120,
    },
    secret
  );

  const result = await verifyEdgeLiveTicket(ticket, secret, 1_777_800_060);

  assert.equal(result.ok, true);
  assert.equal(result.claims.sub, "user-1");
  assert.equal(result.claims.role, "listener");
});

test("verifyEdgeLiveTicket rejects expired tickets", async () => {
  const secret = randomBytes(32).toString("hex");
  const ticket = await signTestTicket(
    {
      sub: "user-1",
      role: "listener",
      scope: "live_metadata",
      iat: 1_777_800_000,
      exp: 1_777_800_010,
    },
    secret
  );

  const result = await verifyEdgeLiveTicket(ticket, secret, 1_777_800_060);

  assert.equal(result.ok, false);
  assert.match(result.error, /expired/);
});

test("verifyEdgeLiveTicket rejects invalid signatures", async () => {
  const ticket = await signTestTicket(
    {
      sub: "user-1",
      role: "listener",
      scope: "live_metadata",
      iat: 1_777_800_000,
      exp: 1_777_800_120,
    },
    "01234567890123456789012345678901"
  );

  const result = await verifyEdgeLiveTicket(ticket, "different-secret-012345678901234", 1_777_800_060);

  assert.equal(result.ok, false);
  assert.match(result.error, /signature/);
});

test("verifyEdgeLiveTicket rejects invalid scope", async () => {
  const secret = randomBytes(32).toString("hex");
  const ticket = await signTestTicket(
    {
      sub: "user-1",
      role: "listener",
      scope: "admin",
      iat: 1_777_800_000,
      exp: 1_777_800_120,
    },
    secret
  );

  const result = await verifyEdgeLiveTicket(ticket, secret, 1_777_800_060);

  assert.equal(result.ok, false);
  assert.match(result.error, /scope/);
});

function publishRequest(rawBody, timestamp, signature) {
  return new Request("https://live.filfinance.test/publish/live", {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      "X-FilFinance-Timestamp": timestamp,
      "X-FilFinance-Signature": signature,
    },
    body: rawBody,
  });
}

async function signTestTicket(claims, secret) {
  const header = {
    alg: "HS256",
    typ: "JWT",
  };
  const signingInput = `${base64UrlJson(header)}.${base64UrlJson(claims)}`;
  const key = await crypto.subtle.importKey(
    "raw",
    new TextEncoder().encode(secret),
    { name: "HMAC", hash: "SHA-256" },
    false,
    ["sign"]
  );
  const signature = await crypto.subtle.sign("HMAC", key, new TextEncoder().encode(signingInput));

  return `${signingInput}.${bytesToBase64Url(new Uint8Array(signature))}`;
}

function base64UrlJson(value) {
  return bytesToBase64Url(new TextEncoder().encode(JSON.stringify(value)));
}

function bytesToBase64Url(bytes) {
  let binary = "";
  for (const byte of bytes) {
    binary += String.fromCharCode(byte);
  }

  return btoa(binary).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/g, "");
}

function sourceStatusMessage() {
  return {
    type: "source_status",
    payload: {
      event_id: "source_status:2026-05-09T09:12:00Z",
      server_time: "2026-05-09T09:12:00Z",
      sources: [
        {
          source: "financial_juice",
          enabled: true,
          status: "active",
          updated_at: "2026-05-09T09:12:00Z",
        },
        {
          source: "financial_juice_browser",
          enabled: false,
          status: "blocked",
          updated_at: "2026-05-09T09:12:00Z",
        },
      ],
      calendar_day_summary: {
        local_day: "2026-05-09",
        status: "audio_ready",
        audio_available: true,
        updated_at: "2026-05-09T09:12:00Z",
      },
    },
  };
}

function audioReadyMessage(index) {
  const padded = String(index).padStart(20, "0");
  const streamId = `clip-20260504-${padded}`;

  return {
    type: "audio_ready",
    payload: {
      event_id: `audio_ready:${padded}`,
      server_time: "2026-05-04T08:00:00Z",
      translation_id: index,
      news_event_id: index + 1000,
      timestamp: "2026-05-04T07:59:58Z",
      stream_id: streamId,
      playlist_url: `/audio/clips/${streamId}/playlist.m3u8`,
      published_at: "2026-05-04T08:00:00Z",
      target_duration_secs: 2,
    },
  };
}

function translatedNewsItemMessage(index) {
  const padded = String(index).padStart(20, "0");

  return {
    type: "translated_news_item",
    payload: {
      event_id: `translated_news_item:${padded}`,
      server_time: "2026-05-04T08:00:00Z",
      item: {
        id: index,
        news_event_id: index + 1000,
        language: "fr",
        status: "ready",
        source: "financial_juice",
        event_kind: "headline",
        source_title: `Source ${index}`,
        source_body: null,
        translated_title: `Titre ${index}`,
        translated_body: null,
        narration_text: `Titre ${index}`,
        translation_mode: "opus_mt",
        error_message: null,
        news_level: null,
        url: null,
        timestamp: "2026-05-04T08:00:00Z",
        created_at: "2026-05-04T08:00:00Z",
        updated_at: "2026-05-04T08:00:00Z",
      },
    },
  };
}
