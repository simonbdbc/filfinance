const assert = require("node:assert/strict"),
  fs = require("node:fs"),
  path = require("node:path");
const puppeteer = require("../../../landing/node_modules/puppeteer-core");
const { fixture, day } = require("./fixtures.cjs");
const origin = "http://127.0.0.1:8052",
  output = process.env.QA_OUTPUT_DIR || path.resolve(
    __dirname,
    "../../../..",
    "apps/pwa/test-results/public-qa",
  );
(async () => {
  const browser = await puppeteer.launch({
    executablePath:
      process.env.CHROME_PATH ||
      "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
    headless: true,
    args: ["--no-sandbox", "--autoplay-policy=no-user-gesture-required"],
  });
  const page = await browser.newPage();
  page.setDefaultTimeout(12000);
  await page.setViewport({ width: 390, height: 844 });
  let state = { authenticated: true, role: "listener", empty: false };
  const errors = [],
    requests = [],
    checks = [];
  page.on("pageerror", (e) => errors.push(e.message));
  await page.evaluateOnNewDocument((today) => {
    window.__qaSockets = [];
    window.__qaAudios = [];
    window.__qaPlays = 0;
    window.__qaPauses = 0;
    const create = document.createElement.bind(document);
    document.createElement = function (tag, ...args) {
      const el = create(tag, ...args);
      if (tag === "audio") window.__qaAudios.push(el);
      return el;
    };
    const play = HTMLMediaElement.prototype.play,
      pause = HTMLMediaElement.prototype.pause;
    HTMLMediaElement.prototype.play = function () {
      window.__qaPlays++;
      return play.call(this);
    };
    HTMLMediaElement.prototype.pause = function () {
      window.__qaPauses++;
      return pause.call(this);
    };
    class Socket extends EventTarget {
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
          this.readyState = 1;
          const e = new Event("open");
          this.dispatchEvent(e);
          this.onopen?.(e);
          this.emit({
            type: "hello",
            payload: {
              protocol_version: 1,
              sources: [
                { source: "financial_juice", enabled: true, status: "active" },
              ],
              calendar_day_summary: {
                local_day: today,
                status: "audio_ready",
                audio_available: true,
                updated_at: new Date().toISOString(),
              },
            },
          });
        }, 30);
      }
      emit(message) {
        const e = new MessageEvent("message", {
          data: JSON.stringify(message),
        });
        this.dispatchEvent(e);
        this.onmessage?.(e);
      }
      send() {}
      close() {
        this.readyState = 3;
      }
    }
    window.WebSocket = Socket;
  }, day);
  await page.setRequestInterception(true);
  page.on("request", (r) => {
    const u = new URL(r.url());
    if (u.origin === origin || ["blob:", "data:"].includes(u.protocol))
      return void r.continue();
    if (u.origin !== "http://127.0.0.1:9999") return void r.abort();
    requests.push({ method: r.method(), path: u.pathname });
    const headers = {
      "access-control-allow-origin": origin,
      "access-control-allow-credentials": "true",
      "access-control-allow-headers": "content-type,authorization",
      "access-control-allow-methods": "GET,POST,PATCH,OPTIONS",
      "content-type": "application/json",
    };
    if (r.method() === "OPTIONS")
      return void r.respond({ status: 204, headers });
    if (u.pathname.startsWith("/qa-hls/")) {
      const file = path.join("/tmp/ff-hls", path.basename(u.pathname));
      return void r.respond({
        status: 200,
        headers: {
          ...headers,
          "content-type": file.endsWith(".m3u8")
            ? "application/vnd.apple.mpegurl"
            : "video/mp2t",
        },
        body: fs.readFileSync(file),
      });
    }
    if (u.pathname.endsWith("/summary"))
      return void r.respond({
        status: 200,
        headers,
        body: JSON.stringify({
          localDay: day,
          status: "audio_ready",
          currentPlaylistUrl: "http://127.0.0.1:9999/qa-hls/playlist.m3u8",
          immutablePlaylistUrl: "http://127.0.0.1:9999/qa-hls/playlist.m3u8",
          playlistUrl: "http://127.0.0.1:9999/qa-hls/playlist.m3u8",
          title: "Résumé de recette",
          highEventCount: 0,
        }),
      });
    const f = fixture(u, state);
    const reply = () =>
      r.respond({
        status: f.status || 200,
        headers,
        body: JSON.stringify(f.body),
      });
    if (u.pathname === "/api/me" && state.delayProfile) setTimeout(reply, 1500);
    else void reply();
  });
  const textOf = (e) =>
    (e.getAttribute("aria-label") || e.textContent)
      .replace(/[\uE000-\uF8FF]/g, "")
      .trim();
  const click = async (text) => {
    for (const el of await page.$$('[role="button"],button,[role="link"]'))
      if ((await el.evaluate(textOf)) === text) {
        await el.evaluate((e) => e.scrollIntoView({ block: "center" }));
        await el.click();
        return;
      }
    throw Error("Absent: " + text);
  };
  const goto = async (route) => {
    await page.goto(origin + route, { waitUntil: "networkidle0" });
    await page.waitForSelector('[role="heading"]');
  };
  try {
    await goto("/feed");
    await page.waitForSelector('[data-testid="news-1"]');
    const row = await page.$('[data-testid="news-1"]');
    await row.focus();
    await page.keyboard.press("Enter");
    await page.waitForSelector('[aria-label="Fermer le panneau"]');
    assert.equal(
      await page.evaluate(() =>
        document.activeElement.getAttribute("aria-label"),
      ),
      "Fermer le panneau",
    );
    await page.keyboard.press("Tab");
    assert(
      await page.evaluate(() =>
        Boolean(document.activeElement.closest('[role="dialog"]')),
      ),
      "Focus confiné",
    );
    await page.keyboard.press("Escape");
    await page.waitForSelector('[aria-label="Fermer le panneau"]', {
      hidden: true,
    });
    await page.waitForFunction(
      () => document.activeElement?.getAttribute("data-testid") === "news-1",
    );
    assert.equal(
      await page.evaluate(() =>
        document.activeElement.getAttribute("data-testid"),
      ),
      "news-1",
    );
    checks.push("détail clavier : focus initial, confinement, Échap, retour");
    await click("Écouter le direct");
    await page.waitForFunction(
      () => !!document.querySelector('[aria-label="Arrêter le direct"]'),
    );
    await page.evaluate(() =>
      window.__qaSockets
        .find((s) => s.url.includes("/ws/news"))
        .emit({
          type: "audio_ready",
          payload: {
            event_id: "qa-audio",
            server_time: new Date().toISOString(),
            translation_id: 1,
            stream_id: "qa-stream",
            playlist_url: "http://127.0.0.1:9999/qa-hls/playlist.m3u8",
            published_at: new Date().toISOString(),
            target_duration_secs: 4,
          },
        }),
    );
    await page.waitForFunction(() =>
      window.__qaAudios.some((a) => !a.paused && a.currentTime > 0.1),
    );
    async function continuity(label) {
      const before = await page.evaluate(() => ({
        plays: window.__qaPlays,
        pauses: window.__qaPauses,
        audios: window.__qaAudios.length,
        sockets: window.__qaSockets.length,
        time: window.__qaAudios.at(-1).currentTime,
      }));
      await page.evaluate(() => {
        window.__qaPlayer = document.querySelector(
          '[data-testid="session-audio"]',
        );
      });
      await click("Calendrier");
      await page.waitForFunction(() =>
        document.body.innerText.includes("Calendrier économique"),
      );
      await page.mouse.move(350, 550);
      await page.mouse.wheel({ deltaY: 400 });
      await page.waitForFunction(() =>
        document.querySelector('[data-testid="page-scroll"]').scrollTop > 0,
      );
      await click("Basculer entre les thèmes clair et sombre");
      await click("Profil");
      await page.waitForFunction(() =>
        document.body.innerText.includes("Mon compte"),
      );
      const after = await page.evaluate(() => ({
        plays: window.__qaPlays,
        pauses: window.__qaPauses,
        audios: window.__qaAudios.length,
        sockets: window.__qaSockets.length,
        time: window.__qaAudios.at(-1).currentTime,
        paused: window.__qaAudios.at(-1).paused,
        samePlayer:
          window.__qaPlayer ===
          document.querySelector('[data-testid="session-audio"]'),
      }));
      assert.equal(after.plays, before.plays);
      assert.equal(after.pauses, before.pauses);
      assert.equal(after.audios, before.audios);
      assert.equal(after.sockets, before.sockets);
      assert(after.time >= before.time && !after.paused && after.samePlayer);
      checks.push({ label, before, after });
    }
    await continuity(
      "direct HLS réel : navigation et thème sans recréation ni arrêt",
    );
    await click("Arrêter le direct");
    await click("Résumé audio du jour");
    await page.waitForFunction(
      () =>
        window.__qaAudios.at(-1) &&
        !window.__qaAudios.at(-1).paused &&
        window.__qaAudios.at(-1).currentTime > 0.1,
    );
    await continuity(
      "résumé HLS réel : navigation et thème sans recréation ni arrêt",
    );
    await click("Arrêter la lecture");
    assert(!requests.some((r) => r.method === "PATCH" && r.path === "/api/me"));
    checks.push("aucune synchronisation du thème au serveur");
    // Stockage refusé : la visite reste interactive, y compris pendant la restauration du profil.
    const storageScript = await page.evaluateOnNewDocument(() => {
      Object.defineProperty(window, "localStorage", {
        get() {
          throw new DOMException("Refus de recette", "SecurityError");
        },
      });
    });
    await goto("/profile");
    assert.equal(
      await page.evaluate(() => document.documentElement.className),
      "light",
    );
    await click("Sombre");
    await page.waitForFunction(
      () => document.documentElement.className === "dark",
    );
    assert.equal(
      await page.evaluate(() => document.documentElement.className),
      "dark",
    );
    await page.removeScriptToEvaluateOnNewDocument(storageScript.identifier);
    checks.push(
      "stockage refusé : clair initial et bascule conservée en mémoire",
    );
    // Le choix utilisateur précède une réponse de profil lente et contradictoire.
    await goto("/profile");
    await click("Clair");
    await page.reload({ waitUntil: "networkidle0" });
    assert.equal(
      await page.evaluate(() => document.documentElement.className),
      "light",
    );
    checks.push("profil serveur sombre ignoré, choix local clair restauré");
    state.delayProfile = true;
    await page.evaluate(() => localStorage.setItem("filfinance.theme", "dark"));
    await page.goto(origin + "/profile", { waitUntil: "domcontentloaded" });
    await page.waitForSelector('[aria-label="Basculer entre les thèmes clair et sombre"]');
    await click("Basculer entre les thèmes clair et sombre");
    await page.waitForFunction(
      () => document.documentElement.className === "light",
    );
    await page.waitForFunction(() =>
      document.body.innerText.includes("Mon compte"),
    );
    assert.equal(
      await page.evaluate(() => document.documentElement.className),
      "light",
    );
    state.delayProfile = false;
    checks.push(
      "choix explicite conservé pendant une réponse de profil tardive",
    );
    state.authenticated = false;
    await goto("/login");
    await page.waitForSelector('input[placeholder="Adresse e-mail"]');
    await page.type('input[placeholder="Adresse e-mail"]', "brouillon@example.test");
    await click("Basculer entre les thèmes clair et sombre");
    assert.equal(
      await page.$eval('input[placeholder="Adresse e-mail"]', (e) => e.value),
      "brouillon@example.test",
    );
    await click("Créer un compte");
    await page.waitForSelector('input[placeholder="Nom"]');
    const links = await page.$$eval('a[target="_blank"]', (els) =>
      els.map((e) => ({ href: e.href, rel: e.rel })),
    );
    assert(links.length >= 4);
    assert(
      links.every(
        (l) => l.rel.includes("noopener") && l.rel.includes("noreferrer"),
      ),
    );
    assert.equal(
      await page.$$eval('[data-testid="session-audio"]', (a) => a.length),
      0,
    );
    checks.push(
      "inscription et liens légaux : formulaire préservé, aucun lecteur",
    );
    state.authenticated = true;
    state.empty = true;
    await goto("/feed");
    assert(
      await page.evaluate(() =>
        document.body.innerText.includes("Aucune actualité"),
      ),
    );
    checks.push("état vide du fil");
    state.empty = false;
    state.detailError = true;
    await goto("/feed");
    await page.click('[data-testid="news-1"]');
    await page.waitForFunction(() =>
      document.body.innerText.includes(
        "Impossible de charger le détail complet.",
      ),
    );
    state.detailError = false;
    await click("Réessayer");
    await page.waitForFunction(() =>
      document.body.innerText.includes("Les investisseurs suivent"),
    );
    await page.keyboard.press("Escape");
    checks.push("détail en erreur puis relance réussie");
    const beforeReconnect = await page.evaluate(
      () => window.__qaSockets.length,
    );
    await page.evaluate(() => {
      const s = window.__qaSockets.at(-1);
      s.readyState = 3;
      s.onclose?.({ code: 1006, reason: "Recette locale" });
    });
    await page.waitForFunction(
      (n) =>
        window.__qaSockets.length > n &&
        window.__qaSockets.at(-1).readyState === 1,
      {},
      beforeReconnect,
    );
    checks.push("reconnexion du fil après coupure WebSocket simulée");
    state.role = "admin";
    await goto("/admin/logs");
    await page.select('select[aria-label="Niveau des logs"]', "error");
    assert.equal(
      await page.$eval('select[aria-label="Niveau des logs"]', (e) => e.value),
      "error",
    );
    checks.push("sélecteur gluestack : changement effectif du niveau de logs");
    assert.equal(errors.length, 0, errors.join("\n"));
    fs.mkdirSync(output, { recursive: true });
    fs.writeFileSync(
      path.join(output, "interactions-results.json"),
      JSON.stringify(
        {
          date: new Date().toISOString(),
          checks,
          errors,
          scope:
            "API et WebSocket simulés, média HLS silencieux réellement décodé localement",
        },
        null,
        2,
      ),
    );
    console.log("PASS", checks);
  } catch (e) {
    await page.screenshot({
      path: "/tmp/ff-interactions-failure.png",
      fullPage: true,
    });
    console.error(
      "PAGE",
      await page.evaluate(() => document.body.innerText.slice(0, 2500)),
    );
    console.error("ERRORS", errors);
    throw e;
  } finally {
    await browser.close();
  }
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
