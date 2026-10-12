export const HLS_PROXY_PREFIX = "/hls";
export const DEFAULT_HLS_ORIGIN_BASE_URL = "https://api.filfinance.fr";
export const MAX_HLS_PLAYLIST_CACHE_TTL_SECONDS = 5;
export const HLS_PROXY_DISABLED_MESSAGE =
  "FilFinance HLS proxy is disabled pending a compliant media delivery target.";

const CORS_HEADERS = {
  "Access-Control-Allow-Origin": "*",
  "Access-Control-Allow-Methods": "GET, HEAD, OPTIONS",
  "Access-Control-Allow-Headers": "Range, Origin, Accept, Content-Type",
  "Access-Control-Expose-Headers":
    "Content-Length, Content-Type, Cache-Control, CF-Cache-Status, X-FilFinance-HLS-Proxy-Cache",
};

export function isHlsProxyPath(pathname) {
  return pathname === HLS_PROXY_PREFIX || pathname.startsWith(`${HLS_PROXY_PREFIX}/`);
}

export async function handleHlsProxyRequest(request, env) {
  if (!hlsProxyEnabled(env)) {
    return hlsProxyError(HLS_PROXY_DISABLED_MESSAGE, 410, {
      "Cache-Control": "no-store",
    });
  }

  if (request.method === "OPTIONS") {
    return new Response(null, {
      status: 204,
      headers: CORS_HEADERS,
    });
  }

  if (request.method !== "GET" && request.method !== "HEAD") {
    return hlsProxyError("Method Not Allowed", 405, {
      Allow: "GET, HEAD, OPTIONS",
    });
  }

  const origin = buildHlsOriginUrl(request, env);
  if (!origin.ok) {
    return hlsProxyError(origin.error, origin.status);
  }

  const cacheability = hlsCacheability(request, origin.originPath, env);
  if (!cacheability.cacheable) {
    const response = await fetchOriginHls(request, origin.originUrl);
    if (request.method === "GET" && response.ok && isHlsPlaylistPath(origin.originPath)) {
      return rewriteHlsPlaylistResponse(response, {
        cacheStatus: cacheability.cacheStatus,
        originPlaylistUrl: origin.originUrl,
      });
    }
    return withHlsProxyHeaders(response, cacheability.cacheStatus);
  }

  const cache = caches.default;
  const cacheKey = new Request(request.url, { method: "GET" });
  const cachedResponse = await cache.match(cacheKey);
  if (cachedResponse) {
    return withHlsProxyHeaders(cachedResponse, cacheability.cacheHitStatus);
  }

  const originResponse = await fetchOriginHls(request, origin.originUrl);
  if (cacheability.kind === "playlist") {
    if (!originResponse.ok) {
      return withHlsProxyHeaders(originResponse, "BYPASS_ORIGIN_ERROR");
    }

    const response = await rewriteHlsPlaylistResponse(originResponse, {
      cacheStatus: cacheability.cacheMissStatus,
      originPlaylistUrl: origin.originUrl,
      cacheTtlSeconds: cacheability.cacheTtlSeconds,
    });
    await cache.put(cacheKey, response.clone());
    return response;
  }

  const response = withHlsProxyHeaders(originResponse, cacheability.cacheMissStatus);
  if (originResponse.ok) {
    await cache.put(cacheKey, response.clone());
  }

  return response;
}

export function hlsProxyEnabled(env = {}) {
  return String(env.FILFINANCE_HLS_PROXY_ENABLED ?? "").trim().toLowerCase() === "true";
}

