import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import test from "node:test";

const productionEnv = {
  ...process.env,
  NODE_ENV: "production",
  BETTER_AUTH_URL: "https://api.dev.filfinance.fr",
  BETTER_AUTH_JWT_AUDIENCE: "https://api.dev.filfinance.fr",
  BETTER_AUTH_JWT_EXPIRATION: "15m",
  BETTER_AUTH_SECRET: "test-secret-with-at-least-thirty-two-characters",
  BETTER_AUTH_TRUSTED_ORIGINS: "filfinance://,https://app.dev.filfinance.fr",
  AUTH_DATABASE_URL: "postgresql://filfinance_auth:secret@localhost:5432/filfinance",
  DATABASE_URL: "postgresql://filfinance_app:secret@localhost:5432/filfinance",
  APP_WEB_URL: "https://app.dev.filfinance.fr",
  EXPO_SCHEME: "filfinance",
  SMTP_HOST: "smtp-relay.brevo.com",
  SMTP_PORT: "587",
  SMTP_SECURE: "false",
  SMTP_FROM_EMAIL: "noreply@filfinance.fr",
  SMTP_FROM_NAME: "FilFinance",
  SMTP_REQUIRE_TLS: "true",
  SMTP_USER: "test-brevo-user",
  SMTP_PASSWORD: "test-brevo-key"
};

function loadEnv(requireEmailVerification: boolean, overrides: NodeJS.ProcessEnv = {}) {
  return spawnSync(
    process.execPath,
    ["--import", "tsx", "--eval", "await import('./src/env.ts')"],
    {
      cwd: process.cwd(),
      env: {
        ...productionEnv,
        BETTER_AUTH_REQUIRE_EMAIL_VERIFICATION: String(requireEmailVerification),
        ...overrides
      },
      encoding: "utf8"
    }
  );
}

test("production runtime rejects disabled email verification", () => {
  const result = loadEnv(false);

  assert.notEqual(result.status, 0);
  assert.match(
    result.stderr,
    /BETTER_AUTH_REQUIRE_EMAIL_VERIFICATION must be true in production runtime mode/
  );
});

test("production runtime still requires SMTP when email verification is enabled", () => {
  const result = loadEnv(true, { SMTP_HOST: "" });

  assert.notEqual(result.status, 0);
  assert.match(
    result.stderr,
    /SMTP_HOST is required when email delivery is enabled in production/
  );
});

test("production runtime requires encrypted SMTP transport", () => {
  const result = loadEnv(true, {
    SMTP_SECURE: "true",
    SMTP_REQUIRE_TLS: "false"
  });

  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /SMTP_SECURE must be false for the Brevo STARTTLS relay in production/);
});

test("production runtime requires APP_WEB_URL to be a trusted HTTPS origin", () => {
  const untrustedResult = loadEnv(true, {
    APP_WEB_URL: "https://other.filfinance.fr"
  });
  const insecureResult = loadEnv(true, {
    APP_WEB_URL: "http://app.dev.filfinance.fr",
    BETTER_AUTH_TRUSTED_ORIGINS: "filfinance://,http://app.dev.filfinance.fr"
  });

  assert.notEqual(untrustedResult.status, 0);
  assert.match(
    untrustedResult.stderr,
    /APP_WEB_URL origin must be listed in BETTER_AUTH_TRUSTED_ORIGINS/
  );
  assert.notEqual(insecureResult.status, 0);
  assert.match(insecureResult.stderr, /APP_WEB_URL must use https in production/);
});

test("deployed runtime rejects cross-environment and noncanonical PWA origins", () => {
  const crossEnvironment = loadEnv(true, {
    BETTER_AUTH_TRUSTED_ORIGINS: "filfinance://,https://app.dev.filfinance.fr,https://app.filfinance.fr"
  });
  const crossSite = loadEnv(true, {
    APP_WEB_URL: "https://filfinance-dev.pages.dev",
    BETTER_AUTH_TRUSTED_ORIGINS: "filfinance://,https://filfinance-dev.pages.dev"
  });
  const legacyDevPwa = loadEnv(true, {
    APP_WEB_URL: "https://pwa-dev.filfinance.fr",
    BETTER_AUTH_TRUSTED_ORIGINS: "filfinance://,https://pwa-dev.filfinance.fr"
  });
  const noncanonicalDevPwa = loadEnv(true, {
    APP_WEB_URL: "https://app.dev.filfinance.fr:443",
    BETTER_AUTH_TRUSTED_ORIGINS: "filfinance://,https://app.dev.filfinance.fr"
  });
  const additionalDevPwa = loadEnv(true, {
    BETTER_AUTH_TRUSTED_ORIGINS:
      "filfinance://,https://app.dev.filfinance.fr,https://pwa-dev.filfinance.fr"
  });

  assert.notEqual(crossEnvironment.status, 0);
  assert.match(crossEnvironment.stderr, /https:\/\/app\.filfinance\.fr is not allowed for the dev API runtime/);
  assert.notEqual(crossSite.status, 0);
  assert.match(
    crossSite.stderr,
    /APP_WEB_URL must equal https:\/\/app\.dev\.filfinance\.fr for the dev API/
  );
  assert.notEqual(legacyDevPwa.status, 0);
  assert.match(
    legacyDevPwa.stderr,
    /APP_WEB_URL must equal https:\/\/app\.dev\.filfinance\.fr for the dev API/
  );
  assert.notEqual(noncanonicalDevPwa.status, 0);
  assert.match(
    noncanonicalDevPwa.stderr,
    /APP_WEB_URL must equal https:\/\/app\.dev\.filfinance\.fr for the dev API/
  );
  assert.notEqual(additionalDevPwa.status, 0);
  assert.match(
    additionalDevPwa.stderr,
    /https:\/\/pwa-dev\.filfinance\.fr is not allowed for the dev API runtime/
  );
});

