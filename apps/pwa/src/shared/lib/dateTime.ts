type DateTimeInput = Date | number | string | null | undefined;

const locale = "fr-FR";

export function formatLocalDate(
  value: DateTimeInput,
  fallback = "date inconnue",
) {
  const parsed = parseDateTime(value);
  if (!parsed) {
    return fallbackOrRaw(value, fallback);
  }

  return new Intl.DateTimeFormat(locale, {
    day: "2-digit",
    month: "2-digit",
    year: "numeric",
  }).format(parsed);
}

export function formatLocalTime(
  value: DateTimeInput,
  fallback = "heure inconnue",
) {
  const parsed = parseDateTime(value);
  if (!parsed) {
    return fallbackOrRaw(value, fallback);
  }

  return new Intl.DateTimeFormat(locale, {
    hour: "2-digit",
    minute: "2-digit",
  }).format(parsed);
}

export function formatLocalDateTime(
  value: DateTimeInput,
  fallback = "date inconnue",
) {
  const parsed = parseDateTime(value);
  if (!parsed) {
    return fallbackOrRaw(value, fallback);
  }

  return formatDateTime(parsed, false);
}

export function formatLocalDayKey(
  dayKey: string | null | undefined,
  fallback = "date inconnue",
) {
  const parsed = parseLocalDayKey(dayKey);
  if (!parsed) {
    return fallbackOrRaw(dayKey, fallback);
  }

  return new Intl.DateTimeFormat(locale, {
    day: "2-digit",
    month: "long",
    weekday: "long",
    year: "numeric",
  }).format(parsed);
}

export function currentParisLocalDayKey() {
  return localDayKeyFromDate(new Date());
}

export function businessWeekDaysForLocalDay(dayKey: string) {
  const parsed = parseLocalDayKey(dayKey) ?? new Date();
  const monday = mondayForDate(parsed);
  return Array.from({ length: 5 }, (_, index) => {
    const day = new Date(monday);
    day.setDate(monday.getDate() + index);
    return localDayKeyFromLocalDate(day);
  });
}

export function defaultBusinessCalendarDayKey() {
  const todayKey = currentParisLocalDayKey();
  const today = parseLocalDayKey(todayKey);
  if (!today) {
    return todayKey;
  }
  const day = today.getDay();
  if (day >= 1 && day <= 5) {
    return todayKey;
  }
  const monday = mondayForDate(today);
  monday.setDate(monday.getDate() + 7);
  return localDayKeyFromLocalDate(monday);
}

export function shortWeekdayLabel(dayKey: string) {
  const parsed = parseLocalDayKey(dayKey);
  if (!parsed) {
    return dayKey;
  }
  return new Intl.DateTimeFormat(locale, {
    day: "2-digit",
    month: "2-digit",
    weekday: "short",
  }).format(parsed);
}

export function formatLocalDateTimeWithRaw(
  value: DateTimeInput,
  fallback = "n/a",
) {
  const raw = rawValue(value);
  if (!raw) {
    return fallback;
  }

  const parsed = parseDateTime(value);
  if (!parsed) {
    return raw;
  }

  return `${formatDateTime(parsed, true)} \u2022 raw=${raw}`;
}

function formatDateTime(date: Date, includeSeconds: boolean) {
  const options: Intl.DateTimeFormatOptions = {
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    month: "2-digit",
    year: "numeric",
  };

  if (includeSeconds) {
    options.second = "2-digit";
  }

  return new Intl.DateTimeFormat(locale, options).format(date);
}

function parseDateTime(value: DateTimeInput) {
  if (value === null || value === undefined || value === "") {
    return null;
  }

  if (value instanceof Date) {
    return Number.isNaN(value.getTime()) ? null : value;
  }

  const parsed = new Date(value);
  return Number.isNaN(parsed.getTime()) ? null : parsed;
}

function parseLocalDayKey(dayKey: string | null | undefined) {
  const match = /^(\d{4})-(\d{2})-(\d{2})$/u.exec(dayKey ?? "");
  if (!match) {
    return null;
  }

  const year = Number.parseInt(match[1], 10);
  const month = Number.parseInt(match[2], 10);
  const day = Number.parseInt(match[3], 10);
  const parsed = new Date(year, month - 1, day, 12, 0, 0, 0);

  if (
    parsed.getFullYear() !== year ||
    parsed.getMonth() !== month - 1 ||
    parsed.getDate() !== day
  ) {
    return null;
  }

  return parsed;
}

function localDayKeyFromDate(date: Date) {
  const parts = new Intl.DateTimeFormat("en-CA", {
    day: "2-digit",
    month: "2-digit",
    timeZone: "Europe/Paris",
    year: "numeric",
  }).formatToParts(date);
  const valueByType = Object.fromEntries(
    parts.map((part) => [part.type, part.value]),
  );
  return `${valueByType.year}-${valueByType.month}-${valueByType.day}`;
}

function localDayKeyFromLocalDate(date: Date) {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

function mondayForDate(date: Date) {
  const monday = new Date(date);
  const day = monday.getDay();
  const daysFromMonday = day === 0 ? 6 : day - 1;
  monday.setDate(monday.getDate() - daysFromMonday);
  monday.setHours(12, 0, 0, 0);
  return monday;
}

function fallbackOrRaw(value: DateTimeInput, fallback: string) {
  const raw = rawValue(value);
  return raw || fallback;
}

function rawValue(value: DateTimeInput) {
  if (value === null || value === undefined || value === "") {
    return null;
  }

  if (value instanceof Date) {
    return Number.isNaN(value.getTime()) ? null : value.toISOString();
  }

  return String(value);
}
