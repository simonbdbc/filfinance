const assert = require("node:assert/strict");
const fs = require("node:fs/promises");
const path = require("node:path");

module.exports = async (browser, { url }) => {
  const context = await browser.createBrowserContext();
  const page = await context.newPage();
  await page.setCacheEnabled(false);
  const routes = ["mentions-legales", "politique-de-confidentialite", "conditions-generales-utilisation", "cookies"];
  const results = { date: new Date().toISOString(), scope: "Recette locale des documents légaux", renders: [] };
  const directory = path.resolve("reports/legal");
  await fs.mkdir(directory, { recursive: true });
  const errors = [];
  page.on("pageerror", error => errors.push(error.message));
  for (const width of [320, 390, 768, 1440]) {
    await page.setViewport({ width, height: 900, deviceScaleFactor: 1 });
    for (const theme of ["light", "dark"]) {
      for (const route of routes) {
        const response = await page.goto(new URL(`/${route}/`, url).href, { waitUntil: "networkidle0" });
        assert.equal(response.status(), 200);
        if (await page.$eval("html", root => root.dataset.theme) !== theme) await page.click("#theme-toggle");
        const render = await page.evaluate(() => ({
          width: innerWidth, documentWidth: document.documentElement.scrollWidth,
          title: document.querySelector("h1").textContent,
          theme: document.documentElement.dataset.theme,
          links: [...document.querySelectorAll('footer nav[aria-label="Informations légales"] a')].map(a => a.getAttribute("href")),
        }));
        assert.equal(render.documentWidth, width, `${route}/${theme}/${width} déborde`);
        assert.equal(render.theme, theme);
        assert.deepEqual(render.links, routes.map(route => `/${route}/`));
        assert.notEqual(render.title, "Écoutez les marchés.");
        results.renders.push({ route, ...render });
        if (route === "politique-de-confidentialite" && [390, 1440].includes(width)) {
          await page.evaluate(() => document.activeElement.blur());
          await page.screenshot({ path: path.join(directory, `confidentialite-${theme}-${width}.png`), fullPage: true });
        }
      }
    }
  }
  await page.focus("#theme-toggle");
  await page.keyboard.press("Space");
  assert.equal(await page.$eval("html", root => root.dataset.theme), "light");
  await page.reload();
  assert.equal(await page.$eval("html", root => root.dataset.theme), "light");
  await page.click('footer a[href="/#calendrier"]');
  await page.waitForSelector("#calendar-title");
  assert.equal(new URL(page.url()).hash, "#calendrier");
  assert.deepEqual(errors, []);
  results.keyboardThemeAndHomeAnchor = "passed";
  results.errors = errors;
  await fs.writeFile(path.join(directory, "browser-qa.json"), JSON.stringify(results, null, 2) + "\n");
  await context.close();
  console.log("Recette légale : 32 rendus sans débordement, liens, thème et clavier validés.");
  // Le parcours PWA utilise son build local et des réponses d'API simulées, uniquement sur demande.
  if (process.env.PWA_LEGAL_QA_ORIGIN) await require("./pwa-legal-browser-qa.cjs")(browser, { url: process.env.PWA_LEGAL_QA_ORIGIN });
};
