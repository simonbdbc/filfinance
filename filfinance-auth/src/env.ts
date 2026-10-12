const REQUIRED_IN_PRODUCTION = new Set([
  "BETTER_AUTH_SECRET",
  "BETTER_AUTH_URL",
  "AUTH_DATABASE_URL",
  "DATABASE_URL",
  "APP_WEB_URL"
]);

const PRODUCTION_PLACEHOLDER_FRAGMENTS = [
  "change-me",
  "replace-with",
  "example.com",
  "example.net",
  ".invalid",
  "example.test"
];

function read(name: string, fallback = ""): string {
  const value = process.env[name] ?? fallback;
  if (process.env.NODE_ENV === "production" && REQUIRED_IN_PRODUCTION.has(name) && value.trim() === "") {
    throw new Error(`${name} is required in production`);
  }
  return value;
}

function readBoolean(name: string, fallback: boolean): boolean {
  const raw = process.env[name];
  if (raw == null || raw.trim() === "") {
    return fallback;
  }
  const normalized = raw.trim().toLowerCase();
  if (["1", "true", "yes", "on"].includes(normalized)) return true;
  if (["0", "false", "no", "off"].includes(normalized)) return false;
  throw new Error(`${name} must be a boolean`);
}

function splitCsv(name: string, fallback: string[]): string[] {
  const raw = process.env[name];
  if (raw == null || raw.trim() === "") {
    return fallback;
  }
  return raw
    .split(",")
    .map((item) => item.trim())
    .filter(Boolean);
}

function readInteger(name: string, fallback: number): number {
  const raw = read(name, String(fallback));
  if (!/^[0-9]+$/u.test(raw)) {
    throw new Error(`${name} must be an integer between 1 and 65535`);
  }
  const value = Number(raw);
  if (!Number.isSafeInteger(value) || value <= 0 || value > 65535) {
    throw new Error(`${name} must be an integer between 1 and 65535`);
  }
  return value;
}

const nodeEnv = read("NODE_ENV", "development");
const PROD_PWA_ORIGIN = "https://app.filfinance.fr";
const DEV_PWA_ORIGIN = "https://app.dev.filfinance.fr";
const defaultTrustedOrigins =
  nodeEnv === "production"
    ? ["filfinance://", PROD_PWA_ORIGIN]
    : [
        "filfinance://",
        "http://localhost:8052",
        "http://localhost:3000",
        "exp://",
        "exp://**",
        "exp://192.168.*.*:*/**"
      ];

const parsedEnv = {
  NODE_ENV: nodeEnv,
  PORT: readInteger("PORT", 3000),
  BETTER_AUTH_URL: read("BETTER_AUTH_URL", "http://localhost:3000"),
  BETTER_AUTH_SECRET: read("BETTER_AUTH_SECRET", "dev-only-change-me"),
  BETTER_AUTH_JWT_AUDIENCE: read(
    "BETTER_AUTH_JWT_AUDIENCE",
    read("BETTER_AUTH_URL", "http://localhost:3000")
  ),
  BETTER_AUTH_JWT_EXPIRATION: read("BETTER_AUTH_JWT_EXPIRATION", "15m"),
  BETTER_AUTH_REQUIRE_EMAIL_VERIFICATION: readBoolean(
    "BETTER_AUTH_REQUIRE_EMAIL_VERIFICATION",
    true
  ),
  AUTH_DATABASE_URL: read("AUTH_DATABASE_URL", "postgresql://filfinance_auth:filfinance_auth@localhost:5432/filfinance"),
  DATABASE_URL: read("DATABASE_URL", "postgresql://filfinance_app:filfinance_app@localhost:5432/filfinance"),
  POSTGRES_SSL: readBoolean("POSTGRES_SSL", false),
  APP_WEB_URL: read(
    "APP_WEB_URL",
    nodeEnv === "production" ? "" : "http://localhost:8052"
  ),
  EXPO_SCHEME: read("EXPO_SCHEME", "filfinance"),
  LANDING_USES_BETTER_AUTH: readBoolean("FILFINANCE_LANDING_USES_BETTER_AUTH", false),
  ADMIN_EMAIL_TO_PROMOTE: read("ADMIN_EMAIL_TO_PROMOTE", ""),
  TRUSTED_ORIGINS: splitCsv("BETTER_AUTH_TRUSTED_ORIGINS", defaultTrustedOrigins),
  SMTP_HOST: read("SMTP_HOST", ""),
  SMTP_PORT: readInteger("SMTP_PORT", 587),
  SMTP_SECURE: readBoolean("SMTP_SECURE", false),
  SMTP_REQUIRE_TLS: readBoolean("SMTP_REQUIRE_TLS", nodeEnv === "production"),
  SMTP_USER: read("SMTP_USER", ""),
  SMTP_PASSWORD: read("SMTP_PASSWORD", ""),
  SMTP_FROM_EMAIL: read("SMTP_FROM_EMAIL", ""),
  SMTP_FROM_NAME: read("SMTP_FROM_NAME", "")
};

