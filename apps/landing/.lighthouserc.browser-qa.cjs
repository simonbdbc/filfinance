// Ce passage précède les audits ; il produit les captures et les preuves fonctionnelles.
const base = require("./.lighthouserc.mobile.cjs");
module.exports = {
  ci: {
    ...base.ci,
    collect: {
      ...base.ci.collect,
      ...(process.env.CHROME_PATH ? { chromePath: process.env.CHROME_PATH } : {}),
      puppeteerScript: "./scripts/lighthouse-browser-qa.cjs",
      puppeteerLaunchOptions: { args: ["--no-sandbox", "--disable-dev-shm-usage"] },
      settings: { ...base.ci.collect.settings, chromeFlags: undefined },
    },
  },
};
