import assert from "node:assert/strict";
import { mkdtemp, mkdir, writeFile } from "node:fs/promises";
import { createServer } from "node:http";
import { tmpdir } from "node:os";
import path from "node:path";
import { spawn } from "node:child_process";
import test from "node:test";

const root = path.resolve(import.meta.dirname, "..");

test("Bunny storage upload publishes index last, removes manifest orphans, then purges", async () => {
  const requests = [];
  const server = createServer((request, response) => {
    const body = [];
    request.on("data", chunk => body.push(chunk));
    request.on("end", () => {
      requests.push({ method: request.method, path: request.url, body: Buffer.concat(body).toString() });
      if (request.method === "GET" && request.url === "/zone/.filfinance-bunny-manifest.json") {
        response.setHeader("Content-Type", "application/json");
        response.end(JSON.stringify({ files: [{ path: "old.js" }] }));
        return;
      }
      response.statusCode = request.url?.startsWith("/pullzone/") ? 204 : 200;
      response.end();
    });
  });
  await new Promise(resolve => server.listen(0, "127.0.0.1", resolve));
  const address = server.address();
  const base = `http://127.0.0.1:${address.port}`;
  const directory = await mkdtemp(path.join(tmpdir(), "filfinance-bunny-test-"));
  await mkdir(path.join(directory, "assets"));
  await writeFile(path.join(directory, "assets", "app.js"), "bundle");
  await writeFile(path.join(directory, "index.html"), "index");

  const child = spawn(process.execPath, ["scripts/deploy_bunny_storage.mjs", "--dist", directory], {
    cwd: root,
    env: {
      ...process.env,
      BUNNY_STORAGE_ZONE: "zone",
      BUNNY_STORAGE_PASSWORD: "storage-password",
      BUNNY_PULL_ZONE_ID: "42",
      BUNNY_API_KEY: "api-key",
      BUNNY_STORAGE_ENDPOINT: base,
      BUNNY_API_ENDPOINT: base,
    },
  });
  const exitCode = await new Promise(resolve => child.on("close", resolve));
  server.close();
  assert.equal(exitCode, 0);

  const putPaths = requests.filter(item => item.method === "PUT").map(item => item.path);
  assert.deepEqual(putPaths.slice(0, 2), ["/zone/assets/app.js", "/zone/index.html"]);
  assert.ok(requests.some(item => item.method === "DELETE" && item.path === "/zone/old.js"));
  assert.ok(requests.some(item => item.method === "POST" && item.path === "/pullzone/42/purgeCache"));
});
