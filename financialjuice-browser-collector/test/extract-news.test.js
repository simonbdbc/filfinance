import assert from "node:assert/strict";
import test from "node:test";

import {
  expandNewsHistoryFromPage,
  extractNewsFromPageWithDiagnostics,
  extractNewsCandidatesFromText,
  normalizeNewsCandidates,
  normalizeNewsCandidatesWithDiagnostics,
  normalizeWhitespace,
} from "../src/extract-news.js";

const config = {
  homeUrl: "https://www.financialjuice.com/home",
  maxBatchSize: 100,
};

test("normalizeWhitespace collapses source spacing", () => {
  assert.equal(normalizeWhitespace("  Fed   signals\nrates\tstay high "), "Fed signals rates stay high");
});

test("normalizeNewsCandidates maps DOM candidates to backend payload", () => {
  const events = normalizeNewsCandidates(
    [
      {
        externalId: "9566991",
        title: "  Fed   signals rates stay high  ",
        description: "  Market detail  ",
        url: "/news/9566991",
        publishedAt: "2026-05-06T12:00:00Z",
        newsLevel: "active active-critical",
      },
    ],
    config,
  );

  assert.deepEqual(events, [
    {
      external_id: "9566991",
      title: "Fed signals rates stay high",
      description: "Market detail",
      url: "https://www.financialjuice.com/news/9566991",
      published_at: "2026-05-06T12:00:00.000Z",
      news_level: "active active-critical",
    },
  ]);
});

test("normalizeNewsCandidates drops challenge-looking text", () => {
  const events = normalizeNewsCandidates(
    [
      {
        externalId: "cf",
        title: "Checking your browser before accessing FinancialJuice",
      },
    ],
    config,
  );

  assert.deepEqual(events, []);
});

test("normalizeNewsCandidatesWithDiagnostics counts rejection reasons", () => {
  const result = normalizeNewsCandidatesWithDiagnostics(
    [
      { title: "Sign Out" },
      { title: "Fed's Hammack concludes remarks", publishedAt: "2026-05-07T15:31:00Z" },
      { title: "Fed's Hammack concludes remarks", publishedAt: "2026-05-07T15:31:00Z" },
      { title: "Fed's Williams Speaks", publishedAt: "2026-05-07T20:10:00Z" },
    ],
    { ...config, nowIso: "2026-05-07T15:32:00.000Z" },
  );

  assert.equal(result.events.length, 1);
  assert.deepEqual(result.diagnostics, {
    candidates: 4,
    normalized: 1,
    rejected: {
      duplicate_title_time: 1,
      future: 1,
      ui_chrome: 1,
    },
  });
});

test("normalizeNewsCandidates drops FinancialJuice chrome and promotions", () => {
  const events = normalizeNewsCandidates(
    [
      { title: "Sign Out" },
      { title: "News Settings" },
      { title: "My Account" },
      { title: "© 2026 FinancialJuice" },
      { title: "Partners" },
      { title: "Affiliates" },
      { title: "TradingView" },
      { title: "Disclaimer" },
      { title: "Privacy Policy" },
      { title: "Contact Us" },
      { title: "Search ×" },
      { title: "[11:24 ET] Iran imposes new rules for Strait of Hormuz passage - CNN" },
      { title: "Join us and Go Real-time" },
      { title: "Don't like Ads? GO PRO" },
    ],
    config,
  );

  assert.deepEqual(events, []);
});

test("normalizeNewsCandidates drops calendar event header rows", () => {
  const events = normalizeNewsCandidates(
    [
      {
        title: "US Consumer Credit - 13.72B 9.48B",
        publishedAt: "2026-05-07T19:00:00Z",
        newsLevel: "event-header",
      },
      {
        title: "Coinbase Q1 2026 Earnings - - -",
        publishedAt: "2026-05-07T20:10:00Z",
        newsLevel: "economic-calendar event-header",
      },
    ],
    config,
  );

  assert.deepEqual(events, []);
});

test("normalizeNewsCandidates drops browser fallback calendar dates and market labels", () => {
  const events = normalizeNewsCandidates(
    [
      {
        title: "December 18",
        publishedAt: "2026-05-07T21:00:00Z",
      },
      {
        title: "US Indexes",
        publishedAt: "2026-05-07T15:26:00Z",
      },
      {
        title: "Energy",
        publishedAt: "2026-05-07T15:26:00Z",
      },
    ],
    { ...config, nowIso: "2026-05-07T15:32:00.000Z" },
  );

  assert.deepEqual(events, []);
});

test("normalizeNewsCandidates drops future FinancialJuice browser rows", () => {
  const events = normalizeNewsCandidates(
    [
      {
        title: "Fed's Williams Speaks",
        publishedAt: "2026-05-07T20:10:00Z",
      },
      {
        title: "Fed's Hammack concludes remarks",
        publishedAt: "2026-05-07T15:31:00Z",
      },
    ],
    { ...config, nowIso: "2026-05-07T15:32:00.000Z" },
  );

  assert.equal(events.length, 1);
  assert.equal(events[0]?.title, "Fed's Hammack concludes remarks");
});

