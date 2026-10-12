#!/usr/bin/env node

const listenerToken = process.env.FILFINANCE_LISTENER_ACCESS_TOKEN ?? "";
const moderatorToken = process.env.FILFINANCE_MODERATOR_ACCESS_TOKEN ?? "";
const adminToken = process.env.FILFINANCE_ADMIN_ACCESS_TOKEN ?? "";
const timeoutMs = Number.parseInt(process.env.FILFINANCE_WS_SMOKE_TIMEOUT_MS ?? "5000", 10);

if (!listenerToken.trim()) {
  console.error("FILFINANCE_LISTENER_ACCESS_TOKEN is required");
  process.exit(2);
}
if (!moderatorToken.trim()) {
  console.error("FILFINANCE_MODERATOR_ACCESS_TOKEN is required");
  process.exit(2);
}
if (!adminToken.trim()) {
  console.error("FILFINANCE_ADMIN_ACCESS_TOKEN is required");
  process.exit(2);
}

if (typeof WebSocket !== "function") {
  console.error("A Node.js runtime with global WebSocket support is required");
  process.exit(2);
}

function deriveWsBaseUrl() {
  const explicitBaseUrl = process.env.FILFINANCE_WS_BASE_URL;

  if (explicitBaseUrl?.trim()) {
    return explicitBaseUrl.trim().replace(/\/$/, "");
  }

  const httpBaseUrl = process.env.FILFINANCE_API_BASE_URL ?? "http://127.0.0.1:8088";
  const parsedUrl = new URL(httpBaseUrl);
  parsedUrl.protocol = parsedUrl.protocol === "https:" ? "wss:" : "ws:";
  parsedUrl.pathname = "";
  parsedUrl.search = "";
  parsedUrl.hash = "";

  return parsedUrl.toString().replace(/\/$/, "");
}

function websocketUrl(path) {
  return `${deriveWsBaseUrl()}${path}`;
}

function authPayload(token) {
  return JSON.stringify({
    type: "auth",
    access_token: token,
  });
}

function eventDataToString(data) {
  if (typeof data === "string") {
    return data;
  }

  if (data instanceof ArrayBuffer) {
    return Buffer.from(data).toString("utf8");
  }

  if (ArrayBuffer.isView(data)) {
    return Buffer.from(data.buffer, data.byteOffset, data.byteLength).toString("utf8");
  }

  return String(data);
}

function waitForFirstSocketResult(path, token) {
  return new Promise((resolve, reject) => {
    const socket = new WebSocket(websocketUrl(path));
    let settled = false;

    const timer = setTimeout(() => {
      if (settled) {
        return;
      }

      settled = true;
      socket.close();
      reject(new Error(`${path} timed out after ${timeoutMs}ms`));
    }, timeoutMs);

    function finish(result) {
      if (settled) {
        return;
      }

      settled = true;
      clearTimeout(timer);
      resolve(result);
    }

    socket.addEventListener("open", () => {
      socket.send(authPayload(token));
    });

    socket.addEventListener("message", (event) => {
      const text = eventDataToString(event.data);
      socket.close();
      finish({ kind: "message", text });
    });

    socket.addEventListener("close", (event) => {
      finish({
        kind: "close",
        code: event.code,
        reason: event.reason,
      });
    });

    socket.addEventListener("error", () => {
      if (!settled) {
        settled = true;
        clearTimeout(timer);
        reject(new Error(`${path} websocket transport error`));
      }
    });
  });
}

function parseJsonMessage(label, text) {
  try {
    return JSON.parse(text);
  } catch (error) {
    throw new Error(`${label}: first message is not valid JSON: ${error.message}`);
  }
}

async function expectHello(label, path, token) {
  const result = await waitForFirstSocketResult(path, token);

  if (result.kind !== "message") {
    throw new Error(`${label}: expected hello message, got close ${result.code} ${result.reason}`);
  }

  const payload = parseJsonMessage(label, result.text);

  if (payload.type !== "hello") {
    throw new Error(`${label}: expected type=hello, got ${payload.type ?? "<missing>"}`);
  }

  console.log(`OK ${label}: hello`);
}

async function expectPolicyClose(label, path, token, expectedReason) {
  const result = await waitForFirstSocketResult(path, token);

  if (result.kind !== "close") {
    throw new Error(`${label}: expected policy close, got message ${result.text}`);
  }

  if (result.code !== 1008) {
    throw new Error(`${label}: expected close 1008, got ${result.code} ${result.reason}`);
  }

  if (expectedReason && !result.reason.includes(expectedReason)) {
    throw new Error(`${label}: expected reason containing ${expectedReason}, got ${result.reason}`);
  }

  console.log(`OK ${label}: close ${result.code} ${result.reason}`);
}

async function main() {
  await expectHello("ws/news accepts listener", "/ws/news", listenerToken);
  await expectPolicyClose(
    "ws/test-runs rejects listener",
    "/ws/test-runs",
    listenerToken,
    "insufficient role",
  );

  await expectHello("ws/test-runs accepts moderator", "/ws/test-runs", moderatorToken);
  await expectHello("ws/test-runs accepts admin", "/ws/test-runs", adminToken);

  console.log(`Better Auth WebSocket role smoke test completed against ${deriveWsBaseUrl()}`);
}

main().catch((error) => {
  console.error(`FAIL ${error.message}`);
  process.exit(1);
});
