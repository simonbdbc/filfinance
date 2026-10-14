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
];
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

  const checks = [];
  const axeResults = [];
  try {
    if (!process.env.QA_ONLY_ACCESSIBILITY) {
      // Seuils de navigation et du panneau, immédiatement de part et d'autre.
      state.role = "listener";
      await goto("/feed");
      for (const width of [1023, 1024, 1279, 1280]) {
        await page.setViewport({ width, height: 1000 });
        await goto("/feed");
        assert.equal(
          Boolean(await page.$('[aria-label="Ouvrir le menu"]')),
          width < 1024,
        );
        await page.click('[data-testid="news-1"]');
        if (width < 1280) await page.waitForSelector('[role="dialog"]');
        assert.equal(Boolean(await page.$('[role="dialog"]')), width < 1280);
        if (width < 1280) {
          await page.keyboard.press("Escape");
          await page.waitForSelector('[role="dialog"]', { hidden: true });
        }
      }
      checks.push("seuils 1023/1024 et 1279/1280");
      await page.setViewport({ width: 390, height: 844 });
      await goto("/");
      assert(new URL(page.url()).pathname === "/feed");
      checks.push("route racine redirigée vers le fil");
      // Conserver les droits existants : résumé journalier réservé à l’admin.
      for (const role of ["listener", "moderator", "admin"]) {
        state.role = role;
        for (const route of routes.filter((r) => r.startsWith("/admin"))) {
          await goto(route);
          const text = await page.evaluate(() => document.body.innerText);
          const refused =
            /Accès réservé|Acces refuse|Accès refusé|Acces non autorise/.test(
              text,
            );
          const expectedRefusal =
            role === "listener" ||
            (role === "moderator" &&
              ["/admin/calendar-day", "/admin/daily-summary"].includes(route));
          assert.equal(refused, expectedRefusal, role + route);
        }
      }
      checks.push("droits des trois rôles sur toutes les routes admin");
      state.authenticated = false;
      await goto("/login");
      await click("Mot de passe oublié");
      assert.equal(
        await page.$$('input[type="password"]').then((a) => a.length),
        0,
      );
      await page.type('input[placeholder="Adresse e-mail"]', "recette@example.test");
      checks.push("formulaire récupération");
      await goto("/login?token=jeton-factice-recette");
      await page.waitForSelector('input[placeholder="Nouveau mot de passe"]');
      await page.waitForFunction(() => !location.search.includes("token="));
      await page.type('input[placeholder="Nouveau mot de passe"]', "court");
      assert(
        await page.evaluate(() =>
          [...document.querySelectorAll('[role="button"],button')].some(
            (e) =>
              e.textContent.includes("Modifier le mot de passe") &&
              (e.getAttribute("aria-disabled") === "true" || e.disabled),
          ),
        ),
      );
      checks.push(
        "réinitialisation : jeton nettoyé, mot de passe court refusé",
      );
      await goto("/login?error=TOKEN_EXPIRED");
      assert(
        await page.evaluate(() =>
          document.body.innerText.includes("invalide ou expire"),
        ),
      );
      checks.push("lien expiré");
    }
    if(process.env.QA_ONLY_ACCESSIBILITY) await goto("/feed");
    // Audit automatique des noms, contrastes, rôles et formulaires. Pas une simulation de VoiceOver.
    state.role = "admin";
    for (const theme of ["light", "dark"])
      for (const route of (process.env.QA_DETAILS ? ["/feed", "/admin/logs", "/admin"] : routes)) {
        state.authenticated = !["/login", "/email-verification"].includes(
          route,
        );
        await page.evaluate(
          (t) => localStorage.setItem("filfinance.theme", t),
          theme,
        );
        await goto(route);
        await page.addScriptTag({
          path: require.resolve("../../../landing/node_modules/axe-core/axe.min.js"),
        });
        const audit = await page.evaluate(async () => {
          const r = await axe.run(document, {
            runOnly: {
              type: "tag",
              values: ["wcag2a", "wcag2aa", "wcag21a", "wcag21aa"],
            },
          });
          return {
            violations: r.violations.map((v) => ({
              id: v.id,
              impact: v.impact,
              description: v.description,
              nodes: v.nodes.map((n) => ({
                html: n.html,
                target: n.target,
                summary: n.failureSummary,
              })),
            })),
            incomplete: r.incomplete.map((v) => ({
              id: v.id,
              count: v.nodes.length,
              nodes: v.nodes.map(n=>({html:n.html,target:n.target,summary:n.failureSummary})),
            })),
          };
        });
        axeResults.push({ route, theme, ...audit });
      }
    await fs.writeFile(
      path.join(output, process.env.QA_DETAILS ? "accessibility-details.json" : "accessibility-results.json"),
      JSON.stringify(
        { date: new Date().toISOString(), checks, axeResults, errors },
        null,
        2,
      ),
    );
    assert.equal(
      axeResults.filter((r) => r.violations.length).length,
      0,
      JSON.stringify(
        axeResults.filter((r) => r.violations.length),
        null,
        2,
      ),
    );
    assert.equal(errors.length, 0, errors.join("\n"));
    console.log("PASS", checks);
  } finally {
    await browser.close();
  }
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
