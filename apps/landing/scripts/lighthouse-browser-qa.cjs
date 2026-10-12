// Ces contrôles utilisent le navigateur isolé de Lighthouse, jamais le profil utilisateur.
const assert = require("node:assert/strict");
const fs = require("node:fs/promises");
const path = require("node:path");

module.exports = async (browser, { url }) => {
  const directory = path.resolve("reports/epic038");
  await fs.mkdir(directory, { recursive: true });
  const results = { timestamp: new Date().toISOString(), url, cases: [], viewports: [] };
  const check = (name, actual, expected) => {
    assert.deepEqual(actual, expected, name);
    results.cases.push({ name, passed: true });
  };
  const context = await browser.createBrowserContext();
  const page = await context.newPage();
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.emulateMediaFeatures([{ name: "prefers-color-scheme", value: "dark" }]);
  await page.goto(url, { waitUntil: "networkidle0" });
  const state = () => page.evaluate(() => ({
    theme: document.documentElement.dataset.theme,
    pressed: document.querySelector("#theme-toggle").getAttribute("aria-pressed"),
    hidden: document.querySelector("#theme-toggle").hidden,
  }));
  check("Première visite claire malgré un système sombre", await state(), { theme: "light", pressed: "false", hidden: false });
  await page.focus("#theme-toggle");
  await page.keyboard.press("Space");
  check("Bascule au clavier et état annoncé", await state(), { theme: "dark", pressed: "true", hidden: false });
  check("Mémorisation", await page.evaluate(() => localStorage.getItem("filfinance.landing.theme")), "dark");
  await page.reload({ waitUntil: "networkidle0" });
  check("Retour avec préférence sombre", (await state()).theme, "dark");
  await page.focus("#theme-toggle");
  check("Focus visible", await page.$eval("#theme-toggle", (button) => getComputedStyle(button).outlineWidth), "3px");

  for (const width of [320, 390, 768, 1440]) {
    await page.setViewport({ width, height: 900, deviceScaleFactor: 1 });
    for (const theme of ["light", "dark"]) {
      if ((await state()).theme !== theme) await page.click("#theme-toggle");
      await page.evaluate(() => document.fonts.ready);
      const layout = await page.evaluate(() => {
        const root = document.documentElement;
        const bounds = (selector) => {
          const box = document.querySelector(selector).getBoundingClientRect();
          return { left: box.left, right: box.right, width: box.width };
        };
        const styles = getComputedStyle(root);
        return {
          width: innerWidth, pageWidth: root.scrollWidth,
          theme: root.dataset.theme,
          colorScheme: styles.colorScheme,
          browserColor: document.querySelector('meta[name="theme-color"]').content,
          colors: ["background", "surface", "text", "text-secondary", "accent", "button", "button-text"].map((key) => styles.getPropertyValue(`--color-${key}`).trim().toUpperCase()),
          audio: bounds("#flux-audio"), summary: bounds("#resume-audio"),
          days: document.querySelectorAll("thead th").length,
          services: document.querySelectorAll(".service").length,
          tableScroll: document.querySelector(".calendar-scroll").scrollWidth > document.querySelector(".calendar-scroll").clientWidth,
          commandVisible: document.querySelector("#theme-toggle").getBoundingClientRect().height === 44,
          sections: [...document.querySelectorAll("main > section")].map((section) => section.id),
        };
      });
      check(`${theme}/${width} : aucun débordement de page`, layout.pageWidth, width);
      check(`${theme}/${width} : deux bandes pleine largeur`, [layout.audio.width, layout.summary.width], [width, width]);
      check(`${theme}/${width} : quatre services et sept jours`, [layout.services, layout.days], [4, 7]);
      check(`${theme}/${width} : commande visible`, layout.commandVisible, true);
      check(`${theme}/${width} : couleurs du navigateur`, [layout.colorScheme, layout.browserColor], [theme, theme === "light" ? "#F1E5D3" : "#111827"]);
      check(`${theme}/${width} : ordre des sections`, layout.sections, ["", "offre", "flux-audio", "actualites", "resume-audio", "calendrier"]);
      check(`${theme}/${width} : palette exacte`, layout.colors, theme === "light"
        ? ["#F1E5D3", "#FBF5EB", "#382E26", "#796B5D", "#7B5E45", "#7B5E45", "#FBF5EB"]
        : ["#111827", "#192535", "#FBF5EB", "#F1E5D3", "#8B5E3C", "#714426", "#FBF5EB"]);
      // Retirer le focus et revenir en haut uniquement pour la capture de composition.
      await page.evaluate(() => { document.activeElement.blur(); window.scrollTo({ top: 0, behavior: "instant" }); });
      const filename = `landing-${theme}-${width}.png`;
      await page.screenshot({ path: path.join(directory, filename), fullPage: true });
      results.viewports.push({ ...layout, capture: filename });
    }
  }

  await page.setViewport({ width: 320, height: 900, deviceScaleFactor: 1 });
  await page.focus(".mobile-menu summary");
  await page.keyboard.press("Enter");
  check("Menu mobile au clavier", await page.$eval(".mobile-menu", (menu) => menu.open), true);
  await page.keyboard.press("Tab");
  check("Premier lien du menu au clavier", await page.evaluate(() => document.activeElement.getAttribute("href")), "#flux-audio");
  await page.keyboard.press("Enter");
  check("Ancre audio", new URL(page.url()).hash, "#flux-audio");
  for (const anchor of ["#actualites", "#resume-audio", "#calendrier"]) {
    await page.click(`.mobile-menu a[href="${anchor}"]`);
    check(`Navigation ${anchor}`, new URL(page.url()).hash, anchor);
  }
  await page.focus(".mobile-menu summary");
  await page.keyboard.press("Enter");
  await page.focus(".calendar-scroll");
  await page.keyboard.press("ArrowRight");
  await page.waitForFunction(() => document.querySelector(".calendar-scroll").scrollLeft > 0);
  check("Calendrier défilable au clavier", await page.$eval(".calendar-scroll", (element) => element.scrollLeft > 0), true);
  await page.click(".footer-brand");
  check("Retour à la marque", new URL(page.url()).hash, "#top");
  await page.focus(".skip-link");
  await page.keyboard.press("Enter");
  check("Lien d'évitement au clavier", await page.evaluate(() => document.activeElement.id), "contenu");
  check("Console sans erreur", errors, []);
  await context.close();

  // Chaque panne est injectée avant le script applicatif dans un contexte neuf.
  for (const scenario of ["invalid", "read-denied", "write-denied", "first-paint", "no-js"]) {
    const isolated = await browser.createBrowserContext();
    const tab = await isolated.newPage();
    await tab.evaluateOnNewDocument((scenario) => {
      if (scenario === "invalid") localStorage.setItem("filfinance.landing.theme", "invalid");
      if (scenario === "first-paint") {
        localStorage.setItem("filfinance.landing.theme", "dark");
        window.__firstBodyTheme = null;
        const observer = new MutationObserver(() => {
          if (document.body && window.__firstBodyTheme === null) {
            window.__firstBodyTheme = document.documentElement.dataset.theme;
            observer.disconnect();
          }
        });
        observer.observe(document, { subtree: true, childList: true });
      }
      if (scenario === "read-denied") Storage.prototype.getItem = () => { throw new Error("Stockage refusé pour la recette"); };
      if (scenario === "write-denied") Storage.prototype.setItem = () => { throw new Error("Stockage refusé pour la recette"); };
    }, scenario);
    await tab.goto(url, { waitUntil: "networkidle0" });
    if (scenario === "no-js") {
      await tab.evaluate(() => localStorage.setItem("filfinance.landing.theme", "dark"));
      await tab.setJavaScriptEnabled(false);
      await tab.reload({ waitUntil: "networkidle0" });
      check("Sans JavaScript : clair et commande cachée", await tab.evaluate(() => [document.documentElement.dataset.theme, document.querySelector("#theme-toggle").hidden]), ["light", true]);
      await tab.setViewport({ width: 390, height: 844 });
      await tab.click(".mobile-menu summary");
      check("Sans JavaScript : menu natif disponible", await tab.$eval(".mobile-menu", (menu) => menu.open), true);
    } else if (scenario === "first-paint") {
      check("Sombre avant l'insertion du body", await tab.evaluate(() => window.__firstBodyTheme), "dark");
    } else {
      check(`${scenario} : repli clair`, await tab.$eval("html", (root) => root.dataset.theme), "light");
      await tab.click("#theme-toggle");
      check(`${scenario} : bascule pendant la visite`, await tab.$eval("html", (root) => root.dataset.theme), "dark");
    }
    await isolated.close();
  }
  await fs.writeFile(path.join(directory, "browser-qa.json"), JSON.stringify(results, null, 2) + "\n");
  console.log(`Recette navigateur : ${results.cases.length} contrôles réussis, 8 captures.`);
  await require("./lighthouse-legal-qa.cjs")(browser, { url });
};
