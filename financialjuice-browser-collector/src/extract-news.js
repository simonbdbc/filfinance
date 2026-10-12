import { createHash } from "node:crypto";

const MAIN_FEED_ROOT_SELECTOR = "#mainFeed[data-feedid='mainfeed']";
const MAIN_FEED_SELECTOR = `${MAIN_FEED_ROOT_SELECTOR} .headline-item[data-headlineid]`;

const CHALLENGE_MARKERS = [
  "checking your browser",
  "cloudflare",
  "managed challenge",
  "verify you are human",
  "just a moment",
];

const BLOCKED_NEWS_LEVEL_MARKERS = ["event-header"];
const FUTURE_PUBLISHED_AT_SKEW_MS = 2 * 60 * 1000;
const HISTORY_LOAD_TIMEOUT_MS = 8_000;

const UI_CHROME_TITLES = new Set([
  "affiliates",
  "china",
  "cny",
  "contact us",
  "crypto",
  "disclaimer",
  "energy",
  "eur",
  "europe",
  "forex",
  "germany",
  "my account",
  "news settings",
  "partners",
  "privacy policy",
  "search",
  "search x",
  "sign out",
  "tradingview",
  "us bonds",
  "us indexes",
  "us stocks",
  "usd",
]);

const UI_CHROME_TITLE_PATTERNS = [
  /^\[\d{1,2}:\d{2}\s+et\]/u,
  /^copyright\s+\d{4}\s+financialjuice$/u,
  /^\d{4}\s+financialjuice$/u,
  /^don'?t like ads\?\s+go pro$/u,
  /^join us and go real-time$/u,
];

export async function extractNewsFromPage(page, config) {
  const result = await extractNewsFromPageWithDiagnostics(page, config);
  return result.events;
}

export async function extractNewsFromPageWithDiagnostics(page, config) {
  const snapshot = await page.evaluate(readNewsSnapshotFromDom, {
    mainFeedRootSelector: MAIN_FEED_ROOT_SELECTOR,
    mainFeedSelector: MAIN_FEED_SELECTOR,
  });
  const nowIso = config.nowIso ?? new Date().toISOString();
  const configWithNow = { ...config, nowIso };
  const dom = normalizeNewsCandidatesWithDiagnostics(snapshot.candidates, configWithNow);
  let source = "dom";
  let events = dom.events;
  let fallbackText = null;

  if (events.length === 0 && snapshot.mainFeedText) {
    const textCandidates = extractNewsCandidatesFromText(snapshot.mainFeedText, {
      nowIso,
      timezoneOffsetMinutes: snapshot.timezoneOffsetMinutes,
    });
    fallbackText = normalizeNewsCandidatesWithDiagnostics(textCandidates, configWithNow);
    if (fallbackText.events.length > 0) {
      source = "text";
      events = fallbackText.events;
    }
  }

  return {
    diagnostics: {
      dom: dom.diagnostics,
      fallbackText: fallbackText?.diagnostics ?? null,
      source,
    },
    events,
  };
}

export async function expandNewsHistoryFromPage(page, config) {
  const timezone = config.historyTimezone ?? "Europe/Paris";
  const previousDayKey = localDayKeyDaysAgo(1, timezone);
  let pagesLoaded = 0;
  let stopReason = "page_limit";
  let lastOldId = null;

  while (pagesLoaded < config.historyPageLimit) {
    const state = await readVisibleNewsHistoryState(page, config);
    if (state.oldestDayKey && state.oldestDayKey < previousDayKey) {
      stopReason = "older_than_previous_day";
      return { pagesLoaded, stopReason, ...state };
    }

    const oldId = state.lastHeadlineId ?? state.oldestEventId;
    if (!oldId) {
      stopReason = "no_old_id";
      return { pagesLoaded, stopReason, ...state };
    }
    if (oldId === lastOldId) {
      stopReason = "no_progress";
      return { pagesLoaded, stopReason, ...state };
    }
    lastOldId = oldId;

    const call = await requestPreviousNewsPage(page, oldId);
    if (!call.ok) {
      stopReason = `load_failed:${call.reason}`;
      return { pagesLoaded, stopReason, ...state };
    }

    const loaded = await waitForPreviousNewsPage(page, state.domItems, state.lastHeadlineId, config);
    pagesLoaded += 1;
    if (!loaded.changed) {
      const nextState = await readVisibleNewsHistoryState(page, config);
      stopReason = loaded.reason;
      return { pagesLoaded, stopReason, ...nextState };
    }
  }

  return { pagesLoaded, stopReason, ...(await readVisibleNewsHistoryState(page, config)) };
}

