// Les seuils et les trois passages restent ceux des configurations claires.
module.exports = (device) => {
  const base = require(`./.lighthouserc.${device}.cjs`);
  const { chromeFlags, ...settings } = base.ci.collect.settings;
  return {
    ci: {
      ...base.ci,
      collect: {
        ...base.ci.collect,
        ...(process.env.CHROME_PATH ? { chromePath: process.env.CHROME_PATH } : {}),
        puppeteerScript: "./scripts/lighthouse-dark.cjs",
        puppeteerLaunchOptions: { args: ["--no-sandbox", "--disable-dev-shm-usage"] },
        settings: { ...settings, disableStorageReset: true },
      },
      upload: { ...base.ci.upload, outputDir: `./reports/lighthouse-${device}-dark` },
    },
  };
};