if (nodeEnv === "production") {
  if (!parsedEnv.BETTER_AUTH_REQUIRE_EMAIL_VERIFICATION) {
    throw new Error("BETTER_AUTH_REQUIRE_EMAIL_VERIFICATION must be true in production runtime mode");
  }

  for (const [name, value] of [
    ["BETTER_AUTH_SECRET", parsedEnv.BETTER_AUTH_SECRET],
    ["BETTER_AUTH_URL", parsedEnv.BETTER_AUTH_URL],
    ["AUTH_DATABASE_URL", parsedEnv.AUTH_DATABASE_URL],
    ["DATABASE_URL", parsedEnv.DATABASE_URL],
    ["APP_WEB_URL", parsedEnv.APP_WEB_URL],
    ["SMTP_HOST", parsedEnv.SMTP_HOST],
    ["SMTP_FROM_EMAIL", parsedEnv.SMTP_FROM_EMAIL],
    ["SMTP_FROM_NAME", parsedEnv.SMTP_FROM_NAME],
    ["SMTP_USER", parsedEnv.SMTP_USER],
    ["SMTP_PASSWORD", parsedEnv.SMTP_PASSWORD],
    ["ADMIN_EMAIL_TO_PROMOTE", parsedEnv.ADMIN_EMAIL_TO_PROMOTE]
  ] as const) {
    const normalized = value.toLowerCase();
    if (PRODUCTION_PLACEHOLDER_FRAGMENTS.some((fragment) => normalized.includes(fragment))) {
      throw new Error(`${name} contains a placeholder value in production`);
    }
  }

  if (parsedEnv.BETTER_AUTH_SECRET.length < 32) {
    throw new Error("BETTER_AUTH_SECRET must contain at least 32 characters in production");
  }

  if (!parsedEnv.BETTER_AUTH_URL.startsWith("https://")) {
    throw new Error("BETTER_AUTH_URL must use https in production");
  }

  if (!parsedEnv.APP_WEB_URL.startsWith("https://")) {
    throw new Error("APP_WEB_URL must use https in production");
  }

  const appWebOrigin = new URL(parsedEnv.APP_WEB_URL).origin;
  if (!parsedEnv.TRUSTED_ORIGINS.includes(appWebOrigin)) {
    throw new Error("APP_WEB_URL origin must be listed in BETTER_AUTH_TRUSTED_ORIGINS");
  }

  const apiHost = new URL(parsedEnv.BETTER_AUTH_URL).hostname;
  if (apiHost === "api.filfinance.fr" || apiHost === "api.dev.filfinance.fr") {
    const expectedApiUrl = `https://${apiHost}`;
    if (parsedEnv.BETTER_AUTH_URL !== expectedApiUrl) {
      throw new Error("BETTER_AUTH_URL must exactly match the deployed public HTTPS API origin");
    }
    if (parsedEnv.BETTER_AUTH_JWT_AUDIENCE !== parsedEnv.BETTER_AUTH_URL) {
      throw new Error("BETTER_AUTH_JWT_AUDIENCE must equal BETTER_AUTH_URL for deployed runtimes");
    }
    if (parsedEnv.BETTER_AUTH_JWT_EXPIRATION !== "15m") {
      throw new Error("BETTER_AUTH_JWT_EXPIRATION must equal 15m for deployed runtimes");
    }
    if (parsedEnv.EXPO_SCHEME !== "filfinance") {
      throw new Error("EXPO_SCHEME must equal filfinance for deployed runtimes");
    }

    const isProdTarget = apiHost === "api.filfinance.fr";
    const expectedPwaOrigin = isProdTarget ? PROD_PWA_ORIGIN : DEV_PWA_ORIGIN;
    const expectedLandingOrigin = isProdTarget
      ? "https://filfinance.fr"
      : "https://dev.filfinance.fr";
    if (
      appWebOrigin !== expectedPwaOrigin
      || (!isProdTarget && parsedEnv.APP_WEB_URL !== DEV_PWA_ORIGIN)
    ) {
      throw new Error(
        `APP_WEB_URL must equal ${expectedPwaOrigin} for the ${isProdTarget ? "production" : "dev"} API`
      );
    }

    const allowedOrigins = new Set(["filfinance://", expectedPwaOrigin]);
    if (parsedEnv.LANDING_USES_BETTER_AUTH) {
      allowedOrigins.add(expectedLandingOrigin);
    }
    if (!isProdTarget) {
      allowedOrigins.add("exp://");
      allowedOrigins.add("exp://**");
      allowedOrigins.add("exp://192.168.*.*:*/**");
    }
    for (const origin of parsedEnv.TRUSTED_ORIGINS) {
      if (!allowedOrigins.has(origin)) {
        throw new Error(`${origin} is not allowed for the ${isProdTarget ? "production" : "dev"} API runtime`);
      }
    }
    if (!parsedEnv.TRUSTED_ORIGINS.includes("filfinance://")) {
      throw new Error("filfinance:// must be listed in BETTER_AUTH_TRUSTED_ORIGINS");
    }
    if (parsedEnv.LANDING_USES_BETTER_AUTH
      && !parsedEnv.TRUSTED_ORIGINS.includes(expectedLandingOrigin)) {
      throw new Error(`${expectedLandingOrigin} must be listed when the landing uses Better Auth`);
    }
  }

  if (parsedEnv.AUTH_DATABASE_URL === parsedEnv.DATABASE_URL) {
    throw new Error("AUTH_DATABASE_URL and DATABASE_URL must use separate roles in production");
  }

  const smtpConfigured = Boolean(
    parsedEnv.SMTP_HOST ||
      parsedEnv.SMTP_FROM_EMAIL ||
      parsedEnv.SMTP_FROM_NAME ||
      parsedEnv.SMTP_USER ||
      parsedEnv.SMTP_PASSWORD
  );

  if (parsedEnv.BETTER_AUTH_REQUIRE_EMAIL_VERIFICATION || smtpConfigured) {
    if (!parsedEnv.SMTP_HOST) {
      throw new Error("SMTP_HOST is required when email delivery is enabled in production");
    }
    if (parsedEnv.SMTP_HOST !== "smtp-relay.brevo.com") {
      throw new Error("SMTP_HOST must equal smtp-relay.brevo.com in production");
    }
    if (parsedEnv.SMTP_PORT !== 587) {
      throw new Error("SMTP_PORT must equal 587 in production");
    }
    if (parsedEnv.SMTP_SECURE) {
      throw new Error("SMTP_SECURE must be false for the Brevo STARTTLS relay in production");
    }
    if (!parsedEnv.SMTP_REQUIRE_TLS) {
      throw new Error("SMTP_REQUIRE_TLS must be true for the Brevo STARTTLS relay in production");
    }
    if (!parsedEnv.SMTP_FROM_EMAIL) {
      throw new Error("SMTP_FROM_EMAIL is required when email delivery is enabled in production");
    }
    if (!parsedEnv.SMTP_FROM_NAME.trim()) {
      throw new Error("SMTP_FROM_NAME is required when email delivery is enabled in production");
    }
    if (parsedEnv.SMTP_FROM_EMAIL !== "noreply@filfinance.fr") {
      throw new Error("SMTP_FROM_EMAIL must equal noreply@filfinance.fr in production");
    }
    if (/\r|\n/u.test(parsedEnv.SMTP_FROM_NAME)) {
      throw new Error("SMTP_FROM_NAME must not contain line breaks");
    }
    if (!/^[^<>\s@]+@[^<>\s@]+\.[^<>\s@]+$/u.test(parsedEnv.SMTP_FROM_EMAIL)) {
      throw new Error("SMTP_FROM_EMAIL must be a valid mailbox address");
    }
    if (!parsedEnv.SMTP_USER) {
      throw new Error("SMTP_USER is required for the Brevo relay in production");
    }
    if (!parsedEnv.SMTP_PASSWORD) {
      throw new Error("SMTP_PASSWORD is required for the Brevo relay in production");
    }
  }
}

export const env = parsedEnv;
