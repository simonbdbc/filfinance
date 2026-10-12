import assert from "node:assert/strict";
import { createServer } from "node:http";
import { spawn } from "node:child_process";
import test from "node:test";
import { fileURLToPath } from "node:url";

const script = new URL("./check_bunny_dev_release_preflight.sh", import.meta.url);

test("Bunny DEV preflight accepts the configured Basic pull zones and rejects Advanced", async () => {
  let hlsIsBasic = true;
  const server = createServer((request, response) => {
    response.setHeader("content-type", "application/json");
    const body = resourceFor(request.url, hlsIsBasic);
    response.end(JSON.stringify(body));
  });
  await listen(server);
  const endpoint = `http://127.0.0.1:${server.address().port}`;

  try {
    const accepted = await runPreflight(endpoint);
    assert.equal(accepted.code, 0, accepted.stderr);
    assert.match(accepted.stdout, /Bunny DEV preflight passed/);

    hlsIsBasic = false;
    const rejected = await runPreflight(endpoint);
    assert.notEqual(rejected.code, 0);
    assert.match(rejected.stderr, /HLS is not Bunny Shield Basic/);
  } finally {
    await new Promise((resolve, reject) => server.close((error) => (error ? reject(error) : resolve())));
  }
});

function resourceFor(pathname, hlsIsBasic) {
  if (pathname === "/dnszone/857837") return { Domain: "dev.filfinance.fr" };
  if (pathname === "/shield/shield-zones") {
    return {
      data: [
        shieldZone(6451222, 1001, true),
        shieldZone(6033259, 1002, hlsIsBasic),
        shieldZone(6451226, 1003, true),
      ],
    };
  }
  if (pathname === "/shield/rate-limits/1001?perPage=10") {
    return { data: [{ ruleJson: '{"path":"/api/auth/"}' }] };
  }
  if (pathname === "/shield/rate-limits/1003?perPage=10") {
    return { data: [{ ruleJson: '{"path":"/ws/live"}' }] };
  }

  const pullZones = {
    "/pullzone/6451217": { Hostnames: ["dev.filfinance.fr"] },
    "/pullzone/6451219": { Hostnames: ["app.dev.filfinance.fr"] },
    "/pullzone/6451222": { Hostnames: ["api.dev.filfinance.fr"], OriginUrl: "https://origin.dev.filfinance.fr" },
    "/pullzone/6033259": {
      Hostnames: ["media.dev.filfinance.fr"],
      OriginUrl: "https://origin.dev.filfinance.fr",
      EdgeRules: [{ ActionType: "Block", TriggerValue: "POST" }],
    },
    "/pullzone/6451226": {
      Hostnames: ["live.dev.filfinance.fr"],
      OriginUrl: "https://origin.dev.filfinance.fr",
      EnableWebSockets: true,
    },
  };
  return pullZones[pathname] ?? { error: "unexpected request" };
}

function shieldZone(pullZoneId, shieldZoneId, basic) {
  return {
    pullZoneId,
    shieldZoneId,
    rateLimitRulesLimit: basic ? 2 : 10,
    customWafRulesLimit: basic ? 0 : 10,
  };
}

function listen(server) {
  return new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", resolve);
  });
}

function runPreflight(endpoint) {
  const env = {
    ...process.env,
    BUNNY_API_ENDPOINT: endpoint,
    BUNNY_API_KEY_DEV: "test-key",
    BUNNY_DNS_ZONE_ID_DEV: "857837",
    BUNNY_STORAGE_ZONE_DEV_LANDING: "1802401",
    BUNNY_STORAGE_PASSWORD_DEV_LANDING: "landing-password",
    BUNNY_PULL_ZONE_ID_DEV_LANDING: "6451217",
    BUNNY_STORAGE_ZONE_DEV_PWA: "1802402",
    BUNNY_STORAGE_PASSWORD_DEV_PWA: "pwa-password",
    BUNNY_PULL_ZONE_ID_DEV_PWA: "6451219",
    BUNNY_PULL_ZONE_ID_DEV_API: "6451222",
    BUNNY_PULL_ZONE_ID_DEV_MEDIA: "6033259",
    BUNNY_PULL_ZONE_ID_DEV_LIVE: "6451226",
    FILFINANCE_DEV_LANDING_URL: "https://dev.filfinance.fr",
    FILFINANCE_DEV_PWA_URL: "https://app.dev.filfinance.fr",
    FILFINANCE_DEV_API_URL: "https://api.dev.filfinance.fr",
    FILFINANCE_DEV_HLS_BASE_URL: "https://media.dev.filfinance.fr",
    FILFINANCE_DEV_EDGE_URL: "https://live.dev.filfinance.fr",
    FILFINANCE_DEV_ORIGIN_URL: "https://origin.dev.filfinance.fr",
  };
  return new Promise((resolve, reject) => {
    const child = spawn("bash", [fileURLToPath(script)], { env, stdio: ["ignore", "pipe", "pipe"] });
    let stdout = "";
    let stderr = "";
    child.stdout.setEncoding("utf8").on("data", (chunk) => { stdout += chunk; });
    child.stderr.setEncoding("utf8").on("data", (chunk) => { stderr += chunk; });
    child.on("error", reject);
    child.on("close", (code) => resolve({ code, stdout, stderr }));
  });
}
