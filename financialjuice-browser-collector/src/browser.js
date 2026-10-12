import { chromium } from "playwright";

export async function openBrowserContext(config) {
  return chromium.launchPersistentContext(config.userDataDir, {
    headless: config.headless,
    viewport: { width: 1440, height: 960 },
    locale: "en-US",
    timezoneId: "Europe/Paris",
  });
}
