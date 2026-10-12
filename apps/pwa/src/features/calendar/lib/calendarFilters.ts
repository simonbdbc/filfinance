import type { CalendarDayItem } from "@shared/api/contracts";
import {
  formatImportanceLabel,
  importanceSortValue,
  normalizeImportance,
} from "@shared/lib/importance";

export type CalendarFilterGroup = "importance" | "location" | "type";
export type LocationFilter = "ASIE" | "Autres" | "EU" | "US";

export type CalendarFilters = Record<CalendarFilterGroup, string[]>;

export type CalendarFilterOption = {
  count: number;
  label: string;
  value: string;
};

export const emptyCalendarFilters: CalendarFilters = {
  importance: [],
  location: [],
  type: [],
};

export const locationFilters: LocationFilter[] = ["US", "EU", "ASIE", "Autres"];

const euCountries = new Set([
  "AT",
  "BE",
  "DE",
  "EU",
  "ES",
  "FI",
  "FR",
  "GB",
  "GR",
  "IE",
  "IT",
  "NL",
  "PT",
  "UK",
  "ZE",
]);
const euCurrencies = new Set(["EUR", "GBP"]);
const asiaCountries = new Set([
  "AU",
  "CN",
  "HK",
  "ID",
  "IN",
  "JP",
  "KR",
  "NZ",
  "SG",
]);
const asiaCurrencies = new Set([
  "AUD",
  "CNY",
  "HKD",
  "IDR",
  "INR",
  "JPY",
  "KRW",
  "NZD",
  "SGD",
]);

export function calendarFilterOptions(items: CalendarDayItem[]) {
  const importanceOptions: CalendarFilterOption[] = ["high", "medium", "low"].map(
    (value) => ({
      count: items.filter((item) => normalizeImportance(item.finalImportance) === value)
        .length,
      label: formatImportanceLabel(value),
      value,
    }),
  );
  const typeOptions = Array.from(new Set(items.map((item) => itemTypeFilter(item))))
    .sort((left, right) => left.localeCompare(right, "fr"))
    .map((value) => ({
      count: items.filter((item) => itemTypeFilter(item) === value).length,
      label: value,
      value,
    }));
  const locationOptions = locationFilters.map((value) => ({
    count: items.filter((item) => itemLocationFilter(item) === value).length,
    label: value,
    value,
  }));

  return {
    importance: importanceOptions,
    location: locationOptions,
    type: typeOptions,
  };
}

export function applyCalendarFilters(
  items: CalendarDayItem[],
  filters: CalendarFilters,
) {
  return items.filter((item) => {
    const importance = normalizeImportance(item.finalImportance);
    const type = itemTypeFilter(item);
    const location = itemLocationFilter(item);

    return (
      matchesFilter(filters.importance, importance) &&
      matchesFilter(filters.type, type) &&
      matchesFilter(filters.location, location)
    );
  });
}

export function sortCalendarItems(items: CalendarDayItem[]) {
  return [...items].sort((left, right) => {
    const eventTimeDelta = eventTimeSortValue(left) - eventTimeSortValue(right);
    if (eventTimeDelta !== 0) {
      return eventTimeDelta;
    }

    const importanceDelta =
      importanceSortValue(right.finalImportance) -
      importanceSortValue(left.finalImportance);
    if (importanceDelta !== 0) {
      return importanceDelta;
    }

    return left.id - right.id;
  });
}

export function nextCalendarSlotItems(items: CalendarDayItem[], minuteNow: number) {
  const upcomingItems = sortCalendarItems(items).filter((item) => {
    const minuteValue = eventMinuteValue(item);
    return minuteValue !== null && minuteValue >= minuteNow;
  });
  const nextMinute = upcomingItems[0] ? eventMinuteValue(upcomingItems[0]) : null;
  return nextMinute === null
    ? []
    : upcomingItems.filter((item) => eventMinuteValue(item) === nextMinute);
}

export function itemLocationFilter(item: CalendarDayItem): LocationFilter {
  const country = normalizeMarketCode(item.country);
  const currency = normalizeMarketCode(item.currency);
  if (country === "US" || currency === "USD") {
    return "US";
  }
  if ((country && euCountries.has(country)) || (currency && euCurrencies.has(currency))) {
    return "EU";
  }
  if ((country && asiaCountries.has(country)) || (currency && asiaCurrencies.has(currency))) {
    return "ASIE";
  }
  return "Autres";
}

export function eventTimeSortValue(item: CalendarDayItem) {
  const minuteValue = eventMinuteValue(item);
  return minuteValue ?? Number.MAX_SAFE_INTEGER;
}

export function eventMinuteValue(item: CalendarDayItem) {
  if (!item.eventTime) {
    return null;
  }

  const value = new Date(item.eventTime).getTime();
  return Number.isFinite(value) ? floorToMinute(value) : null;
}

export function floorToMinute(value: number) {
  return Math.floor(value / 60_000) * 60_000;
}

function matchesFilter(selectedValues: string[], value: string | null) {
  return selectedValues.length === 0 || Boolean(value && selectedValues.includes(value));
}

function normalizeMarketCode(value: string | null) {
  return value?.trim().toUpperCase() || null;
}

function itemTypeFilter(item: CalendarDayItem) {
  return normalizeFilterValue(item.category) ?? "type n/a";
}

function normalizeFilterValue(value: string | null | undefined) {
  return value?.trim().toLowerCase() || null;
}
