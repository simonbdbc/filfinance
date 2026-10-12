const test = require("node:test");
const assert = require("node:assert/strict");
const ts = require("typescript");
const fs = require("node:fs");
const Module = require("node:module");
const source = fs.readFileSync(
  require.resolve("../src/shared/api/authSession.ts"),
  "utf8",
);
const mod = new Module(__filename);
mod._compile(
  ts.transpileModule(source, {
    compilerOptions: {
      module: ts.ModuleKind.CommonJS,
      target: ts.ScriptTarget.ES2022,
    },
  }).outputText,
  __filename,
);
const { AuthSessionClient, authenticatedFetch, SessionExpiredError } =
  mod.exports;
const originalFetch = global.fetch;
let clients = [];
test.afterEach(() => {
  for (const c of clients) c.close();
  clients = [];
  global.fetch = originalFetch;
});
const grant = (token, subject = "user-1") => ({
  token,
  subject,
  expiresAt: Date.now() + 900_000,
});
const deferred = () => {
  let resolve, reject;
  const promise = new Promise((r, j) => {
    resolve = r;
    reject = j;
  });
  return { promise, resolve, reject };
};
async function setup(renew, notify) {
  let calls = 0;
  const session = new AuthSessionClient(
    "user-1",
    () => (++calls === 1 ? Promise.resolve(grant("old")) : renew()),
    notify,
  );
  clients.push(session);
  await session.refresh();
  return { session, calls: () => calls };
}
const auth = (req) => req.headers.get("Authorization");
test("five concurrent 401s share renewal; each request is replayed once with its body and headers", async () => {
  const gate = deferred();
  const { session, calls } = await setup(() => gate.promise);
  const requests = [];
  let rejected = 0;
  const allRejected = deferred();
  global.fetch = async (req) => {
    requests.push({
      auth: auth(req),
      method: req.method,
      body: await req.text(),
      session: req.headers.get("x-local-session"),
      custom: req.headers.get("x-custom"),
    });
    if (auth(req) === "Bearer old") {
      if (++rejected === 5) allRejected.resolve();
      return new Response("", { status: 401 });
    }
    return new Response("ok");
  };
  const pending = Array.from({ length: 5 }, (_, i) =>
    authenticatedFetch(session, "https://service.test/api", {
      method: "POST",
      headers: new Headers({
        "x-local-session": "reader",
        "x-custom": "value",
      }),
      body: JSON.stringify({ i }),
    }),
  );
  await allRejected.promise;
  await new Promise((r) => setImmediate(r));
  assert.equal(calls(), 2);
  gate.resolve(grant("new"));
  const responses = await Promise.all(pending);
  assert(responses.every((r) => r.status === 200));
  assert.equal(requests.length, 10);
  for (let i = 0; i < 5; i++) {
    const pair = requests.filter((r) => r.body === JSON.stringify({ i }));
    assert.equal(pair.length, 2);
    assert(
      pair.every(
        (r) =>
          r.method === "POST" && r.session === "reader" && r.custom === "value",
      ),
    );
  }
});
test("late 401 after preventive renewal reuses new token", async () => {
  const { session, calls } = await setup(async () => grant("new"));
  const gate = deferred();
  global.fetch = async (req) =>
    auth(req) === "Bearer old" ? gate.promise : new Response("ok");
  const pending = authenticatedFetch(session, "https://service.test/api");
  await new Promise((r) => setImmediate(r));
  await session.refresh();
  gate.resolve(new Response("", { status: 401 }));
  assert.equal((await pending).status, 200);
  assert.equal(calls(), 2);
});
test("second 401 is returned, no loop or logout; 403 does not renew", async () => {
  const { session, calls } = await setup(async () => grant("new"));
  let sends = 0;
  global.fetch = async () => {
    sends++;
    return new Response("", { status: 401 });
  };
  assert.equal(
    (await authenticatedFetch(session, "https://service.test/api")).status,
    401,
  );
  assert.equal(sends, 2);
  assert.equal(calls(), 2);
  assert.equal(session.getToken(), "new");
  global.fetch = async () => new Response("", { status: 403 });
  assert.equal(
    (await authenticatedFetch(session, "https://service.test/api")).status,
    403,
  );
  assert.equal(calls(), 2);
});
test("network failure of a mutation is never replayed", async () => {
  const { session, calls } = await setup(async () => grant("new"));
  let sends = 0;
  global.fetch = async () => {
    sends++;
    throw new Error("offline");
  };
  await assert.rejects(
    authenticatedFetch(session, "https://service.test/api", {
      method: "POST",
      body: "{}",
    }),
    /offline/,
  );
  assert.equal(sends, 1);
  assert.equal(calls(), 1);
});
test("temporary renewal failure preserves session, token and cache", async () => {
  const states = [];
  const { session } = await setup(
    async () => {
      throw new Error("offline");
    },
    (s) => states.push(s),
  );
  session.cache.set("/api", {
    expiresAt: Date.now() + 5000,
    promise: Promise.resolve(1),
  });
  await assert.rejects(session.refresh(), /offline/);
  assert.equal(session.getToken(), "old");
  assert.equal(session.cache.size, 1);
  assert.equal(states.at(-1), "unavailable");
});
test("revoked session clears credentials/cache; a new login is isolated", async () => {
  const { session } = await setup(async () => {
    throw new SessionExpiredError();
  });
  session.cache.set("private", {});
  await assert.rejects(session.refresh(), SessionExpiredError);
  assert.equal(session.getToken(), null);
  assert.equal(session.cache.size, 0);
  assert(session.lifetime.signal.aborted);
  const other = new AuthSessionClient("other", async () =>
    grant("other", "other"),
  );
  clients.push(other);
  await other.refresh();
  assert.notEqual(session.id, other.id);
});
test("logout during renewal rejects late grant without reviving session", async () => {
  const gate = deferred();
  const { session } = await setup(() => gate.promise);
  const pending = session.refresh();
  await Promise.resolve();
  session.close();
  gate.resolve(grant("new"));
  await assert.rejects(pending, SessionExpiredError);
  assert.equal(session.getToken(), null);
});
test("cross-user grant is rejected", async () => {
  const { session } = await setup(async () => grant("wrong", "other"));
  await assert.rejects(session.refresh(), /Invalid token grant/);
  assert.equal(session.getToken(), "old");
});
test("aborted waiter is not replayed and does not cancel shared renewal", async () => {
  const gate = deferred();
  const { session, calls } = await setup(() => gate.promise);
  let sends = 0;
  global.fetch = async (req) => {
    sends++;
    return new Response("", { status: auth(req) === "Bearer old" ? 401 : 200 });
  };
  const controller = new AbortController();
  const cancelled = authenticatedFetch(session, "https://service.test/a", {
    signal: controller.signal,
  });
  const kept = authenticatedFetch(session, "https://service.test/b");
  await new Promise((r) => setImmediate(r));
  controller.abort();
  await assert.rejects(cancelled, { name: "AbortError" });
  gate.resolve(grant("new"));
  assert.equal((await kept).status, 200);
  assert.equal(calls(), 2);
  assert.equal(sends, 3);
});
test("Request inputs are cloned before sending; already aborted requests never fetch", async () => {
  const { session } = await setup(async () => grant("new"));
  const bodies = [];
  global.fetch = async (req) => {
    bodies.push(await req.text());
    return new Response("", { status: bodies.length === 1 ? 401 : 200 });
  };
  await authenticatedFetch(
    session,
    new Request("https://service.test/api", {
      method: "PATCH",
      body: "payload",
    }),
  );
  assert.deepEqual(bodies, ["payload", "payload"]);
  const controller = new AbortController();
  controller.abort();
  await assert.rejects(
    authenticatedFetch(session, "https://service.test/api", {
      signal: controller.signal,
    }),
    { name: "AbortError" },
  );
  assert.equal(bodies.length, 2);
});
test("preventive timer and concurrent refresh use one renewal", async (t) => {
  t.mock.timers.enable({ apis: ["setTimeout", "Date"], now: 1_000_000 });
  const gate = deferred();
  const { session, calls } = await setup(() => gate.promise);
  t.mock.timers.tick(840_000);
  const pending = session.refresh();
  await Promise.resolve();
  assert.equal(calls(), 2);
  gate.resolve(grant("new"));
  await pending;
  assert.equal(session.getToken(), "new");
  session.close();
  t.mock.timers.reset();
});