export function buildHlsOriginUrl(request, env = {}) {
  const requestUrl = new URL(typeof request === "string" ? request : request.url);
  if (!isHlsProxyPath(requestUrl.pathname)) {
    return { ok: false, status: 404, error: "not an HLS proxy route" };
  }

  const originPath = requestUrl.pathname.slice(HLS_PROXY_PREFIX.length) || "/";
  if (!isAllowedHlsOriginPath(originPath)) {
    return { ok: false, status: 404, error: "HLS route is not allowed" };
  }

  const originBase = normalizeOriginBaseUrl(env.FILFINANCE_HLS_ORIGIN_BASE_URL);
  if (!originBase.ok) {
    return originBase;
  }

  const originUrl = new URL(`${originPath}${requestUrl.search}`, originBase.value);
  return {
    ok: true,
    originPath,
    originUrl: originUrl.toString(),
  };
}

export function isAllowedHlsOriginPath(originPath) {
  const decodedPath = safeDecodeURIComponent(originPath);
  if (!decodedPath || decodedPath.includes("..")) {
    return false;
  }

  if (originPath === "/audio/replay/today/playlist.m3u8") {
    return true;
  }

  if (originPath.startsWith("/audio/replay/today/segments/")) {
    return originPath.endsWith(".ts");
  }

  if (originPath.startsWith("/audio/calendar-summary/")) {
    return originPath.endsWith("/playlist.m3u8")
      || originPath.endsWith("/current.m3u8")
      || originPath.endsWith(".ts");
  }

  if (!originPath.startsWith("/audio/clips/")) {
    return false;
  }

  return originPath.endsWith("/playlist.m3u8") || originPath.endsWith(".ts");
}

export function hlsCacheability(request, originPath, env = {}) {
  if (request.method !== "GET") {
    return { cacheable: false, cacheStatus: "BYPASS_METHOD" };
  }

  if (request.headers.has("Range")) {
    return { cacheable: false, cacheStatus: "BYPASS_RANGE" };
  }

  if (isHlsPlaylistPath(originPath)) {
    const playlistCacheTtlSeconds = hlsPlaylistCacheTtlSeconds(env);
    if (playlistCacheTtlSeconds <= 0) {
      return { cacheable: false, cacheStatus: "BYPASS_PLAYLIST_DISABLED" };
    }

    return {
      cacheable: true,
      kind: "playlist",
      cacheHitStatus: "PLAYLIST_HIT",
      cacheMissStatus: "PLAYLIST_MISS",
      cacheTtlSeconds: playlistCacheTtlSeconds,
    };
  }

  if (!isCacheableHlsSegmentPath(originPath)) {
    return { cacheable: false, cacheStatus: "BYPASS_UNCACHEABLE" };
  }

  return {
    cacheable: true,
    kind: "segment",
    cacheHitStatus: "HIT",
    cacheMissStatus: "MISS",
  };
}

export function hlsPlaylistCacheTtlSeconds(env = {}) {
  const rawValue = String(env.FILFINANCE_HLS_PLAYLIST_CACHE_TTL_SECONDS ?? "").trim();
  if (!rawValue) {
    return 0;
  }

  const parsed = Number.parseInt(rawValue, 10);
  if (!Number.isFinite(parsed) || parsed <= 0) {
    return 0;
  }

  return Math.min(parsed, MAX_HLS_PLAYLIST_CACHE_TTL_SECONDS);
}

export function isCacheableHlsSegmentPath(originPath) {
  if (!originPath.endsWith(".ts")) {
    return false;
  }

  return (
    originPath.startsWith("/audio/replay/today/segments/")
    || originPath.startsWith("/audio/calendar-summary/")
    || originPath.startsWith("/audio/clips/")
  );
}

export function isHlsPlaylistPath(originPath) {
  return (
    originPath === "/audio/replay/today/playlist.m3u8"
    || (originPath.startsWith("/audio/calendar-summary/")
      && (originPath.endsWith("/playlist.m3u8") || originPath.endsWith("/current.m3u8")))
    || (originPath.startsWith("/audio/clips/") && originPath.endsWith("/playlist.m3u8"))
  );
}