export async function detectBlockedPage(page) {
  const snapshot = await page.evaluate(() => ({
    title: document.title ?? "",
    body: document.body?.innerText?.slice(0, 2_000) ?? "",
  }));
  const text = normalizeWhitespace(`${snapshot.title} ${snapshot.body}`).toLowerCase();
  return CHALLENGE_MARKERS.some((marker) => text.includes(marker));
}

export function normalizeNewsCandidates(candidates, config) {
  return normalizeNewsCandidatesWithDiagnostics(candidates, config).events;
}

export function normalizeNewsCandidatesWithDiagnostics(candidates, config) {
  const seen = new Set();
  const seenTitleTimes = new Set();
  const events = [];
  const rejected = {};

  const reject = (reason) => {
    rejected[reason] = (rejected[reason] ?? 0) + 1;
  };

  for (const candidate of candidates) {
    const title = normalizeWhitespace(candidate.title);
    const titleRejectReason = unusableTitleReason(title, candidate);
    if (titleRejectReason) {
      reject(titleRejectReason);
      continue;
    }

    const publishedAt = normalizePublishedAt(candidate.publishedAt);
    if (isFuturePublishedAt(publishedAt, config.nowIso)) {
      reject("future");
      continue;
    }
    const titleTimeKey = `${publishedAt ?? "undated"}|${title.toLowerCase()}`;
    if (seenTitleTimes.has(titleTimeKey)) {
      reject("duplicate_title_time");
      continue;
    }
    const externalId = normalizeWhitespace(candidate.externalId) || stableEventId(title, publishedAt);
    if (seen.has(externalId)) {
      reject("duplicate_external_id");
      continue;
    }
    seen.add(externalId);
    seenTitleTimes.add(titleTimeKey);

    events.push({
      external_id: externalId,
      title,
      description: normalizeWhitespace(candidate.description) || null,
      url: normalizeUrl(candidate.url, config.homeUrl),
      published_at: publishedAt,
      news_level: normalizeWhitespace(candidate.newsLevel) || null,
    });
  }

  const cappedEvents = events.slice(0, config.maxBatchSize);
  if (events.length > cappedEvents.length) {
    rejected.batch_cap = events.length - cappedEvents.length;
  }

  return {
    diagnostics: {
      candidates: candidates.length,
      normalized: cappedEvents.length,
      rejected,
    },
    events: cappedEvents,
  };
}

export function extractNewsCandidatesFromText(
  text,
  { nowIso = new Date().toISOString(), timezoneOffsetMinutes = new Date().getTimezoneOffset() } = {},
) {
  const lines = String(text ?? "")
    .split(/\r?\n/u)
    .map(normalizeWhitespace)
    .filter(Boolean);
  const candidates = [];

  for (let index = 0; index < lines.length; index += 1) {
    const title = lines[index];
    if (parseFinancialJuiceMetaLine(title, nowIso, timezoneOffsetMinutes)) {
      continue;
    }

    const meta = findNearbyMetaLine(lines, index + 1, nowIso, timezoneOffsetMinutes);
    if (!meta) {
      continue;
    }

    candidates.push({
      externalId: `text:${meta.publishedAt ?? meta.raw}:${title}`,
      title,
      description: "",
      url: "",
      publishedAt: meta.publishedAt,
      newsLevel: meta.newsLevel,
      extraction: "text",
    });
  }

  return candidates;
}

export function normalizeWhitespace(value) {
  return String(value ?? "").split(/\s+/u).filter(Boolean).join(" ");
}

