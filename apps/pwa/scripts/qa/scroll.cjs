// Régression du défilement : build local, API simulées, aucune requête distante.
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const puppeteer = require("../../../landing/node_modules/puppeteer-core");
const { fixture } = require("./fixtures.cjs");
const origin = "http://127.0.0.1:8052";
const root = path.resolve(__dirname, "../../dist");
const output = process.env.QA_OUTPUT_DIR || "/tmp/filfinance-scroll-qa";
const routes = ["/feed", "/calendar", "/profile", "/admin", "/admin/config-sources",
  "/admin/flux-status", "/admin/blacklist", "/admin/glossary", "/admin/history",
  "/admin/calendar-history", "/admin/calendar-day", "/admin/daily-summary", "/admin/logs",
  "/login", "/login#sign-up", "/login#forgot-password", "/login?token=qa-reset", "/email-verification"];
const pause = (ms = 180) => new Promise(resolve => setTimeout(resolve, ms));
const authOnly = process.argv.includes("--auth-only");
const matrixRoutes = authOnly ? routes.filter(route => route.startsWith("/login") || route === "/email-verification") : routes;

(async () => {
  fs.mkdirSync(output, { recursive: true });
  const browser = await puppeteer.launch({
    executablePath: process.env.CHROME_PATH || "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
    headless: true, args: ["--no-sandbox"],
  });
  const page = await browser.newPage();
  page.setDefaultTimeout(12000);
  const state = { authenticated: true, role: "admin" };
  const results = [], checks = [], errors = [];
  let theme = "light";
  page.on("pageerror", e => errors.push(e.message));
  await page.setRequestInterception(true);
  page.on("request", async request => {
    const url = new URL(request.url());
    if (url.origin === origin) {
      let file = path.join(root, url.pathname);
      if (!fs.existsSync(file) || fs.statSync(file).isDirectory()) file = path.join(root, "index.html");
      const types = { ".html": "text/html", ".js": "application/javascript", ".css": "text/css", ".png": "image/png", ".ttf": "font/ttf", ".ico": "image/x-icon" };
      // Initialisation du thème avant le bundle, uniquement dans la réponse de recette.
      let body = fs.readFileSync(file);
      if (file.endsWith("index.html")) body = Buffer.from(body.toString().replace("<head>", `<head><script>localStorage.setItem('filfinance.theme','${theme}')</script>`));
      return request.respond({ status: 200, contentType: types[path.extname(file)] || "application/octet-stream", body });
    }
    if (url.origin === "http://127.0.0.1:9999") {
      let f = fixture(url, state);
      if (state.longDetail && /^\/api\/translations\/\d+$/.test(url.pathname))
        f.body.translated_body = "Paragraphe illustratif pour vérifier le défilement du panneau. ".repeat(200);
      if (state.logs && url.pathname.startsWith("/api/admin/calendar-day/logs/")) {
        const entry = { id: "file0", name: "workflow-0.log", relativePath: "workflow-0.log", kind: "file",
          size: "0", identity: "qa", allocatedSize: null, state: "closed", eligible: true, reason: "Recette" };
        if (url.pathname.endsWith("/session") || url.pathname.endsWith("/health")) f = { body: { token: "qa-reader", serverId: "qa-reader" } };
        else if (url.pathname.endsWith("/roots")) f = { body: { rootId: "root", nodeId: "root", absolutePath: "/qa/logs" } };
        else if (url.pathname.endsWith("/entries")) f = { body: { entries: Array.from({ length: 80 }, (_, i) => ({ ...entry, id: `file${i}`, name: `workflow-${i}.log`, relativePath: `workflow-${i}.log` })), revision: "1", cursor: null } };
        else if (url.pathname.endsWith("/statistics")) f = { body: { items: {}, sampledAt: new Date().toISOString(), partial: false, busy: false, notices: [] } };
        else if (url.pathname.endsWith("/content")) {
          const bytes = Buffer.from(Array.from({ length: 1000 }, (_, i) => `Ligne ${i} ${"contenu illustratif ".repeat(8)}\n`).join(""));
          f = { body: { generation: "qa", start: "0", end: String(bytes.length), size: String(bytes.length), data: bytes.toString("base64"), partialStart: false } };
        } else f = { body: entry };
      }
      return request.respond({ status: request.method() === "OPTIONS" ? 204 : f.status || 200,
        headers: { "content-type": "application/json", "access-control-allow-origin": origin,
          "access-control-allow-credentials": "true", "access-control-allow-headers": "content-type,authorization,x-local-session",
          "access-control-allow-methods": "GET,POST,PATCH,OPTIONS" }, body: JSON.stringify(f.body) });
    }
    if (["blob:", "data:"].includes(url.protocol)) return request.continue();
    return request.abort();
  });
  async function goto(route, width = 390, height = 600) {
    state.authenticated = !["/login", "/email-verification"].includes(route.split(/[?#]/)[0]);
    await page.setViewport({ width, height, hasTouch: width < 1024 });
    await page.goto(origin + route, { waitUntil: "networkidle0" });
    await page.waitForSelector('[role="heading"]');
    const modeLabel = route.endsWith("#sign-up") ? "Créer un compte" : route.endsWith("#forgot-password") ? "Mot de passe oublié" : null;
    if (modeLabel) {
      for (const button of await page.$$('[role="button"],button')) {
        if ((await button.evaluate(el => el.textContent.trim())) === modeLabel) { await button.click(); break; }
      }
      await page.waitForFunction(label => [...document.querySelectorAll('[role="heading"]')].some(el => el.textContent.trim() === label), {}, modeLabel);
    }
    await page.evaluate(() => {
      const candidates = [...document.querySelectorAll("div")].filter(e => getComputedStyle(e).overflowY === "auto" && e.clientHeight > 100);
      window.__scrollTarget = document.querySelector('[data-testid="page-scroll"]') ||
        candidates.find(e => e.scrollHeight > e.clientHeight + 1) || candidates[0];
    });
    assert(await page.evaluate(() => !!window.__scrollTarget), "Conteneur défilant absent");
  }
  const position = () => page.evaluate(() => {
    const el = window.__scrollTarget, r = el.getBoundingClientRect();
    return { top: el.scrollTop, max: el.scrollHeight - el.clientHeight, height: el.clientHeight,
      x: r.left + 8, y: Math.min(r.bottom - 110, r.top + 180), bottom: r.bottom,
      horizontalOverflow: document.documentElement.scrollWidth > innerWidth,
      contentOverflow: el.scrollWidth > el.clientWidth + 1 };
  });
  async function wheel(deltaY) {
    const p = await position();
    await page.mouse.move(p.x, p.y);
    await page.mouse.wheel({ deltaY });
    await pause();
  }
  async function click(label) {
    const selector = `[aria-label=${JSON.stringify(label)}]`;
    await page.waitForSelector(selector);
    await page.click(selector);
  }
  try {
    for (const width of process.argv.includes("--targeted") ? [] : [320, 390, 768, 1024, 1440]) {
      for (theme of ["light", "dark"]) {
        for (const route of matrixRoutes) {
          await goto(route, width);
          const before = await position();
          assert.equal(await page.title(), "Fil Finance");
          assert.equal(await page.evaluate(() => document.documentElement.className), theme);
          assert(!before.horizontalOverflow && !before.contentOverflow, `Débordement ${route} ${width}`);
          await wheel(Math.max(650, before.max + 300));
          const after = await position();
          assert(Math.abs(after.top - after.max) <= 2, `Bas inaccessible ${route} ${width}: ${JSON.stringify(after)}`);
          if (before.max > 1) assert(after.top > 0, `Molette inactive ${route}`);
          if (state.authenticated) {
            // La fin réelle du contenu reste au-dessus de la navigation fixe.
            const end = await page.evaluate(() => {
              const content = window.__scrollTarget.firstElementChild.firstElementChild;
              const nav = document.querySelector('[aria-label="Navigation principale"]');
              return { bottom: content.getBoundingClientRect().bottom, navTop: nav.getBoundingClientRect().top };
            });
            assert(end.bottom <= (width < 1024 ? end.navTop : after.bottom) + 2, `Fin masquée ${route}`);
          } else {
            const lastLegalLinkBottom = await page.evaluate(() => [...document.querySelectorAll('a[target="_blank"]')].at(-1).getBoundingClientRect().bottom);
            assert(lastLegalLinkBottom <= after.bottom + 2, `Footer légal inaccessible ${route}`);
          }
          results.push({ route, width, theme, before, after });
        }
      }
      console.log(`Défilement : ${width}px OK`);
    }
    if (results.length) fs.writeFileSync(path.join(output, authOnly ? "auth-scroll-matrix.json" : "scroll-matrix.json"), JSON.stringify(results, null, 2));
    if (authOnly) {
      assert.equal(errors.length, 0, errors.join("\n"));
      console.log(`PASS : ${results.length} parcours d'authentification`);
      return;
    }
    // Clavier et thème : le focus porte sur le conteneur, sans scrollIntoView artificiel.
    await goto("/calendar");
    await page.focus('[data-testid="page-scroll"]');
    await page.keyboard.press("PageDown");
    await pause(300);
    const keyboard = await position();
    assert(keyboard.top > 0, "PageDown ne défile pas");
    await click("Basculer entre les thèmes clair et sombre");
    await pause();
    assert(Math.abs((await position()).top - keyboard.top) <= 1, "Le thème réinitialise la position");
    await page.focus('[data-testid="page-scroll"]');
    // End déplace le conteneur focalisé jusqu'à sa dernière ligne.
    await page.keyboard.press("End");
    await pause(300);
    assert(Math.abs((await position()).top - (await position()).max) <= 2, "Fin inaccessible au clavier");
    await click("Profil");
    await page.waitForFunction(() => location.pathname === "/profile");
    await pause();
    assert.equal((await position()).top, 0, "Navigation sans retour en haut");
    checks.push("PageDown/End, position conservée au thème, retour en haut à la navigation");

    // Geste tactile envoyé au navigateur, sans affecter directement scrollTop.
    await goto("/calendar");
    const cdp = await page.createCDPSession();
    const touch = (type, y) => cdp.send("Input.dispatchTouchEvent", { type,
      touchPoints: type === "touchEnd" ? [] : [{ x: 35, y, radiusX: 4, radiusY: 4, force: 1 }] });
    await touch("touchStart", 480);
    for (let y = 460; y >= 200; y -= 20) { await touch("touchMove", y); await pause(20); }
    await touch("touchEnd", 200);
    await pause(400);
    assert((await position()).top > 100, "Geste tactile sans défilement");
    checks.push("Balayage tactile : défilement réel du calendrier mobile");

    state.longDetail = true;
    await goto("/feed");
    await page.focus('[data-testid="news-1"]');
    await page.keyboard.press("Enter");
    await page.waitForSelector('[aria-label="Fermer le panneau"]');
    await page.waitForFunction(() => [...document.querySelectorAll('[role="dialog"] div')].some(el => getComputedStyle(el).overflowY === "auto" && el.scrollHeight > el.clientHeight + 100));
    const panel = await page.evaluate(() => {
      window.__panelScroll = [...document.querySelectorAll('[role="dialog"] div')].find(el => getComputedStyle(el).overflowY === "auto" && el.scrollHeight > el.clientHeight + 100);
      const r = window.__panelScroll.getBoundingClientRect(); return { x: r.left + 25, y: r.top + 40 };
    });
    await page.mouse.move(panel.x, panel.y);
    await page.mouse.wheel({ deltaY: 300 });
    await pause();
    assert(await page.evaluate(() => window.__panelScroll.scrollTop > 0), "Panneau long non défilant");
    await page.keyboard.press("Escape");
    await page.waitForFunction(() => document.activeElement?.getAttribute("data-testid") === "news-1");
    const closed = await position();
    await wheel(250);
    assert((await position()).top > closed.top, "Défilement bloqué après fermeture du panneau");
    checks.push("Panneau long : défilement interne, retour du focus et défilement après fermeture");
    state.longDetail = false;

    state.logs = true;
    await goto("/admin/calendar-day", 1440, 900);
    await wheel(100000);
    await click("workflow-0.log");
    await page.waitForFunction(() => document.querySelector('.log-scroll')?.scrollTop > 0);
    const outer = (await position()).top;
    const logBefore = await page.$eval('.log-scroll', el => el.scrollTop);
    const logRect = await page.$eval('.log-scroll', el => { const r = el.getBoundingClientRect(); return { x: r.left + 50, y: r.top + 70 }; });
    await page.mouse.move(logRect.x, logRect.y);
    await page.mouse.wheel({ deltaY: -300, deltaX: 150 });
    await pause();
    assert(await page.$eval('.log-scroll', (el, before) => el.scrollTop < before && el.scrollLeft > 0, logBefore), "Logs internes non défilants");
    assert.equal((await position()).top, outer, "Défilement des logs transmis à la page");
    const treeRect = await page.$eval('.tree-scroll', el => { const r = el.getBoundingClientRect(); return { x: r.left + 50, y: r.top + 60 }; });
    await page.mouse.move(treeRect.x, treeRect.y);
    await page.mouse.wheel({ deltaY: 250 });
    await pause();
    assert(await page.$eval('.tree-scroll', el => el.scrollTop > 0), "Arbre des logs non défilant");
    checks.push("Logs : arbre et lecteur défilants, axes vertical/horizontal indépendants de la page");
    state.logs = false;

    // Reflow correspondant à 200 % d'une fenêtre 1440×900 (viewport CSS divisé par deux).
    for (theme of ["light", "dark"]) {
      for (const route of routes) {
        await goto(route, 720, 450);
        await wheel(100000);
        const p = await position();
        assert(!p.horizontalOverflow && !p.contentOverflow && Math.abs(p.top - p.max) <= 2, `Reflow ${route}`);
        if (!state.authenticated) assert(await page.evaluate(() => [...document.querySelectorAll('a[target="_blank"]')].at(-1).getBoundingClientRect().bottom <= window.__scrollTarget.getBoundingClientRect().bottom + 2), `Footer légal en reflow ${route}`);
      }
    }
    checks.push(`${routes.length * 2} parcours en reflow 200 % équivalent (720×450 CSS)`);
    await goto("/calendar", 390);
    await wheel(100000);
    await page.screenshot({ path: path.join(output, "calendar-mobile-bottom.png") });
    await goto("/admin", 1440);
    await wheel(100000);
    await page.screenshot({ path: path.join(output, "admin-desktop-bottom.png") });
    assert.equal(errors.length, 0, errors.join("\n"));
    fs.writeFileSync(path.join(output, "scroll-results.json"), JSON.stringify({ date: new Date().toISOString(),
      scope: "Build Web local, API simulées, Chrome headless, gestes tactiles émulés", results, checks, errors }, null, 2));
    console.log(`PASS : ${results.length} parcours, ${checks.length} contrôles ciblés`);
  } catch (error) {
    await page.screenshot({ path: path.join(output, "failure.png") });
    console.error("Échec", page.url(), await position());
    throw error;
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
