const assert = require("node:assert/strict"),
  fs = require("node:fs"),
  path = require("node:path");
const puppeteer = require("../../../landing/node_modules/puppeteer-core");
const { fixture, day } = require("./fixtures.cjs");
const origin = "http://127.0.0.1:8059",
  output =
    process.env.QA_OUTPUT_DIR ||
    path.resolve(
      __dirname,
      "../../../..",
      "docs/design/pwa/recette/2026-09-17-auth",
    );
(async () => {
  const mediaDir = fs.mkdtempSync(
    path.join(require("node:os").tmpdir(), "filfinance-auth-hls-"),
  );
  require("node:child_process").execFileSync(
    process.env.FFMPEG_PATH || "ffmpeg",
    [
      "-hide_banner",
      "-loglevel",
      "error",
      "-f",
      "lavfi",
      "-i",
      "sine=frequency=220:sample_rate=24000",
      "-t",
      "30",
      "-c:a",
      "aac",
      "-b:a",
      "32k",
      "-f",
      "hls",
      "-hls_time",
      "4",
      "-hls_list_size",
      "0",
      path.join(mediaDir, "playlist.m3u8"),
    ],
  );
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
  let tokenCount = 0;
  let forceDetail401 = false;
  let lastToken = "";
  let failRenewal = false;
  const errors = [],
    requests = [],
    checks = [];
  page.on("pageerror", (e) => errors.push(e.message));
  await page.evaluateOnNewDocument((today) => {
    window.__FILFINANCE_ENV__ = {
      EXPO_PUBLIC_BETTER_AUTH_URL: "http://127.0.0.1:9999",
      EXPO_PUBLIC_FILFINANCE_WS_URL: "ws://127.0.0.1:9999/ws/news",
      EXPO_PUBLIC_FILFINANCE_EDGE_WS_URL: "ws://127.0.0.1:9999/ws/news",
      EXPO_PUBLIC_FILFINANCE_HLS_BASE_URL: "http://127.0.0.1:9999",
      EXPO_PUBLIC_LANDING_ORIGIN: "https://filfinance.fr",
    };
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
      send(message) {
        if (JSON.parse(message).type === "auth") this.authenticated = true;
      }
      expire() {
        this.readyState = 3;
        this.onclose?.(
          new CloseEvent("close", { code: 1008, reason: "auth token expired" }),
        );
      }
      close() {
        this.readyState = 3;
      }
    }
    window.WebSocket = Socket;
  }, day);
  await page.setRequestInterception(true);
  page.on("request", (r) => {
    const u = new URL(r.url());
    if (["blob:", "data:"].includes(u.protocol)) return void r.continue();
    if (u.origin === origin) {
      const root = path.resolve(__dirname, "../../dist");
      const candidate = path.join(root, u.pathname);
      const file =
        fs.existsSync(candidate) && fs.statSync(candidate).isFile()
          ? candidate
          : path.join(root, "index.html");
      const types = {
        ".html": "text/html",
        ".js": "text/javascript",
        ".css": "text/css",
        ".png": "image/png",
        ".woff": "font/woff",
        ".woff2": "font/woff2",
      };
      return void r.respond({
        status: 200,
        contentType: types[path.extname(file)] || "application/octet-stream",
        body: fs.readFileSync(file),
      });
    }
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
      const file = path.join(mediaDir, path.basename(u.pathname));
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
    if (u.pathname.includes("edge") && u.pathname.includes("ticket"))
      return void r.respond({
        status: 200,
        headers,
        body: JSON.stringify({
          ticket: "qa-ticket",
          expires_at: new Date(Date.now() + 900000).toISOString(),
        }),
      });
    if (u.pathname.endsWith("/token")) {
      tokenCount++;
      if (failRenewal)
        return void r.respond({
          status: 503,
          headers,
          body: JSON.stringify({ message: "temporary outage" }),
        });
      lastToken = `eyJhbGciOiJSUzI1NiJ9.${Buffer.from(JSON.stringify({ sub: "ui-qa-user", exp: Math.floor(Date.now() / 1000) + (tokenCount === 1 ? 68 : 900), jti: tokenCount })).toString("base64url")}.test`;
      return void r.respond({
        status: 200,
        headers,
        body: JSON.stringify({ token: lastToken }),
      });
    }
    if (forceDetail401 && /^\/api\/translations\/\d+$/.test(u.pathname)) {
      forceDetail401 = false;
      return void r.respond({ status: 401, headers, body: "{}" });
    }
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
  const historyCount = () =>
    requests.filter((r) => r.path === "/api/translations").length;
  async function captureBaseline() {
    return page.evaluate(() => {
      const row = document.querySelector('[data-testid="news-1"]');
      const scroll = document.querySelector('[data-testid="page-scroll"]');
      window.__baseline = {
        row,
        scroll,
        player: document.querySelector('[data-testid="session-audio"]'),
        top: scroll.scrollTop,
        plays: window.__qaPlays,
        pauses: window.__qaPauses,
        audio: window.__qaAudios.at(-1),
        time: window.__qaAudios.at(-1)?.currentTime,
      };
      window.__lost = [];
      window.__observer?.disconnect();
      window.__observer = new MutationObserver(() => {
        if (!row.isConnected || row.getAttribute("aria-expanded") !== "true")
          window.__lost.push("selection/list removed");
        if (scroll !== document.querySelector('[data-testid="page-scroll"]'))
          window.__lost.push("page replaced");
      });
      window.__observer.observe(document.body, {
        childList: true,
        subtree: true,
        attributes: true,
        attributeFilter: ["aria-expanded"],
      });
      return { sockets: window.__qaSockets.length };
    });
  }
  async function assertPreserved(label, audio) {
    const result = await page.evaluate(() => ({
      lost: window.__lost,
      playerSame:
        window.__baseline.player ===
        document.querySelector('[data-testid="session-audio"]'),
      selected: window.__baseline.row.getAttribute("aria-expanded"),
      scrollSame: window.__baseline.top === window.__baseline.scroll.scrollTop,
      plays: window.__qaPlays - window.__baseline.plays,
      pauses: window.__qaPauses - window.__baseline.pauses,
      audioSame: window.__baseline.audio === window.__qaAudios.at(-1),
      audioPlaying: !window.__qaAudios.at(-1)?.paused,
      audioAdvanced:
        window.__qaAudios.at(-1)?.currentTime >= window.__baseline.time,
    }));
    assert.deepEqual(result.lost, [], label);
    assert(result.playerSame, label);
    assert.equal(result.selected, "true", label);
    assert(result.scrollSame, label);
    if (audio) {
      assert.equal(result.plays, 0, label);
      assert.equal(result.pauses, 0, label);
      assert(result.audioSame, label);
      assert(result.audioPlaying, label);
      assert(result.audioAdvanced, label);
    }
    checks.push({ label, result });
  }
  try {
    fs.mkdirSync(output, { recursive: true });
    for (const role of ["listener", "moderator", "admin"]) {
      state.role = role;
      await page
        .evaluate(
          (dark) =>
            localStorage.setItem("filfinance.theme", dark ? "dark" : "light"),
          role === "moderator",
        )
        .catch(() => {});
      tokenCount = 0;
      await page.setViewport({
        width: role === "moderator" ? 390 : 1440,
        height: 900,
      });
      await goto("/feed");
      await page.waitForSelector('[data-testid="news-1"]');
      if (role === "listener") {
        await click("Écouter le direct");
        await page.waitForFunction(() =>
          window.__qaSockets.some(
            (s) => s.url.includes("/ws/news") && s.readyState === 1,
          ),
        );
        await page.evaluate(() =>
          window.__qaSockets
            .find((s) => s.url.includes("/ws/news"))
            .emit({
              type: "audio_ready",
              payload: {
                event_id: "auth-qa-audio",
                server_time: new Date().toISOString(),
                translation_id: 1,
                stream_id: "auth-stream",
                playlist_url: "http://127.0.0.1:9999/qa-hls/playlist.m3u8",
                published_at: new Date().toISOString(),
                target_duration_secs: 4,
              },
            }),
        );
        await page.waitForFunction(() =>
          window.__qaAudios.some((a) => !a.paused && a.currentTime > 0.1),
        );
      }
      if (role === "admin") {
        await click("Résumé audio du jour");
        await page.waitForFunction(() =>
          window.__qaAudios.some((a) => !a.paused && a.currentTime > 0.1),
        );
      }
      await page.click('[data-testid="news-1"]');
      await page.waitForFunction(
        () =>
          document
            .querySelector('[data-testid="news-1"]')
            ?.getAttribute("aria-expanded") === "true",
      );
      await page.waitForFunction(
        () => !document.querySelector('[role="progressbar"]'),
      );
      const before = await captureBaseline();
      const count = historyCount();
      await page.screenshot({ path: path.join(output, `${role}-before.png`) });
      await new Promise((resolve, reject) => {
        const start = Date.now();
        const timer = setInterval(() => {
          if (tokenCount >= 2) {
            clearInterval(timer);
            resolve();
          } else if (Date.now() - start > 15000) {
            clearInterval(timer);
            reject(Error("Preventive refresh missing"));
          }
        }, 100);
      });
      await new Promise((r) => setTimeout(r, 300));
      await assertPreserved(
        `${role}: preventive renewal`,
        role !== "moderator",
      );
      assert.equal(historyCount(), count, "No history reload");
      assert.equal(
        await page.evaluate(() => window.__qaSockets.length),
        before.sockets,
        "No socket restarted on rotation",
      );
      await page.evaluate(() =>
        window.__qaSockets
          .find((s) => s.url.includes("/ws/news") && s.readyState === 1)
          .expire(),
      );
      await page.waitForFunction(
        (n) => window.__qaSockets.length > n,
        {},
        before.sockets,
      );
      await new Promise((r) => setTimeout(r, 150));
      await assertPreserved(`${role}: socket expiry`, role !== "moderator");
      assert.equal(tokenCount, 2, "Socket reuses latest token");
      assert.equal(historyCount(), count);
      const liveItem = {
        ...fixture(new URL("http://fixture/api/translations/1")).body,
        id: 999,
        status: "ready",
        source_title: "Nouvelle actualité de recette",
        translated_title: "Nouvelle actualité de recette",
        news_event_id: 999,
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
      };
      await page.evaluate(
        (item) =>
          window.__qaSockets
            .find((s) => s.url.includes("/ws/news") && s.readyState === 1)
            .emit({ type: "translated_news_item", payload: item }),
        liveItem,
      );
      await page.waitForSelector('[data-testid="news-999"]');
      await assertPreserved(`${role}: new news inserted`, role !== "moderator");
      assert.equal(
        await page.$eval('[data-testid^="news-"]', (e) =>
          e.getAttribute("data-testid"),
        ),
        "news-999",
      );
      await page.screenshot({ path: path.join(output, `${role}-after.png`) });
      await page.evaluate(() => window.__observer.disconnect());
    }
    // A real component request receives 401, is renewed and replayed, keeping its detail.
    forceDetail401 = true;
    await page.click('[data-testid="news-2"]');
    await page.waitForFunction(
      () =>
        document
          .querySelector('[data-testid="news-2"]')
          ?.getAttribute("aria-expanded") === "true" &&
        !document.querySelector('[role="progressbar"]'),
    );
    assert.equal(tokenCount, 3);
    checks.push({ label: "Detail 401 renewed and replayed" });
    // An explicit 403 is covered by the transport tests; user roles are supplied by profile fixtures.
    assert.deepEqual(errors, []);
    fs.writeFileSync(
      path.join(output, "results.json"),
      JSON.stringify(
        { at: new Date().toISOString(), checks, errors, requests },
        null,
        2,
      ),
    );
    console.log(JSON.stringify({ checks: checks.length, errors, output }));
  } catch (error) {
    fs.mkdirSync(output, { recursive: true });
    fs.writeFileSync(
      path.join(output, "failure.json"),
      JSON.stringify(
        { message: error.message, checks, errors, requests },
        null,
        2,
      ),
    );
    throw error;
  } finally {
    await browser.close();
    fs.rmSync(mediaDir, { recursive: true, force: true });
  }
})().catch((e) => {
  console.error(e);
  process.exitCode = 1;
});