function readNewsSnapshotFromDom({ mainFeedRootSelector, mainFeedSelector }) {
  function collapseWhitespaceInBrowser(value) {
    return String(value ?? "").split(/\s+/u).filter(Boolean).join(" ");
  }

  function monthIndexFromShortNameInBrowser(value) {
    if (!value) {
      return null;
    }
    const monthIndex = [
      "jan",
      "feb",
      "mar",
      "apr",
      "may",
      "jun",
      "jul",
      "aug",
      "sep",
      "oct",
      "nov",
      "dec",
    ].indexOf(value.toLowerCase());
    return monthIndex >= 0 ? monthIndex : null;
  }

  function parseFinancialJuiceTimeTextInBrowser(value) {
    const normalized = collapseWhitespaceInBrowser(value);
    const match = /^(\d{1,2}):(\d{2})(?:\s*(Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)\s*(\d{1,2}))?/iu.exec(
      normalized,
    );
    if (!match) {
      return "";
    }

    const now = new Date();
    const hour = Number.parseInt(match[1], 10);
    const minute = Number.parseInt(match[2], 10);
    const month = monthIndexFromShortNameInBrowser(match[3]) ?? now.getMonth();
    const day = match[4] ? Number.parseInt(match[4], 10) : now.getDate();
    if (hour > 23 || minute > 59 || day < 1 || day > 31) {
      return "";
    }

    const localDate = new Date(now.getFullYear(), month, day, hour, minute, 0, 0);
    return localDate.toISOString();
  }

  function isUsableFinancialJuiceUrl(value) {
    const normalized = String(value ?? "").trim();
    return normalized.length > 0 && !normalized.startsWith("javascript:");
  }

  function readMainFeedHeadlineCandidate(element) {
    const headlineId = element.getAttribute("data-headlineid") ?? "";
    const titleNode =
      element.querySelector(".headline-title .headline-title-nolink") ??
      element.querySelector(".headline-title a") ??
      element.querySelector(".headline-title");
    const descriptionNode = element.querySelector(".headline-content");
    const timeText = element.querySelector(".time")?.textContent ?? "";
    const labels = Array.from(element.querySelectorAll(".news-label"))
      .map((label) => label.textContent ?? "")
      .map(collapseWhitespaceInBrowser)
      .filter(Boolean);
    const feedClasses = Array.from(element.querySelectorAll(".feedWrap"))
      .flatMap((node) => Array.from(node.classList))
      .filter((className) => !["feedWrap", "media", "undefined"].includes(className));
    const socialUrl = element.querySelector(".social-nav")?.getAttribute("data-link") ?? "";
    const titleUrl = element.querySelector(".headline-title a[href]")?.href ?? "";

    return {
      externalId: headlineId,
      title: titleNode?.textContent ?? "",
      description: descriptionNode?.textContent ?? "",
      url: isUsableFinancialJuiceUrl(socialUrl) ? socialUrl : titleUrl,
      publishedAt: parseFinancialJuiceTimeTextInBrowser(timeText),
      newsLevel: [...feedClasses, ...labels].join(" "),
      extraction: "mainFeed",
    };
  }

  const candidates = Array.from(document.querySelectorAll(mainFeedSelector))
    .map(readMainFeedHeadlineCandidate)
    .filter((candidate) => candidate.externalId !== "0");

  return {
    candidates,
    mainFeedText: document.querySelector(mainFeedRootSelector)?.innerText ?? "",
    timezoneOffsetMinutes: new Date().getTimezoneOffset(),
  };
}

async function readVisibleNewsHistoryState(page, config) {
  const extraction = await extractNewsFromPageWithDiagnostics(page, {
    ...config,
    maxBatchSize: Number.MAX_SAFE_INTEGER,
  });
  const domState = await page.evaluate(({ mainFeedSelector }) => {
    const items = Array.from(document.querySelectorAll(mainFeedSelector));
    return {
      domItems: items.length,
      firstHeadlineId: items.at(0)?.getAttribute("data-headlineid") || null,
      lastHeadlineId: items.at(-1)?.getAttribute("data-headlineid") || null,
      loadPreviousNewsAvailable: typeof globalThis.LoadPreviousNews === "function",
      objHTTPBusy: Boolean(globalThis.objHTTP),
    };
  }, {
    mainFeedSelector: MAIN_FEED_SELECTOR,
  });
  const oldestEvent = extraction.events.at(-1) ?? null;
  return {
    ...domState,
    normalized: extraction.events.length,
    oldestEventId: oldestEvent?.external_id ?? null,
    oldestPublishedAt: oldestEvent?.published_at ?? null,
    oldestDayKey: localDayKeyFromTimestamp(oldestEvent?.published_at, config.historyTimezone),
  };
}

