import assert from "node:assert/strict";
import test from "node:test";

import {
  loadControl,
  loadControls,
  publishNewsBatch,
  publishStatusToTargets,
} from "../src/publish.js";

test("loadControl reports backend endpoint and network cause", async (t) => {
  const originalFetch = globalThis.fetch;
  t.after(() => {
    globalThis.fetch = originalFetch;
  });

  const cause = Object.assign(new Error("connect ECONNREFUSED 127.0.0.1:8088"), {
    address: "127.0.0.1",
    code: "ECONNREFUSED",
    port: 8088,
  });

  globalThis.fetch = async () => {
    throw new TypeError("fetch failed", { cause });
  };

  await assert.rejects(
    loadControl({
      controlEndpoint: "http://127.0.0.1:8088/api/ingest/financial-juice/browser/control",
      ingestToken: "test-token",
    }),
    /backend GET http:\/\/127\.0\.0\.1:8088\/api\/ingest\/financial-juice\/browser\/control failed: fetch failed - ECONNREFUSED - 127\.0\.0\.1:8088 - connect ECONNREFUSED 127\.0\.0\.1:8088/,
  );
});

test("loadControls keeps one target failure isolated", async (t) => {
  const originalFetch = globalThis.fetch;
  t.after(() => {
    globalThis.fetch = originalFetch;
  });

  globalThis.fetch = async (url) => {
    if (String(url).includes("dev")) {
      throw new TypeError("fetch failed", {
        cause: Object.assign(new Error("connect ECONNREFUSED dev"), {
          address: "127.0.0.1",
          code: "ECONNREFUSED",
          port: 18088,
        }),
      });
    }
    return jsonResponse({
      fallback_active: false,
      message: "primary active",
      poll_interval_ms: 5000,
    });
  };

  const results = await loadControls({
    targets: [
      target("prod", "http://prod:8088", "prod-token"),
      target("dev", "http://dev:18088", "dev-token"),
    ],
  });

  assert.equal(results.length, 2);
  assert.equal(results[0].target.name, "prod");
  assert.equal(results[0].control.fallback_active, false);
  assert.equal(results[0].error, null);
  assert.equal(results[1].target.name, "dev");
  assert.equal(results[1].control, null);
  assert.match(results[1].error.message, /ECONNREFUSED/);
});

test("publishStatusToTargets posts status to every target", async (t) => {
  const originalFetch = globalThis.fetch;
  const requests = [];
  t.after(() => {
    globalThis.fetch = originalFetch;
  });

  globalThis.fetch = async (url, request) => {
    requests.push({
      url: String(url),
      authorization: request.headers.authorization,
      body: JSON.parse(request.body),
    });
    return jsonResponse({ ok: true });
  };

  const results = await publishStatusToTargets(
    {
      targets: [
        target("prod", "http://prod:8088", "prod-token"),
        target("dev", "http://dev:18088", "dev-token"),
      ],
    },
    "ok",
    "shared browser heartbeat",
  );

  assert.equal(results.length, 2);
  assert.deepEqual(
    requests.map((request) => ({
      url: request.url,
      authorization: request.authorization,
      body: request.body,
    })),
    [
      {
        url: "http://prod:8088/api/ingest/financial-juice/browser/status",
        authorization: "Bearer prod-token",
        body: { status: "ok", message: "shared browser heartbeat" },
      },
      {
        url: "http://dev:18088/api/ingest/financial-juice/browser/status",
        authorization: "Bearer dev-token",
        body: { status: "ok", message: "shared browser heartbeat" },
      },
    ],
  );
});

test("publishNewsBatch slices events by target config batch size", async (t) => {
  const originalFetch = globalThis.fetch;
  let postedBody = null;
  t.after(() => {
    globalThis.fetch = originalFetch;
  });

  globalThis.fetch = async (_url, request) => {
    postedBody = JSON.parse(request.body);
    return jsonResponse({ accepted: postedBody.events.length, dropped: 0 });
  };

  const result = await publishNewsBatch(
    {
      ...target("dev", "http://dev:18088", "dev-token"),
      maxBatchSize: 1,
    },
    [
      { external_id: "one" },
      { external_id: "two" },
    ],
  );

  assert.deepEqual(postedBody, { events: [{ external_id: "one" }] });
  assert.deepEqual(result, { accepted: 1, dropped: 0 });
});

function target(name, backendUrl, ingestToken) {
  return {
    name,
    backendUrl,
    ingestToken,
    controlEndpoint: `${backendUrl}/api/ingest/financial-juice/browser/control`,
    statusEndpoint: `${backendUrl}/api/ingest/financial-juice/browser/status`,
    newsEndpoint: `${backendUrl}/api/ingest/financial-juice/browser/news`,
  };
}

function jsonResponse(body, init = {}) {
  return new Response(JSON.stringify(body), {
    status: init.status ?? 200,
    headers: {
      "content-type": "application/json",
    },
  });
}
