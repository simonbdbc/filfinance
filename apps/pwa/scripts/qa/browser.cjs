/* Recette locale exclusivement : aucune requête vers un environnement distant. */
const assert = require("node:assert/strict"),
  fs = require("node:fs/promises"),
  path = require("node:path");
const puppeteer = require("../../../landing/node_modules/puppeteer-core");
const { fixture, day } = require("./fixtures.cjs");
const origin = "http://127.0.0.1:8052";
const output = path.resolve(
  __dirname,
  "../../../..",
  "apps/pwa/test-results/public-qa",
);
const target = process.argv.find(arg => arg.startsWith("--route="))?.slice(8);
const resultPrefix = target ? "targeted-" : "";
const routes = [
  "/feed",
  "/calendar",
  "/profile",
  "/admin",
  "/admin/config-sources",
  "/admin/flux-status",
  "/admin/blacklist",
  "/admin/glossary",
  "/admin/history",
  "/admin/calendar-history",
  "/admin/calendar-day",
  "/admin/daily-summary",
  "/admin/logs",
  "/login",
  "/email-verification",
].filter(route => !target || route === target);
assert(routes.length, "Route de recette inconnue");
(async () => {
  await fs.mkdir(output, { recursive: true });
  const browser = await puppeteer.launch({
    executablePath:
      process.env.CHROME_PATH ||
      "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
    headless: true,
    args: ["--no-sandbox"],
  });
  const state = { role: "admin", authenticated: true, empty: false };
  const requests = [],
    errors = [],
    results = [],
    failures = [];
  const page = await browser.newPage();
  page.setDefaultTimeout(10000);
  page.on("pageerror", (e) => errors.push(e.message));
  await page.evaluateOnNewDocument((today) => {
    window.__qaSockets = [];
    class FixtureWebSocket extends EventTarget {
      static CONNECTING = 0;
      static OPEN = 1;
      static CLOSING = 2;
      static CLOSED = 3;
      readyState = 0;
      url;
      constructor(url) {
        super();
        this.url = String(url);
        window.__qaSockets.push(this);
        setTimeout(() => {
          if (this.readyState === 3) return;
          this.readyState = 1;
          const e = new Event("open");
          this.dispatchEvent(e);
          this.onopen?.(e);
          if (this.url.includes("/ws/news")) {
            const m = new MessageEvent("message", {
              data: JSON.stringify({
                type: "hello",
                payload: {
                  protocol_version: 1,
                  sources: [
                    {
                      source: "financial_juice",
                      enabled: true,
                      status: "active",
                    },
                    { source: "x", enabled: false, status: "disabled" },
                  ],
                  calendar_day_summary: {
                    local_day: today,
                    status: "missing",
                    audio_available: false,
                    updated_at: new Date().toISOString(),
                  },
                },
              }),
            });
            this.dispatchEvent(m);
            this.onmessage?.(m);
          }
        }, 30);
      }
      send() {}
      close() {
        this.readyState = 3;
      }
    }
    window.WebSocket = FixtureWebSocket;
  }, day);
  await page.setRequestInterception(true);
  page.on("request", (r) => {
    const u = new URL(r.url());
    if (u.origin === origin || ["data:", "blob:"].includes(u.protocol))
      return void r.continue();
    if (u.origin !== "http://127.0.0.1:9999") return void r.abort();
    requests.push({ method: r.method(), path: u.pathname });
    const f = fixture(u, state);
    void r.respond({
      status: r.method() === "OPTIONS" ? 204 : f.status || 200,
      headers: {
        "content-type": "application/json",
        "access-control-allow-origin": origin,
        "access-control-allow-credentials": "true",
        "access-control-allow-headers": "content-type,authorization",
        "access-control-allow-methods": "GET,POST,PATCH,OPTIONS",
      },
      body: JSON.stringify(f.body),
    });
  });
  const click = async (text) => {
    for (const el of await page.$$('[role="button"],button,[role="link"]'))
      if (
        (await el.evaluate((e) =>
          (e.getAttribute("aria-label") || e.textContent)
            .replace(/[\uE000-\uF8FF]/g, "")
            .trim(),
        )) === text
      ) {
        await el.evaluate((e) => e.scrollIntoView({ block: "center" }));
        await el.click();
        return;
      }
    throw new Error("Commande absente: " + text);
  };
  const goto = async (route) => {
    await page.goto(origin + route, { waitUntil: "networkidle0" });
    await page.waitForSelector('[role="heading"],h1');
  };
  try {
    await goto("/feed");
    for (const width of [320, 390, 768, 1024, 1440]) {
      await page.setViewport({ width, height: 1000, deviceScaleFactor: 1 });
      for (const theme of ["light", "dark"]) {
        await page.evaluate(
          (v) => localStorage.setItem("filfinance.theme", v),
          theme,
        );
        for (const route of routes) {
          state.authenticated = !["/login", "/email-verification"].includes(
            route,
          );
          await goto(route);
          const metrics = await page.evaluate(() => ({
            width: innerWidth,
            scroll: document.documentElement.scrollWidth,
            body: getComputedStyle(document.body).backgroundColor,
            headings: [...document.querySelectorAll('[role="heading"]')].map(
              (e) => e.textContent,
            ),
            audio: document.querySelectorAll('[data-testid="session-audio"]')
              .length,
          }));
          if (metrics.scroll !== width)
            failures.push({
              route,
              theme,
              width,
              scroll: metrics.scroll,
              overflow: await page.evaluate(() =>
                [...document.querySelectorAll("*")]
                  .filter(
                    (e) => e.getBoundingClientRect().right > innerWidth + 0.1,
                  )
                  .slice(-10)
                  .map((e) => ({
                    text: e.textContent.slice(0, 80),
                    cls: e.className,
                    right: e.getBoundingClientRect().right,
                  })),
              ),
            });
          assert.equal(
            metrics.body,
            theme === "light" ? "rgb(241, 229, 211)" : "rgb(17, 24, 39)",
            `${route}: fond`,
          );
          assert.equal(
            metrics.audio,
            state.authenticated ? 1 : 0,
            `${route}: lecteur commun`,
          );
          assert(metrics.headings.length > 0, route);
          if (route === "/feed")
            assert(
              await page.evaluate(() =>
                document.body.textContent.includes("Les marchés européens"),
              ),
              "Historique chargé",
            );
          if (route === "/calendar")
            assert(
              await page.evaluate(() =>
                document.body.textContent.includes("Indice des prix"),
              ),
              "Calendrier chargé",
            );
          if (
            [320, 1440].includes(width) &&
            ["/feed", "/calendar", "/profile", "/admin", "/login"].includes(
              route,
            )
          ) {
            if (route === "/feed" && width === 1440) {
              const first = await page.$('[data-testid="news-1"]');
              if (first) await first.click();
            }
            await page.screenshot({
              path: path.join(
                output,
                `${route.slice(1)}-${theme}-${width}.png`,
              ),
              fullPage: true,
            });
          }
          results.push({ route, width, theme, ...metrics });
        }
        console.log(`✓ ${width}px ${theme}: ${routes.length} routes`);
      }
    }
    await fs.writeFile(
      path.join(output, `${resultPrefix}matrix-results.json`),
      JSON.stringify({ results, failures, errors }, null, 2),
    );
    assert.equal(failures.length, 0, JSON.stringify(failures, null, 2));
    state.authenticated = true;
    state.role = "listener";
    await page.setViewport({ width: 390, height: 844 });
    await goto("/feed");
    // Sélection, chargement du détail, focus initial, confinement et restitution.
    const first = await page.$('[data-testid="news-1"]');
    assert(first, "Ligne de news");
    await first.focus();
    await page.keyboard.press("Enter");
    await page.waitForSelector('[aria-label="Fermer le panneau"]');
    assert.equal(
      await page.evaluate(() =>
        document.activeElement.getAttribute("aria-label"),
      ),
      "Fermer le panneau",
    );
    await page.keyboard.press("Escape");
    await page.waitForSelector('[aria-label="Fermer le panneau"]', {
      hidden: true,
    });
    await page.waitForFunction(
      () => document.activeElement?.getAttribute("data-testid") === "news-1",
    );
    assert(
      await page.evaluate(() =>
        document.activeElement.textContent.includes("Les marchés européens"),
      ),
      "Retour du focus",
    );
    await click("Hier");
    assert(
      await page.evaluate(
        () =>
          document.querySelector('[aria-pressed="true"]').textContent ===
          "Hier",
      ),
    );
    await click("Aujourd’hui");
    // Aucune écriture du thème sur le profil, même si celui du serveur diffère.
    await page.evaluate(() => localStorage.removeItem("filfinance.theme"));
    await goto("/profile");
    assert.equal(
      await page.evaluate(() => document.documentElement.className),
      "light",
    );
    await click("Sombre");
    assert.equal(
      await page.evaluate(() => localStorage.getItem("filfinance.theme")),
      "dark",
    );
    await page.reload({ waitUntil: "networkidle0" });
    assert.equal(
      await page.evaluate(() => document.documentElement.className),
      "dark",
    );
    assert(
      !requests.some(
        (r) =>
          r.path === "/api/me" && r.method !== "GET" && r.method !== "OPTIONS",
      ),
    );
    for (const role of ["listener", "moderator", "admin"]) {
      state.role = role;
      await goto("/admin");
      const text = await page.evaluate(() => document.body.innerText);
      assert(
        role === "listener"
          ? /Accès|Acces/.test(text)
          : text.includes("Supervision"),
        `Droits ${role}`,
      );
    }
    // Zoom de la mise en page équivalent à 200 %, sans agrandir uniquement les pixels.
    state.role = "admin";
    await page.setViewport({ width: 720, height: 500 });
    for (const route of [
      "/feed",
      "/calendar",
      "/profile",
      "/admin",
      "/admin/logs",
    ]) {
      await goto(route);
      assert.equal(
        await page.evaluate(() => document.documentElement.scrollWidth),
        720,
        `zoom: ${route}`,
      );
    }
    assert.equal(errors.length, 0, errors.join("\n"));
    assert.equal(failures.length, 0, JSON.stringify(failures, null, 2));
    await fs.writeFile(
      path.join(output, `${resultPrefix}browser-results.json`),
      JSON.stringify(
        {
          date: new Date().toISOString(),
          environment: "local fixtures only",
          results,
          checks: [
            "focus detail Escape restore",
            "theme default toggle reload no profile writes",
            "roles listener moderator admin",
            "reflow 720px (viewport équivalent 1440px à 200%)",
          ],
          errors,
        },
        null,
        2,
      ),
    );
    console.log(
      `PASS ${results.length} rendus ; détails, thèmes et rôles vérifiés.`,
    );
  } catch (error) {
    await page.screenshot({ path: "/tmp/ff-qa-failure.png", fullPage: true });
    console.error(
      "PAGE",
      await page.evaluate(() => document.body.innerText.slice(0, 3500)),
    );
    console.error(
      "OVERFLOW",
      await page.evaluate(() =>
        [...document.querySelectorAll("*")]
          .filter((e) => e.getBoundingClientRect().right > innerWidth + 0.1)
          .map((e) => ({
            tag: e.tagName,
            cls: e.className,
            text: e.textContent.slice(0, 100),
            right: e.getBoundingClientRect().right,
            width: e.getBoundingClientRect().width,
          }))
          .slice(-20),
      ),
    );
    console.error("ERRORS", errors);
    throw error;
  } finally {
    await browser.close();
  }
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
