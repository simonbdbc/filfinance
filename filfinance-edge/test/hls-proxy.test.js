import assert from "node:assert/strict";
import test from "node:test";

import {
  buildHlsOriginUrl,
  handleHlsProxyRequest,
  hlsCacheability,
  hlsPlaylistCacheTtlSeconds,
  hlsProxyEnabled,
  isAllowedHlsOriginPath,
  isCacheableHlsSegmentPath,
  isHlsProxyPath,
  rewriteHlsPlaylistForProxy,
} from "../src/hls-proxy.js";

const env = {
  FILFINANCE_HLS_ORIGIN_BASE_URL: "https://filfinance-api.example.test",
};

const playlistCacheEnv = {
  ...env,
  FILFINANCE_HLS_PROXY_ENABLED: "true",
  FILFINANCE_HLS_PLAYLIST_CACHE_TTL_SECONDS: "2",
};

test("hlsProxyEnabled is closed by default and only opens on true", () => {
  assert.equal(hlsProxyEnabled({}), false);
  assert.equal(hlsProxyEnabled({ FILFINANCE_HLS_PROXY_ENABLED: "false" }), false);
  assert.equal(hlsProxyEnabled({ FILFINANCE_HLS_PROXY_ENABLED: "1" }), false);
  assert.equal(hlsProxyEnabled({ FILFINANCE_HLS_PROXY_ENABLED: "true" }), true);
  assert.equal(hlsProxyEnabled({ FILFINANCE_HLS_PROXY_ENABLED: " TRUE " }), true);
});

test("handleHlsProxyRequest returns Gone when the HLS proxy is disabled", async () => {
  const response = await handleHlsProxyRequest(
    new Request("https://edge.example.test/hls/audio/replay/today/playlist.m3u8"),
    env,
  );

  assert.equal(response.status, 410);
  assert.equal(response.headers.get("cache-control"), "no-store");
  assert.equal(response.headers.get("x-filfinance-hls-proxy-cache"), null);
  assert.match(await response.text(), /disabled/u);
});

test("handleHlsProxyRequest keeps playlist proxy behavior when enabled", async () => {
  const previousCaches = globalThis.caches;
  const previousFetch = globalThis.fetch;
  const cacheEntries = new Map();

  globalThis.caches = {
    default: {
      match: async (request) => cacheEntries.get(request.url) ?? null,
      put: async (request, response) => {
        cacheEntries.set(request.url, response);
      },
    },
  };
  globalThis.fetch = async () =>
    new Response("#EXTM3U\n/audio/replay/today/segments/2026-05-10/1.ts?hls_ticket=abc\n", {
      status: 200,
      headers: {
        "Content-Type": "application/vnd.apple.mpegurl",
      },
    });

  try {
    const response = await handleHlsProxyRequest(
      new Request(
        "https://edge.example.test/hls/audio/replay/today/playlist.m3u8?hls_ticket=abc",
      ),
      playlistCacheEnv,
    );

    assert.equal(response.status, 200);
    assert.equal(response.headers.get("x-filfinance-hls-proxy-cache"), "PLAYLIST_MISS");
    assert.match(await response.text(), /\/hls\/audio\/replay\/today\/segments/u);
  } finally {
    globalThis.caches = previousCaches;
    globalThis.fetch = previousFetch;
  }
});

test("isHlsProxyPath matches only the HLS proxy prefix", () => {
  assert.equal(isHlsProxyPath("/hls/audio/replay/today/playlist.m3u8"), true);
  assert.equal(isHlsProxyPath("/hls"), true);
  assert.equal(isHlsProxyPath("/audio/replay/today/playlist.m3u8"), false);
  assert.equal(isHlsProxyPath("/health"), false);
});

