import { DurableObject } from "cloudflare:workers";

import { handleHlsProxyRequest, isHlsProxyPath } from "./hls-proxy.js";
import {
  applySnapshotFromMessage,
  assertPublishAuthorized,
  assertWebSocketAuthorized,
  buildHeartbeatMessage,
  buildHelloMessage,
  emptySnapshot,
  jsonResponse,
  normalizeSnapshot,
  validatePublishMessage,
} from "./protocol.js";

const LIVE_OBJECT_NAME = "filfinance-live";

export default {
  async fetch(request, env) {
    const url = new URL(request.url);

    if (url.pathname === "/health") {
      return jsonResponse({
        status: "ok",
        service: "filfinance-live-edge",
        timestamp: new Date().toISOString(),
      });
    }

    if (url.pathname === "/ws/live") {
      if (request.method !== "GET") {
        return new Response("Method Not Allowed", { status: 405 });
      }

      if (request.headers.get("Upgrade") !== "websocket") {
        return new Response("Expected Upgrade: websocket", { status: 426 });
      }

      const auth = await assertWebSocketAuthorized(request, env);
      if (!auth.ok) {
        return jsonResponse({ error: auth.error }, { status: auth.status });
      }

      return liveObjectStub(env).fetch(request);
    }

    if (url.pathname === "/publish/live") {
      if (request.method !== "POST") {
        return new Response("Method Not Allowed", { status: 405 });
      }

      const body = await request.text();
      const auth = await assertPublishAuthorized(request, env, body);
      if (!auth.ok) {
        return jsonResponse({ error: auth.error }, { status: auth.status });
      }

      return liveObjectStub(env).fetch("https://filfinance.edge/internal/publish", {
        method: "POST",
        headers: {
          "Content-Type": request.headers.get("Content-Type") ?? "application/json",
        },
        body,
      });
    }

    if (isHlsProxyPath(url.pathname)) {
      return handleHlsProxyRequest(request, env);
    }

    return jsonResponse(
      {
        service: "filfinance-live-edge",
        endpoints: ["/health", "/ws/live", "/publish/live", "/hls/audio/..."],
      },
      { status: 404 }
    );
  },
};

export class LiveMetadataDurableObject extends DurableObject {
  constructor(ctx, env) {
    super(ctx, env);
    this.ctx = ctx;
    this.env = env;
    this.sessions = new Map();
    this.snapshot = emptySnapshot();
    this.snapshotLoaded = false;

    for (const websocket of this.ctx.getWebSockets()) {
      const attachment = websocket.deserializeAttachment();
      if (attachment?.id) {
        this.sessions.set(websocket, attachment);
      }
    }

    this.ctx.setWebSocketAutoResponse(new WebSocketRequestResponsePair("ping", "pong"));
  }

  async fetch(request) {
    const url = new URL(request.url);

    if (url.pathname === "/internal/publish") {
      return this.publish(request);
    }

    if (request.headers.get("Upgrade") !== "websocket") {
      return new Response("Expected Upgrade: websocket", { status: 426 });
    }

    const pair = new WebSocketPair();
    const [client, server] = Object.values(pair);
    const session = {
      id: crypto.randomUUID(),
      connected_at: new Date().toISOString(),
    };

    this.ctx.acceptWebSocket(server);
    server.serializeAttachment(session);
    this.sessions.set(server, session);

    await this.ensureSnapshotLoaded();
    server.send(JSON.stringify(buildHelloMessage(this.snapshot)));

    return new Response(null, {
      status: 101,
      webSocket: client,
    });
  }

  async publish(request) {
    let message;

    try {
      message = await request.json();
    } catch {
      return jsonResponse({ error: "publish body must be valid JSON" }, { status: 400 });
    }

    const validation = validatePublishMessage(message);
    if (!validation.ok) {
      return jsonResponse({ error: validation.error }, { status: 400 });
    }

    await this.ensureSnapshotLoaded();
    this.snapshot = applySnapshotFromMessage(this.snapshot, validation.message);
    await this.ctx.storage.put("snapshot", this.snapshot);

    const payload = JSON.stringify(validation.message);
    let delivered = 0;
    let dropped = 0;

    for (const websocket of this.ctx.getWebSockets()) {
      try {
        websocket.send(payload);
        delivered += 1;
      } catch {
        dropped += 1;
        this.sessions.delete(websocket);
      }
    }

    return jsonResponse({
      accepted: true,
      delivered,
      dropped,
      connected: this.ctx.getWebSockets().length,
      latest_event_id: this.snapshot.latest_event_id,
    });
  }

  async webSocketMessage(websocket, message) {
    if (message === "ping") {
      websocket.send("pong");
      return;
    }

    if (message === "heartbeat") {
      websocket.send(JSON.stringify(buildHeartbeatMessage(
        this.snapshot.source_statuses,
        this.snapshot.calendar_day_summary
      )));
    }
  }

  async webSocketClose(websocket) {
    this.sessions.delete(websocket);
  }

  async webSocketError(websocket) {
    this.sessions.delete(websocket);
  }

  async ensureSnapshotLoaded() {
    if (this.snapshotLoaded) {
      return;
    }

    const storedSnapshot = await this.ctx.storage.get("snapshot");
    this.snapshot = normalizeSnapshot(storedSnapshot);
    this.snapshotLoaded = true;
  }
}

function liveObjectStub(env) {
  const objectName = env.LIVE_OBJECT_NAME || LIVE_OBJECT_NAME;
  return env.LIVE_METADATA.getByName(objectName);
}