async function requestPreviousNewsPage(page, oldId) {
  return page.evaluate((oldIdValue) => {
    if (typeof globalThis.LoadPreviousNews !== "function") {
      return { ok: false, reason: "missing_LoadPreviousNews" };
    }
    const numericOldId = Number.parseInt(String(oldIdValue), 10);
    if (!Number.isSafeInteger(numericOldId) || numericOldId < 0) {
      return { ok: false, reason: "invalid_old_id" };
    }
    try {
      globalThis.LoadPreviousNews(false, numericOldId);
      return { ok: true };
    } catch (error) {
      return { ok: false, reason: error instanceof Error ? error.message : String(error) };
    }
  }, oldId);
}

async function waitForPreviousNewsPage(page, previousDomItems, previousLastHeadlineId, config) {
  const timeout = Math.min(
    Math.max(config.requestTimeoutMs ?? HISTORY_LOAD_TIMEOUT_MS, 1_000),
    HISTORY_LOAD_TIMEOUT_MS,
  );
  try {
    await page.waitForFunction(
      ({ mainFeedSelector, previousDomItems, previousLastHeadlineId }) => {
        const items = Array.from(document.querySelectorAll(mainFeedSelector));
        const lastHeadlineId = items.at(-1)?.getAttribute("data-headlineid") || null;
        const requestFinished = !globalThis.objHTTP;
        const changed = items.length > previousDomItems || lastHeadlineId !== previousLastHeadlineId;
        return requestFinished && changed;
      },
      { mainFeedSelector: MAIN_FEED_SELECTOR, previousDomItems, previousLastHeadlineId },
      { polling: 250, timeout },
    );
    return { changed: true };
  } catch {
    const state = await page.evaluate(({ mainFeedSelector }) => {
      const items = Array.from(document.querySelectorAll(mainFeedSelector));
      return {
        domItems: items.length,
        lastHeadlineId: items.at(-1)?.getAttribute("data-headlineid") || null,
        objHTTPBusy: Boolean(globalThis.objHTTP),
      };
    }, {
      mainFeedSelector: MAIN_FEED_SELECTOR,
    });
    if (state.domItems > previousDomItems || state.lastHeadlineId !== previousLastHeadlineId) {
      return { changed: true };
    }
    return {
      changed: false,
      reason: state.objHTTPBusy ? "load_timeout" : "no_more_history",
    };
  }
}

function localDayKeyDaysAgo(daysAgo, timezone) {
  const date = new Date();
  date.setUTCDate(date.getUTCDate() - daysAgo);
  return localDayKeyFromTimestamp(date.toISOString(), timezone);
}

function localDayKeyFromTimestamp(value, timezone = "Europe/Paris") {
  if (!value) {
    return null;
  }
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return null;
  }
  const parts = new Intl.DateTimeFormat("en", {
    timeZone: timezone,
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  }).formatToParts(date);
  const part = (type) => parts.find((entry) => entry.type === type)?.value;
  return `${part("year")}-${part("month")}-${part("day")}`;
}

function isUsableTitle(title, candidate) {
  return !unusableTitleReason(title, candidate);
}

function unusableTitleReason(title, candidate) {
  if (title.length < 8 || title.length > 500) {
    return title.length < 8 ? "short_title" : "long_title";
  }
  const lowerTitle = title.toLowerCase();
  if (CHALLENGE_MARKERS.some((marker) => lowerTitle.includes(marker))) {
    return "challenge";
  }
  if (isBlockedNewsLevel(candidate.newsLevel)) {
    return "blocked_level";
  }
  if (isCalendarDateOnlyTitle(title)) {
    return "calendar_date";
  }
  if (isUiChromeTitle(title)) {
    return "ui_chrome";
  }
  return null;
}

function findNearbyMetaLine(lines, startIndex, nowIso, timezoneOffsetMinutes) {
  const limit = Math.min(lines.length, startIndex + 3);
  for (let index = startIndex; index < limit; index += 1) {
    const meta = parseFinancialJuiceMetaLine(lines[index], nowIso, timezoneOffsetMinutes);
    if (meta) {
      return meta;
    }
  }
  return null;
}