test("buildHlsOriginUrl strips the proxy prefix and preserves the ticket query", () => {
  const result = buildHlsOriginUrl(
    "https://edge.example.test/hls/audio/replay/today/segments/2026-05-10/segment-001.ts?hls_ticket=abc",
    env,
  );

  assert.equal(result.ok, true);
  assert.equal(
    result.originUrl,
    "https://filfinance-api.example.test/audio/replay/today/segments/2026-05-10/segment-001.ts?hls_ticket=abc",
  );
});

test("buildHlsOriginUrl allows clip playlists", () => {
  const result = buildHlsOriginUrl(
    "https://edge.example.test/hls/audio/clips/audio-123/playlist.m3u8?hls_ticket=abc",
    env,
  );

  assert.equal(result.ok, true);
  assert.equal(
    result.originUrl,
    "https://filfinance-api.example.test/audio/clips/audio-123/playlist.m3u8?hls_ticket=abc",
  );
});

test("buildHlsOriginUrl allows calendar summary current playlists", () => {
  const result = buildHlsOriginUrl(
    "https://edge.example.test/hls/audio/calendar-summary/2026-06-15/current.m3u8?hls_ticket=abc",
    env,
  );

  assert.equal(result.ok, true);
  assert.equal(
    result.originUrl,
    "https://filfinance-api.example.test/audio/calendar-summary/2026-06-15/current.m3u8?hls_ticket=abc",
  );
});

test("isAllowedHlsOriginPath rejects traversal and unrelated audio routes", () => {
  assert.equal(isAllowedHlsOriginPath("/audio/replay/today/segments/../secret.ts"), false);
  assert.equal(isAllowedHlsOriginPath("/audio/live/segments/1.ts"), false);
  assert.equal(isAllowedHlsOriginPath("/api/news"), false);
  assert.equal(
    isAllowedHlsOriginPath("/audio/calendar-summary/2026-06-15/audio-1/segment-000.ts"),
    true,
  );
});

test("isCacheableHlsSegmentPath caches only immutable segments", () => {
  assert.equal(
    isCacheableHlsSegmentPath("/audio/replay/today/segments/2026-05-10/segment-001.ts"),
    true,
  );
  assert.equal(isCacheableHlsSegmentPath("/audio/clips/audio-123/segment-001.ts"), true);
  assert.equal(
    isCacheableHlsSegmentPath("/audio/calendar-summary/2026-06-15/audio-123/segment-001.ts"),
    true,
  );
  assert.equal(isCacheableHlsSegmentPath("/audio/clips/audio-123/playlist.m3u8"), false);
  assert.equal(
    isCacheableHlsSegmentPath("/audio/calendar-summary/2026-06-15/current.m3u8"),
    false,
  );
  assert.equal(isCacheableHlsSegmentPath("/audio/replay/today/playlist.m3u8"), false);
});

test("hlsCacheability bypasses playlists when playlist cache is disabled", () => {
  assert.deepEqual(
    hlsCacheability(new Request("https://edge.example.test"), "/audio/replay/today/playlist.m3u8"),
    { cacheable: false, cacheStatus: "BYPASS_PLAYLIST_DISABLED" },
  );
});

test("hlsCacheability caches playlists when a short TTL is configured", () => {
  assert.deepEqual(
    hlsCacheability(
      new Request("https://edge.example.test"),
      "/audio/replay/today/playlist.m3u8",
      playlistCacheEnv,
    ),
    {
      cacheable: true,
      kind: "playlist",
      cacheHitStatus: "PLAYLIST_HIT",
      cacheMissStatus: "PLAYLIST_MISS",
      cacheTtlSeconds: 2,
    },
  );
});

