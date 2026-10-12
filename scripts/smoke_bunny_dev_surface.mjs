#!/usr/bin/env node

const timeoutMs = Number.parseInt(process.env.FILFINANCE_BUNNY_DEV_SMOKE_TIMEOUT_MS ?? "10000", 10);

const config = {
  landingUrl: requiredUrl("FILFINANCE_FRONTEND_URL", "dev.filfinance.fr"),
  pwaUrl: requiredUrl("FILFINANCE_PWA_URL", "app.dev.filfinance.fr"),
  apiUrl: requiredUrl("FILFINANCE_API_URL", "api.dev.filfinance.fr"),
  hlsUrl: requiredUrl("FILFINANCE_HLS_URL", "media.dev.filfinance.fr"),
  liveUrl: requiredUrl("FILFINANCE_EDGE_URL", "live.dev.filfinance.fr"),
  accessToken: required("FILFINANCE_AUTH_TOKEN"),
};

if (process.env.FILFINANCE_BUNNY_DEV_SMOKE !== "1") {
  fail("FILFINANCE_BUNNY_DEV_SMOKE must equal 1");
}

if (!Number.isSafeInteger(timeoutMs) || timeoutMs <= 0) {
  fail("FILFINANCE_BUNNY_DEV_SMOKE_TIMEOUT_MS must be a positive integer");
}

async function main() {
  await expectHtml("landing", config.landingUrl);
  await expectHtml("PWA deep route", new URL("/__bunny-dev-spa-smoke", `${config.pwaUrl}/`).toString());
  await expectJsonOk("API health", new URL("/health", `${config.apiUrl}/`).toString());

  const hlsTicket = await fetchTicket("HLS", "/api/hls-ticket", "hls_ticket");
  const playlistUrl = new URL("/audio/replay/today/playlist.m3u8", `${config.hlsUrl}/`);
  playlistUrl.searchParams.set("hls_ticket", hlsTicket);
  const playlist = await fetchText("HLS playlist", playlistUrl.toString());
  if (!playlist.includes("#EXTM3U")) {
    fail("HLS playlist is not an M3U8 document");
  }
  ok("HLS ticket and playlist");

  const liveTicket = await fetchTicket("live", "/api/edge/live-ticket", "ticket");
  await expectLiveHello(liveTicket);
  ok("live WebSocket hello");
}

function required(name) {
  const value = process.env[name]?.trim();
  if (!value) {
    fail(`${name} is required`);
  }
  return value;
}

function requiredUrl(name, expectedHostname) {
  const value = required(name).replace(/\/$/, "");
  let parsed;
  try {
    parsed = new URL(value);
  } catch {
    fail(`${name} must be an HTTPS URL`);
  }
  if (parsed.protocol !== "https:" || parsed.hostname !== expectedHostname || parsed.pathname !== "/") {
    fail(`${name} must equal https://${expectedHostname}`);
  }
  return parsed.toString().replace(/\/$/, "");
}

async function fetchTicket(label, path, property) {
  const response = await fetchWithTimeout(new URL(path, `${config.apiUrl}/`).toString(), {
    headers: { Authorization: `Bearer ${config.accessToken}` },
  });
  if (!response.ok) {
    fail(`${label} ticket endpoint returned HTTP ${response.status}`);
  }
  const payload = await response.json();
  if (payload.enabled === false || typeof payload[property] !== "string" || !payload[property]) {
    fail(`${label} ticket endpoint did not return an enabled ${property}`);
  }
  return payload[property];
}

async function expectLiveHello(ticket) {
  const url = new URL(config.liveUrl);
  url.protocol = "wss:";
  url.pathname = "/ws/live";
  url.search = "";
  url.searchParams.set("ticket", ticket);

  await new Promise((resolve, reject) => {
    const socket = new WebSocket(url);
    let settled = false;
    const timer = setTimeout(() => finish(new Error("live WebSocket timed out")), timeoutMs);

    function finish(error) {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      socket.close();
      if (error) reject(error);
      else resolve();
    }

    socket.addEventListener("message", (event) => {
      try {
        const payload = JSON.parse(textFromWebSocket(event.data));
        if (payload.type !== "hello") {
          finish(new Error(`live WebSocket expected hello, got ${payload.type ?? "<missing>"}`));
          return;
        }
        finish();
      } catch (error) {
        finish(new Error(`live WebSocket sent invalid JSON: ${error.message}`));
      }
    }, { once: true });
    socket.addEventListener("error", () => finish(new Error("live WebSocket transport error")), { once: true });
    socket.addEventListener("close", (event) => {
      if (!settled) finish(new Error(`live WebSocket closed ${event.code} ${event.reason}`));
    }, { once: true });
  });
}

function textFromWebSocket(data) {
  if (typeof data === "string") return data;
  if (data instanceof ArrayBuffer) return Buffer.from(data).toString("utf8");
  if (ArrayBuffer.isView(data)) return Buffer.from(data.buffer, data.byteOffset, data.byteLength).toString("utf8");
  return String(data);
}

async function expectHtml(label, url) {
  const body = await fetchText(label, url);
  if (!/<html[\s>]/i.test(body)) {
    fail(`${label} did not return an HTML document`);
  }
  ok(label);
}

async function expectJsonOk(label, url) {
  const response = await fetchWithTimeout(url);
  if (!response.ok) fail(`${label} returned HTTP ${response.status}`);
  const payload = await response.json();
  if (payload.status !== "ok") fail(`${label} response is missing status=ok`);
  ok(label);
}

async function fetchText(label, url) {
  const response = await fetchWithTimeout(url);
  if (!response.ok) fail(`${label} returned HTTP ${response.status}`);
  return response.text();
}

function fetchWithTimeout(url, options = {}) {
  return fetch(url, { ...options, signal: AbortSignal.timeout(timeoutMs) });
}

function ok(label) {
  console.log(`OK Bunny DEV ${label}`);
}

function fail(message) {
  throw new Error(`Bunny DEV smoke: ${message}`);
}

main().catch((error) => {
  console.error(`FAIL ${error.message}`);
  process.exit(1);
});
