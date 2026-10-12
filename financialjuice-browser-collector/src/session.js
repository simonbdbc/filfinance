import { detectBlockedPage } from "./extract-news.js";

const EMAIL_SELECTOR = "input[type='email'], input[name*='email' i], input[id*='email' i]";
const MAIN_FEED_ROOT_SELECTOR = "#mainFeed[data-feedid='mainfeed']";
const PASSWORD_SELECTOR = "input[type='password'], input[name*='password' i], input[id*='password' i]";
const SUBMIT_SELECTOR = "button[type='submit'], input[type='submit'], button:has-text('Login'), button:has-text('Sign in')";

export async function ensureFinancialJuiceSession(page, config) {
  if (shouldNavigateToHome(page.url(), config.homeUrl)) {
    await page.goto(config.homeUrl, {
      waitUntil: "domcontentloaded",
      timeout: config.requestTimeoutMs,
    });
  }

  if (await detectBlockedPage(page)) {
    return { ok: false, status: "blocked", message: "FinancialJuice page is blocked by a browser challenge" };
  }

  const loginVisible = await page.locator(PASSWORD_SELECTOR).first().isVisible().catch(() => false);
  if (!loginVisible) {
    return await ensureMainFeedPresent(
      page,
      config,
      "FinancialJuice session is already usable",
    );
  }

  if (!config.email || !config.password) {
    return { ok: false, status: "blocked", message: "FinancialJuice login form is visible but credentials are missing" };
  }

  await page.locator(EMAIL_SELECTOR).first().fill(config.email, { timeout: config.requestTimeoutMs });
  await page.locator(PASSWORD_SELECTOR).first().fill(config.password, { timeout: config.requestTimeoutMs });
  await Promise.all([
    page.waitForLoadState("domcontentloaded", { timeout: config.requestTimeoutMs }).catch(() => null),
    page.locator(SUBMIT_SELECTOR).first().click({ timeout: config.requestTimeoutMs }),
  ]);

  if (await detectBlockedPage(page)) {
    return { ok: false, status: "blocked", message: "FinancialJuice login reached a browser challenge" };
  }

  const stillLogin = await page.locator(PASSWORD_SELECTOR).first().isVisible().catch(() => false);
  if (stillLogin) {
    return { ok: false, status: "blocked", message: "FinancialJuice login form is still visible after submit" };
  }

  return await ensureMainFeedPresent(
    page,
    config,
    "FinancialJuice session refreshed",
  );
}

export function shouldNavigateToHome(currentUrl, homeUrl) {
  if (!currentUrl || currentUrl === "about:blank") {
    return true;
  }

  try {
    const current = new URL(currentUrl);
    const home = new URL(homeUrl);
    return current.origin !== home.origin;
  } catch {
    return true;
  }
}

async function ensureMainFeedPresent(page, config, successMessage) {
  if (await hasMainFeed(page)) {
    return { ok: true, status: "ok", message: successMessage };
  }

  console.warn(
    "[financialjuice-browser] FinancialJuice #mainFeed is missing; refreshing page before marking collector blocked",
  );
  try {
    await page.reload({
      waitUntil: "domcontentloaded",
      timeout: config.requestTimeoutMs,
    });
  } catch (error) {
    const message = `FinancialJuice #mainFeed is missing and page refresh failed: ${error.message}`;
    console.error(`[financialjuice-browser] ${message}`);
    return { ok: false, status: "blocked", message };
  }

  if (await detectBlockedPage(page)) {
    const message = "FinancialJuice #mainFeed is missing and refresh reached a browser challenge";
    console.error(`[financialjuice-browser] ${message}`);
    return { ok: false, status: "blocked", message };
  }

  if (await hasMainFeed(page)) {
    return { ok: true, status: "ok", message: successMessage };
  }

  const message = "FinancialJuice #mainFeed is missing after refresh; live feed DOM is not usable";
  console.error(`[financialjuice-browser] ${message}`);
  return { ok: false, status: "blocked", message };
}

async function hasMainFeed(page) {
  await page
    .locator(MAIN_FEED_ROOT_SELECTOR)
    .first()
    .waitFor({ state: "attached", timeout: 2_000 })
    .catch(() => null);

  const mainFeedCount = await page
    .locator(MAIN_FEED_ROOT_SELECTOR)
    .count()
    .catch(() => 0);
  return mainFeedCount > 0;
}