test("hlsCacheability bypasses HEAD and Range requests", () => {
  const segmentPath = "/audio/replay/today/segments/2026-05-10/segment-001.ts";
  assert.deepEqual(hlsCacheability(new Request("https://edge.example.test"), segmentPath), {
    cacheable: true,
    kind: "segment",
    cacheHitStatus: "HIT",
    cacheMissStatus: "MISS",
  });
  assert.deepEqual(
    hlsCacheability(new Request("https://edge.example.test", { method: "HEAD" }), segmentPath),
    { cacheable: false, cacheStatus: "BYPASS_METHOD" },
  );
  assert.deepEqual(
    hlsCacheability(
      new Request("https://edge.example.test", {
        headers: {
          Range: "bytes=0-100",
        },
      }),
      segmentPath,
    ),
    { cacheable: false, cacheStatus: "BYPASS_RANGE" },
  );
});

test("hlsPlaylistCacheTtlSeconds defaults off and clamps high values", () => {
  assert.equal(hlsPlaylistCacheTtlSeconds({}), 0);
  assert.equal(hlsPlaylistCacheTtlSeconds({ FILFINANCE_HLS_PLAYLIST_CACHE_TTL_SECONDS: "0" }), 0);
  assert.equal(hlsPlaylistCacheTtlSeconds({ FILFINANCE_HLS_PLAYLIST_CACHE_TTL_SECONDS: "abc" }), 0);
  assert.equal(hlsPlaylistCacheTtlSeconds({ FILFINANCE_HLS_PLAYLIST_CACHE_TTL_SECONDS: "2" }), 2);
  assert.equal(hlsPlaylistCacheTtlSeconds({ FILFINANCE_HLS_PLAYLIST_CACHE_TTL_SECONDS: "30" }), 5);
});

test("rewriteHlsPlaylistForProxy maps origin segment paths back through the Worker proxy", () => {
  const playlist = [
    "#EXTM3U",
    "#EXTINF:4.000,",
    "/audio/replay/today/segments/2026-05-10/segment-001.ts?hls_ticket=abc",
    "https://filfinance-api.example.test/audio/replay/today/segments/2026-05-10/segment-002.ts?hls_ticket=abc",
    "#EXT-X-ENDLIST",
  ].join("\n");

  assert.equal(
    rewriteHlsPlaylistForProxy(
      playlist,
      "https://filfinance-api.example.test/audio/replay/today/playlist.m3u8?hls_ticket=abc",
    ),
    [
      "#EXTM3U",
      "#EXTINF:4.000,",
      "/hls/audio/replay/today/segments/2026-05-10/segment-001.ts?hls_ticket=abc",
      "/hls/audio/replay/today/segments/2026-05-10/segment-002.ts?hls_ticket=abc",
      "#EXT-X-ENDLIST",
    ].join("\n"),
  );
});

test("rewriteHlsPlaylistForProxy maps relative clip segment paths through the Worker proxy", () => {
  const playlist = ["#EXTM3U", "segment-001.ts?hls_ticket=abc"].join("\n");

  assert.equal(
    rewriteHlsPlaylistForProxy(
      playlist,
      "https://filfinance-api.example.test/audio/clips/audio-123/playlist.m3u8?hls_ticket=abc",
    ),
    ["#EXTM3U", "/hls/audio/clips/audio-123/segment-001.ts?hls_ticket=abc"].join("\n"),
  );
});

test("rewriteHlsPlaylistForProxy maps calendar summary segments through the Worker proxy", () => {
  const playlist = [
    "#EXTM3U",
    "#EXTINF:2.000,",
    "/audio/calendar-summary/2026-06-15/audio-123/segment-000.ts?hls_ticket=abc",
    "#EXT-X-ENDLIST",
  ].join("\n");

  assert.equal(
    rewriteHlsPlaylistForProxy(
      playlist,
      "https://filfinance-api.example.test/audio/calendar-summary/2026-06-15/current.m3u8?hls_ticket=abc",
    ),
    [
      "#EXTM3U",
      "#EXTINF:2.000,",
      "/hls/audio/calendar-summary/2026-06-15/audio-123/segment-000.ts?hls_ticket=abc",
      "#EXT-X-ENDLIST",
    ].join("\n"),
  );
});
