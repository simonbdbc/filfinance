import assert from "node:assert/strict";
import test from "node:test";

import {
  acquireBetterAuthSmokeToken,
  BetterAuthSmokeTokenError,
} from "./acquire_better_auth_smoke_token.mjs";

const environment = {
  FILFINANCE_API_BASE_URL: "https://api.dev.filfinance.fr",
  FILFINANCE_PWA_URL: "https://app.dev.filfinance.fr",
  FILFINANCE_SMOKE_EMAIL: "listener@example.test",
  FILFINANCE_SMOKE_PASSWORD: "not-a-real-password",
  FILFINANCE_SMOKE_TOKEN_FILE: "/tmp/listener.jwt",
  FILFINANCE_SMOKE_ACCOUNT_LABEL: "listener",
};

function response(body, { cookie = "", rawText, status = 200 } = {}) {
  return {
    ok: status >= 200 && status < 300,
    status,
    headers: {
      getSetCookie: () => (cookie ? [cookie] : []),
    },
    text: async () => rawText ?? (body === null ? "null" : JSON.stringify(body)),
  };
}

function mockAuthFetch({
  httpFailure = null,
  restoredEmail = environment.FILFINANCE_SMOKE_EMAIL,
  revokeSession,
  signedOutRawText,
  token = "header.payload.signature",
  transportFailureAt = null,
}) {
  let sessionActive = false;
  let sessionRequestCount = 0;
  const paths = [];

  return {
    paths,
    fetch: async (url, init = {}) => {
      const path = new URL(url).pathname;
      const call = `${init.method ?? "GET"} ${path}`;
      paths.push(call);
      if (path === "/api/auth/get-session") {
        sessionRequestCount += 1;
      }
      const callKey = path === "/api/auth/get-session"
        ? `${call}#${sessionRequestCount}`
        : call;

      if (callKey === transportFailureAt) {
        throw new Error(
          `transport leaked ${environment.FILFINANCE_SMOKE_EMAIL} `
            + environment.FILFINANCE_SMOKE_PASSWORD,
        );
      }
      if (callKey === httpFailure?.at) {
        return response(httpFailure.body, {
          rawText: httpFailure.rawText,
          status: httpFailure.status,
        });
      }

      if (path === "/api/auth/sign-in/email") {
        sessionActive = true;
        return response({ user: { email: environment.FILFINANCE_SMOKE_EMAIL } }, {
          cookie: "better-auth.session_token=opaque; HttpOnly; Secure",
        });
      }
      if (path === "/api/auth/get-session") {
        if (sessionRequestCount === 2 && signedOutRawText !== undefined) {
          return response(null, { rawText: signedOutRawText });
        }
        return response(
          sessionActive ? { user: { email: restoredEmail } } : null,
        );
      }
      if (path === "/api/auth/token") {
        return response({ token });
      }
      if (path === "/api/auth/sign-out") {
        if (revokeSession) {
          sessionActive = false;
        }
        return response({ success: true });
      }

      return response({ code: "NOT_FOUND" }, { status: 404 });
    },
  };
}

test("acquires a JWT, closes its session, then writes the token with mode 0600", async () => {
  const mock = mockAuthFetch({ revokeSession: true });
  const writes = [];
  const logs = [];
  const signOutRequests = [];

  await acquireBetterAuthSmokeToken({
    environment,
    fetchImpl: async (url, init) => {
      if (new URL(url).pathname === "/api/auth/sign-out") {
        signOutRequests.push({
          body: init.body,
          contentType: init.headers["content-type"],
        });
      }
      return mock.fetch(url, init);
    },
    writeFileImpl: async (...args) => writes.push(args),
    log: (value) => logs.push(value),
  });

  assert.deepEqual(mock.paths, [
    "POST /api/auth/sign-in/email",
    "GET /api/auth/get-session",
    "GET /api/auth/token",
    "POST /api/auth/sign-out",
    "GET /api/auth/get-session",
  ]);
  assert.deepEqual(writes, [
    [environment.FILFINANCE_SMOKE_TOKEN_FILE, "header.payload.signature", {
      encoding: "utf8",
      mode: 0o600,
    }],
  ]);
  assert.deepEqual(logs, ["Better Auth smoke JWT acquired and session closed for listener"]);
  assert.deepEqual(signOutRequests, [{ body: "{}", contentType: "application/json" }]);
});

test("rejects a false-positive sign-out and never writes the JWT", async () => {
  const mock = mockAuthFetch({ revokeSession: false });
  let writeCalled = false;

  await assert.rejects(
    acquireBetterAuthSmokeToken({
      environment,
      fetchImpl: mock.fetch,
      writeFileImpl: async () => {
        writeCalled = true;
      },
      log: () => {},
    }),
    (error) => {
      assert.equal(error instanceof BetterAuthSmokeTokenError, true);
      assert.equal(
        error.message,
        "Better Auth smoke token acquisition failed "
          + "(stage=session_closed kind=response response=json)",
      );
      return true;
    },
  );

  assert.equal(writeCalled, false);
});

