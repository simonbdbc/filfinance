import type { TranslatedNewsItem } from "../api/contracts";
import { formatLocalTime } from "./dateTime";

export function itemTimestampValue(item: TranslatedNewsItem) {
  return item.timestamp || item.created_at;
}

export function localDayKeyFromDate(date: Date) {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

export function localDayKeyDaysAgo(daysAgo: number) {
  const date = new Date();
  date.setHours(12, 0, 0, 0);
  date.setDate(date.getDate() - daysAgo);
  return localDayKeyFromDate(date);
}

export function localDayKeyFromTimestamp(value?: string | null) {
  if (!value) {
    return null;
  }

  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) {
    return null;
  }

  return localDayKeyFromDate(parsed);
}

export function formatLocalHourMinute(value?: string | null) {
  return formatLocalTime(value, "heure inconnue");
}

export function newsLevelContainsActive(newsLevel?: string | null) {
  return /\bactive\b/iu.test(String(newsLevel ?? ""));
}

export function sortHistoryNewestFirst(items: TranslatedNewsItem[]) {
  return [...items].sort((left, right) => {
    const timestampComparison = itemTimestampValue(right).localeCompare(itemTimestampValue(left));
    if (timestampComparison !== 0) {
      return timestampComparison;
    }

    return right.id - left.id;
  });
}

export function mergeHistoryItemsNewestFirst(
  existingItems: TranslatedNewsItem[],
  nextItems: TranslatedNewsItem[],
) {
  const mergedItems = new Map<number, TranslatedNewsItem>();

  for (const item of [...existingItems, ...nextItems]) {
    mergedItems.set(item.id, item);
  }

  return sortHistoryNewestFirst([...mergedItems.values()]);
}

const browserChromeSourceTitles = new Set([
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

const browserChromeSourceTitlePatterns = [
  /^\[\d{1,2}:\d{2}\s+et\]/u,
  /^copyright\s+\d{4}\s+financialjuice$/u,
  /^\d{4}\s+financialjuice$/u,
  /^don'?t like ads\?\s+go pro$/u,
  /^join us and go real-time$/u,
];

const futureBrowserItemSkewMs = 2 * 60 * 1000;

export function isDisplayableRealFeedItem(item: TranslatedNewsItem) {
  if (item.source !== "financial_juice" && item.source !== "financial_juice_browser") {
    return false;
  }

  if (item.source !== "financial_juice_browser") {
    return true;
  }

  const newsLevel = normalizeBrowserChromeText(item.news_level);
  if (newsLevel.includes("event-header")) {
    return false;
  }

  const sourceTitle = normalizeBrowserChromeText(item.source_title);
  if (browserChromeSourceTitles.has(sourceTitle)) {
    return false;
  }
  if (isCalendarDateOnlySourceTitle(sourceTitle)) {
    return false;
  }
  if (isFutureBrowserFeedItem(item)) {
    return false;
  }

  return !browserChromeSourceTitlePatterns.some((pattern) =>
    pattern.test(sourceTitle)
  );
}

export function dedupeRealFeedItemsBySourcePriority(items: TranslatedNewsItem[]) {
  const dedupedItems = new Map<string, TranslatedNewsItem>();

  for (const item of items.filter(isDisplayableRealFeedItem)) {
    const key = realFeedDedupKey(item);
    const existingItem = dedupedItems.get(key);
    if (!existingItem || realFeedSourcePriority(item) > realFeedSourcePriority(existingItem)) {
      dedupedItems.set(key, item);
    }
  }

  return sortHistoryNewestFirst([...dedupedItems.values()]);
}

function realFeedDedupKey(item: TranslatedNewsItem) {
  return [
    "financial_juice",
    normalizeBrowserChromeText(item.source_title),
    timestampMinuteKey(itemTimestampValue(item)),
  ].join("|");
}

function realFeedSourcePriority(item: TranslatedNewsItem) {
  return item.source === "financial_juice" ? 2 : 1;
}

function timestampMinuteKey(value?: string | null) {
  const parsed = value ? new Date(value) : null;
  if (parsed && !Number.isNaN(parsed.getTime())) {
    return parsed.toISOString().slice(0, 16);
  }

  return String(value ?? "");
}

function normalizeBrowserChromeText(value?: string | null) {
  return String(value ?? "")
    .split(/\s+/u)
    .filter(Boolean)
    .join(" ")
    .toLowerCase()
    .replace(/\u00a9/gu, "copyright")
    .replace(/\u00d7/gu, "x")
    .replace(/\u2019/gu, "'");
}

function isCalendarDateOnlySourceTitle(value: string) {
  const match = /^(january|february|march|april|may|june|july|august|september|october|november|december)\s+(\d{1,2})$/u.exec(
    value,
  );
  if (!match) {
    return false;
  }

  const day = Number.parseInt(match[2], 10);
  return day >= 1 && day <= 31;
}

function isFutureBrowserFeedItem(item: TranslatedNewsItem) {
  const timestamp = itemTimestampValue(item);
  if (!timestamp) {
    return false;
  }

  const timestampMs = Date.parse(timestamp);
  if (!Number.isFinite(timestampMs)) {
    return false;
  }

  return timestampMs > Date.now() + futureBrowserItemSkewMs;
}

export function filterHistoryItemsByLocalDayKey(
  items: TranslatedNewsItem[],
  dayKey: string,
) {
  return items.filter(
    (item) => localDayKeyFromTimestamp(itemTimestampValue(item)) === dayKey
  );
}

export function pickNewestKnownItem(items: Array<TranslatedNewsItem | null | undefined>) {
  const knownItems = items.filter((item): item is TranslatedNewsItem => Boolean(item));

  if (knownItems.length === 0) {
    return null;
  }

  return sortHistoryNewestFirst(knownItems)[0] ?? null;
}
