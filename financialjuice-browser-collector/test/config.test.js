import assert from "node:assert/strict";
import test from "node:test";

import { loadConfig, validateConfig } from "../src/config.js";
import { loadLocalEnvFilePath } from "../src/env-file.js";

test("config stays inert when browser collector is disabled", () => {
  const config = loadConfig({
    NEWS_FINANCIAL_JUICE_BROWSER_ENABLED: "false",
  });

  assert.equal(config.enabled, false);
  assert.deepEqual(validateConfig(config), []);
});

test("enabled config requires backend token and FinancialJuice credentials", () => {
  const config = loadConfig({
    NEWS_FINANCIAL_JUICE_BROWSER_ENABLED: "true",
  });

  assert.deepEqual(validateConfig(config), [
    "NEWS_FINANCIAL_JUICE_BROWSER_INGEST_TOKEN is required when the browser collector target \"default\" is enabled",
    "NEWS_FINANCIAL_JUICE_EMAIL is required when the browser collector is enabled",
    "NEWS_FINANCIAL_JUICE_PASSWORD is required when the browser collector is enabled",
  ]);
});

test("config keeps the legacy single backend target shape", () => {
  const config = loadConfig({
    NEWS_FINANCIAL_JUICE_BROWSER_ENABLED: "true",
    NEWS_FINANCIAL_JUICE_BROWSER_BACKEND_URL: "http://backend:8088",
    NEWS_FINANCIAL_JUICE_BROWSER_INGEST_TOKEN: "shared-token",
  });

  assert.equal(config.backendUrl, "http://backend:8088");
  assert.equal(config.ingestToken, "shared-token");
  assert.equal(config.targets.length, 1);
  assert.equal(config.targets[0].name, "default");
  assert.equal(
    config.targets[0].newsEndpoint,
    "http://backend:8088/api/ingest/financial-juice/browser/news",
  );
});

test("config reads named backend targets", () => {
  const config = loadConfig({
    NEWS_FINANCIAL_JUICE_BROWSER_ENABLED: "true",
    NEWS_FINANCIAL_JUICE_BROWSER_TARGETS: "prod,dev",
    NEWS_FINANCIAL_JUICE_BROWSER_BACKEND_URL_PROD: "http://prod:8088",
    NEWS_FINANCIAL_JUICE_BROWSER_INGEST_TOKEN_PROD: "prod-token",
    NEWS_FINANCIAL_JUICE_BROWSER_BACKEND_URL_DEV: "http://host.docker.internal:18088",
    NEWS_FINANCIAL_JUICE_BROWSER_INGEST_TOKEN_DEV: "dev-token",
  });

  assert.deepEqual(
    config.targets.map((target) => ({
      name: target.name,
      backendUrl: target.backendUrl,
      ingestToken: target.ingestToken,
      controlEndpoint: target.controlEndpoint,
    })),
    [
      {
        name: "prod",
        backendUrl: "http://prod:8088",
        ingestToken: "prod-token",
        controlEndpoint: "http://prod:8088/api/ingest/financial-juice/browser/control",
      },
      {
        name: "dev",
        backendUrl: "http://host.docker.internal:18088",
        ingestToken: "dev-token",
        controlEndpoint: "http://host.docker.internal:18088/api/ingest/financial-juice/browser/control",
      },
    ],
  );
});

test("config validates named backend target URLs and tokens", () => {
  const config = loadConfig({
    NEWS_FINANCIAL_JUICE_BROWSER_ENABLED: "true",
    NEWS_FINANCIAL_JUICE_BROWSER_TARGETS: "prod,dev",
    NEWS_FINANCIAL_JUICE_BROWSER_BACKEND_URL_PROD: "http://prod:8088",
    NEWS_FINANCIAL_JUICE_BROWSER_INGEST_TOKEN_PROD: "prod-token",
    NEWS_FINANCIAL_JUICE_EMAIL: "user@example.com",
    NEWS_FINANCIAL_JUICE_PASSWORD: "password",
  });

  assert.deepEqual(validateConfig(config), [
    "NEWS_FINANCIAL_JUICE_BROWSER_BACKEND_URL_DEV is required when the browser collector target \"dev\" is enabled",
    "NEWS_FINANCIAL_JUICE_BROWSER_INGEST_TOKEN_DEV is required when the browser collector target \"dev\" is enabled",
  ]);
});

test("config reads the browser challenge backoff", () => {
  const config = loadConfig({
    NEWS_FINANCIAL_JUICE_BROWSER_ENABLED: "true",
    NEWS_FINANCIAL_JUICE_BROWSER_RATE_LIMIT_BACKOFF_MS: "120000",
  });

  assert.equal(config.rateLimitBackoffMs, 120_000);
});

test("config defaults to headed mode for the persistent Docker collector", () => {
  const config = loadConfig({
    NEWS_FINANCIAL_JUICE_BROWSER_ENABLED: "true",
  });

  assert.equal(config.headless, false);
});

test("config reads history backfill controls", () => {
  const config = loadConfig({
    NEWS_FINANCIAL_JUICE_BROWSER_ENABLED: "true",
    NEWS_FINANCIAL_JUICE_BROWSER_HISTORY_PAGE_LIMIT: "42",
    NEWS_FINANCIAL_JUICE_BROWSER_HISTORY_REFRESH_MS: "120000",
    NEWS_FINANCIAL_JUICE_BROWSER_HISTORY_TIMEZONE: "Europe/Paris",
  });

  assert.equal(config.historyPageLimit, 42);
  assert.equal(config.historyRefreshMs, 120_000);
  assert.equal(config.historyTimezone, "Europe/Paris");
});

test("env loader reads FILFINANCE_ENV_FILE", () => {
  const targetEnv = {
    FILFINANCE_ENV_FILE: "test/fixtures/collector.env",
  };

  assert.equal(loadLocalEnvFilePath(targetEnv), true);
  assert.equal(targetEnv.NEWS_FINANCIAL_JUICE_BROWSER_ENABLED, "true");
});