const stageCallKeys = [
  ["sign_in", "POST /api/auth/sign-in/email"],
  ["session_restore", "GET /api/auth/get-session#1"],
  ["jwt", "GET /api/auth/token"],
  ["sign_out", "POST /api/auth/sign-out"],
  ["session_closed", "GET /api/auth/get-session#2"],
];

for (const [stage, callKey] of stageCallKeys) {
  test(`classifies a ${stage} transport failure without leaking its cause`, async () => {
    const mock = mockAuthFetch({ revokeSession: true, transportFailureAt: callKey });

    await assert.rejects(
      acquireBetterAuthSmokeToken({
        environment,
        fetchImpl: mock.fetch,
        writeFileImpl: async () => assert.fail("must not write a JWT"),
        log: () => {},
      }),
      (error) => {
        assert.equal(error instanceof BetterAuthSmokeTokenError, true);
        assert.deepEqual(
          {
            diagnostic: error.diagnostic,
            kind: error.kind,
            responseFormat: error.responseFormat,
            stage: error.stage,
            status: error.status,
          },
          {
            diagnostic: `stage=${stage} kind=transport`,
            kind: "transport",
            responseFormat: null,
            stage,
            status: null,
          },
        );
        assert.equal(Object.isFrozen(error), true);
        assert.doesNotMatch(error.message, /listener@example\.test|not-a-real-password/u);
        return true;
      },
    );
  });
}

for (const [responseFormat, responseOptions] of [
  [
    "json",
    {
      body: {
        message: `rejected ${environment.FILFINANCE_SMOKE_EMAIL}`,
        token: "must-not-leak",
      },
    },
  ],
  [
    "non_json",
    {
      body: null,
      rawText: `rejected ${environment.FILFINANCE_SMOKE_PASSWORD} must-not-leak`,
    },
  ],
]) {
  for (const [stage, callKey] of stageCallKeys) {
    test(`classifies a ${stage} HTTP ${responseFormat} failure without its response`, async () => {
      const mock = mockAuthFetch({
        httpFailure: { at: callKey, status: 429, ...responseOptions },
        revokeSession: true,
      });

      await assert.rejects(
        acquireBetterAuthSmokeToken({
          environment,
          fetchImpl: mock.fetch,
          writeFileImpl: async () => assert.fail("must not write a JWT"),
          log: () => {},
        }),
        (error) => {
          assert.equal(error instanceof BetterAuthSmokeTokenError, true);
          assert.deepEqual(
            {
              diagnostic: error.diagnostic,
              kind: error.kind,
              responseFormat: error.responseFormat,
              stage: error.stage,
              status: error.status,
            },
            {
              diagnostic: `stage=${stage} kind=http status=429 response=${responseFormat}`,
              kind: "http",
              responseFormat,
              stage,
              status: 429,
            },
          );
          assert.doesNotMatch(
            error.message,
            /listener@example\.test|not-a-real-password|must-not-leak/u,
          );
          return true;
        },
      );
    });
  }
}

for (const [stage, mockOptions, responseFormat] of [
  ["session_restore", { restoredEmail: "another@example.test" }, "json"],
  ["jwt", { token: "must-not-leak" }, "json"],
  ["session_closed", { revokeSession: false }, "json"],
  ["session_closed", { signedOutRawText: "must-not-leak" }, "non_json"],
]) {
  test(`classifies an invalid ${stage} ${responseFormat} response without its content`, async () => {
    const mock = mockAuthFetch({ revokeSession: true, ...mockOptions });

    await assert.rejects(
      acquireBetterAuthSmokeToken({
        environment,
        fetchImpl: mock.fetch,
        writeFileImpl: async () => assert.fail("must not write a JWT"),
        log: () => {},
      }),
      (error) => {
        assert.equal(error instanceof BetterAuthSmokeTokenError, true);
        assert.equal(error.diagnostic, `stage=${stage} kind=response response=${responseFormat}`);
        assert.doesNotMatch(
          error.message,
          /another@example\.test|listener@example\.test|not-a-real-password|must-not-leak/u,
        );
        return true;
      },
    );
  });
}

test("classifies a missing sign-in cookie as a JSON response failure", async () => {
  await assert.rejects(
    acquireBetterAuthSmokeToken({
      environment,
      fetchImpl: async () => response({
        message: `signed in ${environment.FILFINANCE_SMOKE_EMAIL}`,
        token: "must-not-leak",
      }),
      writeFileImpl: async () => assert.fail("must not write a JWT"),
      log: () => {},
    }),
    (error) => {
      assert.equal(error instanceof BetterAuthSmokeTokenError, true);
      assert.equal(error.diagnostic, "stage=sign_in kind=response response=json");
      assert.doesNotMatch(error.message, /listener@example\.test|must-not-leak/u);
      return true;
    },
  );
});