function parseFinancialJuiceMetaLine(value, nowIso, timezoneOffsetMinutes) {
  const normalized = normalizeWhitespace(value);
  const match = /^(\d{1,2}):(\d{2})(.*)$/u.exec(normalized);
  if (!match) {
    return null;
  }

  const hour = Number.parseInt(match[1], 10);
  const minute = Number.parseInt(match[2], 10);
  if (hour > 23 || minute > 59) {
    return null;
  }

  const fallbackDate = new Date(nowIso);
  const metadata = parseFinancialJuiceDateSuffix(match[3]);
  const monthIndex = monthIndexFromShortName(metadata.month);
  const month = monthIndex ?? fallbackDate.getUTCMonth();
  const day = metadata.day ? Number.parseInt(metadata.day, 10) : fallbackDate.getUTCDate();
  if (day < 1 || day > 31) {
    return null;
  }

  const utcMs =
    Date.UTC(fallbackDate.getUTCFullYear(), month, day, hour, minute) +
    timezoneOffsetMinutes * 60_000;

  return {
    raw: normalized,
    publishedAt: new Date(utcMs).toISOString(),
    newsLevel: normalizeWhitespace(metadata.newsLevel),
  };
}

function parseFinancialJuiceDateSuffix(value) {
  const normalized = String(value ?? "");
  const match = /^\s*(Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)\s*(\d{1,2})(.*)$/iu.exec(
    normalized,
  );
  if (!match) {
    return { month: null, day: null, newsLevel: normalized };
  }

  return { month: match[1], day: match[2], newsLevel: match[3] };
}

function monthIndexFromShortName(value) {
  if (!value) {
    return null;
  }
  const monthIndex = [
    "jan",
    "feb",
    "mar",
    "apr",
    "may",
    "jun",
    "jul",
    "aug",
    "sep",
    "oct",
    "nov",
    "dec",
  ].indexOf(value.toLowerCase());
  return monthIndex >= 0 ? monthIndex : null;
}

function isBlockedNewsLevel(value) {
  const normalized = normalizeWhitespace(value).toLowerCase();
  return BLOCKED_NEWS_LEVEL_MARKERS.some((marker) => normalized.includes(marker));
}

function isUiChromeTitle(value) {
  const normalized = normalizeWhitespace(value)
    .toLowerCase()
    .replace(/\u00a9/gu, "copyright")
    .replace(/\u00d7/gu, "x")
    .replace(/\u2019/gu, "'");

  if (UI_CHROME_TITLES.has(normalized)) {
    return true;
  }

  return UI_CHROME_TITLE_PATTERNS.some((pattern) => pattern.test(normalized));
}

function isCalendarDateOnlyTitle(value) {
  const normalized = normalizeWhitespace(value).toLowerCase();
  const match = /^(january|february|march|april|may|june|july|august|september|october|november|december)\s+(\d{1,2})$/u.exec(
    normalized,
  );
  if (!match) {
    return false;
  }

  const day = Number.parseInt(match[2], 10);
  return day >= 1 && day <= 31;
}

function isFuturePublishedAt(value, nowIso = new Date().toISOString()) {
  if (!value) {
    return false;
  }

  const publishedAtMs = Date.parse(value);
  const nowMs = Date.parse(nowIso);
  if (!Number.isFinite(publishedAtMs) || !Number.isFinite(nowMs)) {
    return false;
  }

  return publishedAtMs > nowMs + FUTURE_PUBLISHED_AT_SKEW_MS;
}

function normalizePublishedAt(value) {
  const normalized = normalizeWhitespace(value);
  if (!normalized) {
    return null;
  }
  const date = new Date(normalized);
  if (Number.isNaN(date.getTime())) {
    return null;
  }
  return date.toISOString();
}

function normalizeUrl(value, fallback) {
  const normalized = normalizeWhitespace(value);
  if (!normalized) {
    return fallback;
  }
  try {
    return new URL(normalized, fallback).toString();
  } catch {
    return fallback;
  }
}

function stableEventId(title, publishedAt) {
  return createHash("sha256")
    .update(`${publishedAt ?? "undated"}|${title}`)
    .digest("hex")
    .slice(0, 24);
}