test("normalizeNewsCandidates keeps live FinancialJuice headlines", () => {
  const events = normalizeNewsCandidates(
    [
      {
        externalId: "live-16-40",
        title: "Fed's Hammack concludes remarks",
        publishedAt: "2026-05-07T14:40:00Z",
        newsLevel: "US Bonds US Indexes USD",
      },
      {
        externalId: "live-16-39",
        title:
          "A source close to the Saudi Arabian government, in an interview with AFP, denied US media reports about Trump's withdrawal from operations related to the Strait of Hormuz - Iran International",
        publishedAt: "2026-05-07T14:39:00Z",
        newsLevel: "Energy US Bonds US Indexes USD",
      },
    ],
    config,
  );

  assert.equal(events.length, 2);
  assert.equal(events[0]?.title, "Fed's Hammack concludes remarks");
  assert.equal(events[1]?.title.startsWith("A source close to the Saudi Arabian government"), true);
});

test("extractNewsCandidatesFromText reads visible live feed title/time pairs", () => {
  const candidates = extractNewsCandidatesFromText(
    `
Fed's Hammack concludes remarks

16:40 May 07US BondsUS IndexesUSD

A source close to the Saudi Arabian government, in an interview with AFP, denied US media reports about Trump's withdrawal from operations related to the Strait of Hormuz - Iran International

16:39 May 07EnergyUS BondsUS IndexesUSD

EIA Natural Gas Change BCF Actual 63B (Forecast 72B, Previous 79B)

16:30US IndexesUS BondsUSDForex

Join us and Go Real-time

Don't like Ads? GO PRO
`,
    {
      nowIso: "2026-05-07T14:41:00.000Z",
      timezoneOffsetMinutes: -120,
    },
  );
  const events = normalizeNewsCandidates(candidates, config);

  assert.equal(events.length, 3);
  assert.deepEqual(
    events.map((event) => [event.title, event.published_at, event.news_level]),
    [
      [
        "Fed's Hammack concludes remarks",
        "2026-05-07T14:40:00.000Z",
        "US BondsUS IndexesUSD",
      ],
      [
        "A source close to the Saudi Arabian government, in an interview with AFP, denied US media reports about Trump's withdrawal from operations related to the Strait of Hormuz - Iran International",
        "2026-05-07T14:39:00.000Z",
        "EnergyUS BondsUS IndexesUSD",
      ],
      [
        "EIA Natural Gas Change BCF Actual 63B (Forecast 72B, Previous 79B)",
        "2026-05-07T14:30:00.000Z",
        "US IndexesUS BondsUSDForex",
      ],
    ],
  );
});

test("extractNewsFromPageWithDiagnostics falls back to visible mainFeed text", async () => {
  const page = {
    evaluate: async () => ({
      candidates: [{ title: "Sign Out" }],
      mainFeedText: `
Fed's Hammack concludes remarks

16:40 May 07US BondsUS IndexesUSD
`,
      timezoneOffsetMinutes: -120,
    }),
  };

  const result = await extractNewsFromPageWithDiagnostics(page, {
    ...config,
    nowIso: "2026-05-07T14:41:00.000Z",
  });

  assert.equal(result.diagnostics.source, "text");
  assert.equal(result.diagnostics.dom.normalized, 0);
  assert.equal(result.diagnostics.fallbackText.normalized, 1);
  assert.equal(result.events[0]?.title, "Fed's Hammack concludes remarks");
});

test("expandNewsHistoryFromPage loads current and previous day history", async () => {
  const page = createHistoryPage(
    [
      historyItem("300", "Latest headline", minutesAgo(5)),
      historyItem("299", "Older headline", minutesAgo(10)),
    ],
    [
      [
        historyItem("250", "Previous day headline", hoursAgo(24)),
        historyItem("249", "Previous day older headline", hoursAgo(25)),
      ],
      [
        historyItem("150", "Too old headline", hoursAgo(48)),
      ],
    ],
  );

  const result = await expandNewsHistoryFromPage(page, {
    ...config,
    historyPageLimit: 10,
    historyTimezone: "Europe/Paris",
    requestTimeoutMs: 1_000,
    nowIso: new Date(Date.now() + 60_000).toISOString(),
  });

  assert.equal(result.pagesLoaded, 2);
  assert.equal(result.stopReason, "older_than_previous_day");
  assert.equal(result.domItems, 5);
  assert.deepEqual(page.loadCalls, ["299", "249"]);
});

function createHistoryPage(initialItems, pages) {
  const state = {
    items: [...initialItems],
    pages: pages.map((page) => [...page]),
    loadCalls: [],
  };
  return {
    get loadCalls() {
      return state.loadCalls;
    },
    evaluate: async (_fn, arg) => {
      if (arg?.mainFeedRootSelector) {
        return {
          candidates: state.items,
          mainFeedText: "",
          timezoneOffsetMinutes: 0,
        };
      }
      if (arg?.mainFeedSelector) {
        return {
          domItems: state.items.length,
          firstHeadlineId: state.items.at(0)?.externalId ?? null,
          lastHeadlineId: state.items.at(-1)?.externalId ?? null,
          loadPreviousNewsAvailable: true,
          objHTTPBusy: false,
        };
      }
      state.loadCalls.push(String(arg));
      state.items.push(...(state.pages.shift() ?? []));
      return { ok: true };
    },
    waitForFunction: async () => {},
  };
}

function historyItem(externalId, title, publishedAt) {
  return {
    externalId,
    title,
    description: "",
    url: `/news/${externalId}`,
    publishedAt,
    newsLevel: "Energy",
  };
}

function minutesAgo(minutes) {
  return new Date(Date.now() - minutes * 60_000).toISOString();
}

function hoursAgo(hours) {
  return new Date(Date.now() - hours * 60 * 60_000).toISOString();
}