export function rewriteHlsPlaylistForProxy(playlistText, originPlaylistUrl) {
  return String(playlistText)
    .split(/\r?\n/u)
    .map((line) => rewriteHlsPlaylistLineForProxy(line, originPlaylistUrl))
    .join("\n");
}

async function fetchOriginHls(request, originUrl) {
  return fetch(originUrl, {
    method: request.method,
    headers: originRequestHeaders(request),
    redirect: "follow",
  });
}

function originRequestHeaders(request) {
  const headers = new Headers();
  copyHeader(request, headers, "Accept");
  copyHeader(request, headers, "Range");
  return headers;
}

function copyHeader(request, headers, name) {
  const value = request.headers.get(name);
  if (value) {
    headers.set(name, value);
  }
}

async function rewriteHlsPlaylistResponse(
  response,
  { cacheStatus, originPlaylistUrl, cacheTtlSeconds = 0 },
) {
  const rewrittenPlaylist = rewriteHlsPlaylistForProxy(
    await response.text(),
    originPlaylistUrl,
  );
  const headers = new Headers(response.headers);
  headers.delete("Content-Length");
  if (cacheTtlSeconds > 0) {
    headers.set("Cache-Control", `public, max-age=${cacheTtlSeconds}, must-revalidate`);
    headers.set("Cloudflare-CDN-Cache-Control", `max-age=${cacheTtlSeconds}`);
  }

  return withHlsProxyHeaders(
    new Response(rewrittenPlaylist, {
      status: response.status,
      statusText: response.statusText,
      headers,
    }),
    cacheStatus,
  );
}

function rewriteHlsPlaylistLineForProxy(line, originPlaylistUrl) {
  const trimmed = line.trim();
  if (!trimmed || trimmed.startsWith("#")) {
    return line;
  }

  let segmentUrl;
  try {
    segmentUrl = new URL(trimmed, originPlaylistUrl);
  } catch {
    return line;
  }

  if (!isAllowedHlsOriginPath(segmentUrl.pathname)) {
    return line;
  }

  if (segmentUrl.pathname.startsWith(`${HLS_PROXY_PREFIX}/audio/`)) {
    return `${segmentUrl.pathname}${segmentUrl.search}${segmentUrl.hash}`;
  }

  if (!segmentUrl.pathname.startsWith("/audio/")) {
    return line;
  }

  return `${HLS_PROXY_PREFIX}${segmentUrl.pathname}${segmentUrl.search}${segmentUrl.hash}`;
}

function withHlsProxyHeaders(response, cacheStatus) {
  const headers = new Headers(response.headers);
  for (const [name, value] of Object.entries(CORS_HEADERS)) {
    headers.set(name, value);
  }
  headers.set("X-FilFinance-HLS-Proxy", "1");
  headers.set("X-FilFinance-HLS-Proxy-Cache", cacheStatus);

  return new Response(response.body, {
    status: response.status,
    statusText: response.statusText,
    headers,
  });
}

function hlsProxyError(message, status, headers = {}) {
  return new Response(message, {
    status,
    headers: {
      "Content-Type": "text/plain; charset=utf-8",
      ...CORS_HEADERS,
      ...headers,
    },
  });
}

function normalizeOriginBaseUrl(value) {
  const rawValue = String(value || DEFAULT_HLS_ORIGIN_BASE_URL).trim();
  let parsed;

  try {
    parsed = new URL(rawValue);
  } catch {
    return { ok: false, status: 500, error: "HLS origin base URL is invalid" };
  }

  if (parsed.protocol !== "https:" && parsed.protocol !== "http:") {
    return { ok: false, status: 500, error: "HLS origin base URL protocol is invalid" };
  }

  parsed.pathname = "/";
  parsed.search = "";
  parsed.hash = "";
  return { ok: true, value: parsed.toString() };
}

function safeDecodeURIComponent(value) {
  try {
    return decodeURIComponent(value);
  } catch {
    return null;
  }
}
