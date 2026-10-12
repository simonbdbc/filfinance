#!/usr/bin/env node

import { writeFile } from "node:fs/promises";
import { pathToFileURL } from "node:url";

const BETTER_AUTH_STAGES = new Set([
  "sign_in",
  "session_restore",
  "jwt",
  "sign_out",
  "session_closed",
]);
const BETTER_AUTH_ERROR_KINDS = new Set(["transport", "http", "response"]);
const BETTER_AUTH_RESPONSE_FORMATS = new Set(["json", "non_json"]);

function formatDiagnostic({ stage, kind, status, responseFormat }) {
  return [
    `stage=${stage}`,
    `kind=${kind}`,
    ...(status === null ? [] : [`status=${status}`]),
    ...(responseFormat === null ? [] : [`response=${responseFormat}`]),
  ].join(" ");
}

export class BetterAuthSmokeTokenError extends Error {
  constructor({ stage, kind, status = null, responseFormat = null }) {
    if (!BETTER_AUTH_STAGES.has(stage)) {
      throw new TypeError("invalid Better Auth smoke token error stage");
    }
    if (!BETTER_AUTH_ERROR_KINDS.has(kind)) {
      throw new TypeError("invalid Better Auth smoke token error kind");
    }
    if (kind === "http" && (!Number.isInteger(status) || status < 100 || status > 599)) {
      throw new TypeError("invalid Better Auth smoke token HTTP status");
    }
    if (kind !== "http" && status !== null) {
      throw new TypeError("Better Auth smoke token status is restricted to HTTP errors");
    }
    if (kind === "transport" && responseFormat !== null) {
      throw new TypeError("Better Auth smoke token transport errors cannot have a response format");
    }
    if (kind !== "transport" && !BETTER_AUTH_RESPONSE_FORMATS.has(responseFormat)) {
      throw new TypeError("invalid Better Auth smoke token response format");
    }

    const diagnostic = formatDiagnostic({ stage, kind, status, responseFormat });
    super(`Better Auth smoke token acquisition failed (${diagnostic})`);
    this.name = "BetterAuthSmokeTokenError";
    this.stage = stage;
    this.kind = kind;
    this.status = status;
    this.responseFormat = responseFormat;
    this.diagnostic = diagnostic;
    Object.freeze(this);
  }
}

function required(environment, name) {
  const value = environment[name]?.trim() ?? "";
  if (!value) {
    throw new Error(`${name} is required`);
  }
  return value;
}

export async function acquireBetterAuthSmokeToken({
  environment = process.env,
  fetchImpl = fetch,
  writeFileImpl = writeFile,
  log = console.log,
} = {}) {
  const apiUrl = required(environment, "FILFINANCE_API_BASE_URL").replace(/\/$/u, "");
  const origin = required(environment, "FILFINANCE_PWA_URL").replace(/\/$/u, "");
  const email = required(environment, "FILFINANCE_SMOKE_EMAIL");
  const password = required(environment, "FILFINANCE_SMOKE_PASSWORD");
  const tokenFile = required(environment, "FILFINANCE_SMOKE_TOKEN_FILE");
  const accountLabel = environment.FILFINANCE_SMOKE_ACCOUNT_LABEL?.trim() || "configured account";

  async function authRequest(stage, path, init = {}) {
    let response;
    let bodyText;
    try {
      response = await fetchImpl(`${apiUrl}${path}`, {
        ...init,
        headers: {
          origin,
          ...(init.body ? { "content-type": "application/json" } : {}),
          ...init.headers,
        },
      });
      bodyText = await response.text();
    } catch {
      throw new BetterAuthSmokeTokenError({ stage, kind: "transport" });
    }

    let body = null;
    let responseFormat = "non_json";
    if (bodyText) {
      try {
        body = JSON.parse(bodyText);
        responseFormat = "json";
      } catch {
        body = null;
      }
    }
    if (!response.ok) {
      throw new BetterAuthSmokeTokenError({
        stage,
        kind: "http",
        status: response.status,
        responseFormat,
      });
    }
    return { body, response, responseFormat };
  }

  const signIn = await authRequest("sign_in", "/api/auth/sign-in/email", {
    method: "POST",
    body: JSON.stringify({ email, password }),
  });
  let cookie;
  try {
    cookie = signIn.response.headers
      .getSetCookie()
      .map((value) => value.split(";", 1)[0])
      .join("; ");
  } catch {
    throw new BetterAuthSmokeTokenError({
      stage: "sign_in",
      kind: "response",
      responseFormat: signIn.responseFormat,
    });
  }
  if (!cookie) {
    throw new BetterAuthSmokeTokenError({
      stage: "sign_in",
      kind: "response",
      responseFormat: signIn.responseFormat,
    });
  }

  let token;
  try {
    const session = await authRequest("session_restore", "/api/auth/get-session", {
      headers: { cookie },
    });
    if (session.body?.user?.email?.toLowerCase() !== email.toLowerCase()) {
      throw new BetterAuthSmokeTokenError({
        stage: "session_restore",
        kind: "response",
        responseFormat: session.responseFormat,
      });
    }

    const tokenResponse = await authRequest("jwt", "/api/auth/token", {
      headers: { cookie },
    });
    token = tokenResponse.body?.token;
    if (typeof token !== "string" || token.split(".").length !== 3) {
      throw new BetterAuthSmokeTokenError({
        stage: "jwt",
        kind: "response",
        responseFormat: tokenResponse.responseFormat,
      });
    }
  } finally {
    await authRequest("sign_out", "/api/auth/sign-out", {
      method: "POST",
      body: "{}",
      headers: { cookie },
    });
    const signedOutSession = await authRequest("session_closed", "/api/auth/get-session", {
      headers: { cookie },
    });
    if (signedOutSession.responseFormat !== "json" || signedOutSession.body !== null) {
      throw new BetterAuthSmokeTokenError({
        stage: "session_closed",
        kind: "response",
        responseFormat: signedOutSession.responseFormat,
      });
    }
  }

  await writeFileImpl(tokenFile, token, { encoding: "utf8", mode: 0o600 });
  log(`Better Auth smoke JWT acquired and session closed for ${accountLabel}`);
}

if (process.argv[1] && pathToFileURL(process.argv[1]).href === import.meta.url) {
  await acquireBetterAuthSmokeToken();
}