test("deployed runtime rejects JWT and SMTP configuration drift", () => {
  const audienceDrift = loadEnv(true, {
    BETTER_AUTH_JWT_AUDIENCE: "https://another-api.filfinance.fr"
  });
  const expirationDrift = loadEnv(true, {
    BETTER_AUTH_JWT_EXPIRATION: "24h"
  });
  const placeholderHost = loadEnv(true, {
    SMTP_HOST: "smtp.example.net"
  });

  assert.notEqual(audienceDrift.status, 0);
  assert.match(audienceDrift.stderr, /BETTER_AUTH_JWT_AUDIENCE must equal BETTER_AUTH_URL/);
  assert.notEqual(expirationDrift.status, 0);
  assert.match(expirationDrift.stderr, /BETTER_AUTH_JWT_EXPIRATION must equal 15m/);
  assert.notEqual(placeholderHost.status, 0);
  assert.match(placeholderHost.stderr, /SMTP_HOST contains a placeholder value/);
});

test("production runtime accepts only the dedicated Brevo STARTTLS relay and sender", () => {
  const hostDrift = loadEnv(true, { SMTP_HOST: "mail.sbcenter.fr" });
  const portDrift = loadEnv(true, { SMTP_PORT: "465" });
  const senderDrift = loadEnv(true, { SMTP_FROM_EMAIL: "no-reply@filfinance.fr" });
  const missingUser = loadEnv(true, { SMTP_USER: "", SMTP_PASSWORD: "" });
  const blankName = loadEnv(true, { SMTP_FROM_NAME: "   " });
  const malformedName = loadEnv(true, { SMTP_FROM_NAME: "FilFinance\r\nBcc: test@filfinance.fr" });

  assert.notEqual(hostDrift.status, 0);
  assert.match(hostDrift.stderr, /SMTP_HOST must equal smtp-relay\.brevo\.com/);
  assert.notEqual(portDrift.status, 0);
  assert.match(portDrift.stderr, /SMTP_PORT must equal 587/);
  assert.notEqual(senderDrift.status, 0);
  assert.match(senderDrift.stderr, /SMTP_FROM_EMAIL must equal noreply@filfinance\.fr/);
  assert.notEqual(missingUser.status, 0);
  assert.match(missingUser.stderr, /SMTP_USER is required for the Brevo relay/);
  assert.notEqual(blankName.status, 0);
  assert.match(blankName.stderr, /SMTP_FROM_NAME is required when email delivery is enabled/);
  assert.notEqual(malformedName.status, 0);
  assert.match(malformedName.stderr, /SMTP_FROM_NAME must not contain line breaks/);
});

test("deployed runtime rejects a noncanonical public Better Auth URL", () => {
  const result = loadEnv(true, {
    BETTER_AUTH_URL: "https://api.dev.filfinance.fr/",
    BETTER_AUTH_JWT_AUDIENCE: "https://api.dev.filfinance.fr/"
  });

  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /BETTER_AUTH_URL must exactly match the deployed public HTTPS API origin/);
});

test("runtime rejects malformed booleans and ports", () => {
  const invalidBoolean = loadEnv(true, { SMTP_SECURE: "treu" });
  const invalidPort = loadEnv(true, { SMTP_PORT: "587oops" });

  assert.notEqual(invalidBoolean.status, 0);
  assert.match(invalidBoolean.stderr, /SMTP_SECURE must be a boolean/);
  assert.notEqual(invalidPort.status, 0);
  assert.match(invalidPort.stderr, /SMTP_PORT must be an integer between 1 and 65535/);
});
