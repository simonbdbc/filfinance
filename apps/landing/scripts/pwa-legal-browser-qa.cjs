const assert = require("node:assert/strict");
const fs = require("node:fs/promises");
const path = require("node:path");

// La PWA testée utilise exclusivement des origines loopback et des réponses simulées.
module.exports = async (browser, { url }) => {
  assert(["127.0.0.1", "localhost"].includes(new URL(url).hostname));
  const context = await browser.createBrowserContext();
  const page = await context.newPage();
  const directory = path.resolve("reports/legal");
  let authenticated = false;
  let theme = "light";
  const requests = [];
  const errors = [];
  page.on("pageerror", error => errors.push(error.message));
  await page.setRequestInterception(true);
  const user = { id: "legal-qa-user", name: "Recette locale", email: "legal-qa@example.test", emailVerified: true };
  await page.evaluateOnNewDocument(() => {
    // La recette légale ne teste ni le flux WebSocket ni sa disponibilité.
    class InactiveWebSocket extends EventTarget {
      static CONNECTING = 0; static OPEN = 1; static CLOSING = 2; static CLOSED = 3;
      readyState = 3; close() {} send() {};
    }
    window.WebSocket = InactiveWebSocket;
  });
  page.on("request", request => {
    const target = new URL(request.url());
    if ([new URL(url).origin, "http://127.0.0.1:4321"].includes(target.origin) || ["data:", "blob:"].includes(target.protocol)) { void request.continue(); return; }
    if (target.origin !== "http://127.0.0.1:9999") { void request.abort(); return; }
    requests.push({ method: request.method(), path: target.pathname });
    const headers = { "access-control-allow-origin": new URL(url).origin, "access-control-allow-credentials": "true", "access-control-allow-headers": "content-type, authorization", "access-control-allow-methods": "GET, POST, PATCH, OPTIONS", "content-type": "application/json" };
    if (request.method() === "OPTIONS") { void request.respond({ status: 204, headers }); return; }
    let body = { message: "Service hors périmètre de cette recette" };
    let status = 503;
    if (target.pathname.endsWith("/get-session")) {
      body = authenticated ? { user, session: { id: "legal-qa-session", userId: user.id, expiresAt: new Date(Date.now() + 3600000).toISOString(), token: "local-fixture" } } : null;
      status = 200;
    } else if (target.pathname.endsWith("/token")) {
      const payload = Buffer.from(JSON.stringify({ sub: user.id, exp: Math.floor(Date.now() / 1000) + 3600, email_verified: true })).toString("base64url");
      body = { token: `eyJhbGciOiJSUzI1NiJ9.${payload}.local-test-signature` }; status = 200;
    } else if (target.pathname === "/api/me") {
      body = { id: user.id, email: user.email, role: "listener", preferences: { theme, preferred_tts_voice: null }, created_at: "2026-09-12T00:00:00Z", updated_at: "2026-09-12T00:00:00Z" }; status = 200;
    }
    void request.respond({ status, headers, body: JSON.stringify(body) });
  });
  const clickText = async (text) => {
    const buttons = await page.$$('[role="button"], button, [tabindex="0"]');
    for (const button of buttons) if (await button.evaluate(element => element.textContent.trim()) === text) { await button.click(); return; }
    throw new Error(`Commande absente : ${text}`);
  };
  const renders = [];
  for (const width of [320, 390, 768, 1440]) {
    await page.setViewport({ width, height: 1000, deviceScaleFactor: 1 });
    for (const nextTheme of ["light", "dark"]) {
      theme = nextTheme;
      for (const state of ["registration", "profile"]) {
        authenticated = state === "profile";
        await page.goto(new URL("/login", url).href, { waitUntil: "networkidle0" });
        await page.evaluate(value => localStorage.setItem("filfinance.theme", value), theme);
        await page.goto(new URL(authenticated ? "/profile" : "/login", url).href, { waitUntil: "networkidle0" });
        if (!authenticated) {
          await page.waitForSelector('input[placeholder="Email"]');
          await clickText("Creer un compte");
          await page.waitForSelector('input[placeholder="Nom"]');
        } else await page.waitForFunction(() => document.body.textContent.includes("Infos Profil"));
        const links = await page.$$eval('a[target="_blank"]', anchors => anchors.map(anchor => ({ href: anchor.href, rel: anchor.rel, label: anchor.textContent })));
        for (const route of ["mentions-legales", "politique-de-confidentialite", "conditions-generales-utilisation", "cookies"]) {
          assert(links.some(link => link.href === `http://127.0.0.1:4321/${route}/`), `${state}/${width} : ${route}`);
        }
        assert(links.every(link => link.rel.includes("noopener") && link.rel.includes("noreferrer")));
        assert.equal(await page.evaluate(() => document.documentElement.scrollWidth), width);
        const legalTextColor = await page.$eval('a[href$="/mentions-legales/"]', a => getComputedStyle(a).color);
        assert.equal(legalTextColor, theme === "light" ? "rgb(17, 24, 39)" : "rgb(245, 245, 245)");
        if (!authenticated) assert(await page.$eval('a[aria-label="Politique de confidentialité (nouvel onglet)"]', a => a.closest('div').textContent.includes("SB ALGO")));
        renders.push({ width, theme, state, legalLinks: links.length, legalTextColor, passed: true });
        if (width === 390) await page.screenshot({ path: path.join(directory, `pwa-${state}-${theme}-390.png`), fullPage: true });
      }
    }
  }
  authenticated = false;
  await page.goto(new URL("/login", url).href, { waitUntil: "networkidle0" });
  await clickText("Creer un compte");
  await page.type('input[placeholder="Nom"]', "Recette locale");
  await page.type('input[placeholder="Email"]', "legal-qa@example.test");
  const popupPromise = new Promise(resolve => page.once("popup", resolve));
  await page.focus('a[aria-label="Politique de confidentialité (nouvel onglet)"]');
  await page.keyboard.press("Enter");
  const popup = await popupPromise;
  await popup.waitForSelector("#legal-title");
  assert.equal(await popup.$eval("h1", h1 => h1.textContent), "Politique de confidentialité");
  await popup.close();
  // Better Auth peut revalider la session à la reprise du focus ; attendre le formulaire restauré.
  await page.waitForSelector('input[placeholder="Nom"]');
  assert.equal(await page.$eval('input[placeholder="Nom"]', input => input.value), "Recette locale");
  assert.equal(await page.$eval('input[placeholder="Email"]', input => input.value), "legal-qa@example.test");
  assert(!requests.some(request => request.path.includes("sign-up") || request.path.includes("send-verification")));
  assert.deepEqual(errors, []);
  await fs.writeFile(path.join(directory, "pwa-browser-qa.json"), JSON.stringify({ date: new Date().toISOString(), scope: "Build PWA réel ; compte, profil et API simulés, aucun compte créé", renders, popupAndFormPreservation: "passed", keyboard: "passed", errors }, null, 2) + "\n");
  await context.close();
  console.log("Recette PWA : 16 rendus, accès anonyme/connecté simulé, nouvel onglet et conservation du formulaire validés.");
};
