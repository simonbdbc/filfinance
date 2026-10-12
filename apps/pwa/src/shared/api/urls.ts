import { readPublicEnv } from "@shared/config/runtimeEnv";

export function defaultWsUrl() {
  const configuredUrl = readPublicEnv("EXPO_PUBLIC_FILFINANCE_WS_URL");
  if (configuredUrl) {
    return configuredUrl;
  }

  const browserHost =
    typeof window !== "undefined" && window.location.hostname
      ? window.location.hostname
      : "localhost";

  return `ws://${browserHost}:8088/ws/news`;
}

export function defaultEdgeLiveWsUrl() {
  const configuredUrl = readPublicEnv("EXPO_PUBLIC_FILFINANCE_EDGE_WS_URL");
  if (configuredUrl) {
    return configuredUrl;
  }

  return "";
}

export function defaultApiBaseUrl() {
  const configuredUrl = readPublicEnv("EXPO_PUBLIC_API_URL");
  if (configuredUrl) {
    return configuredUrl.replace(/\/$/, "");
  }

  return "http://localhost:3000";
}

export function deriveTraceWsUrl(wsUrl: string) {
  try {
    const parsed = new URL(wsUrl);
    parsed.pathname = "/ws/test-runs";
    parsed.search = "";
    parsed.hash = "";
    return parsed.toString();
  } catch {
    return wsUrl.replace(/\/ws\/news\/?$/, "/ws/test-runs");
  }
}

export function deriveApiBaseUrl(wsUrl: string) {
  try {
    const parsed = new URL(wsUrl);
    parsed.protocol = parsed.protocol === "wss:" ? "https:" : "http:";
    parsed.pathname = "";
    parsed.search = "";
    parsed.hash = "";
    return parsed.toString().replace(/\/$/, "");
  } catch {
    return "http://localhost:8088";
  }
}

export function defaultHlsBaseUrl() {
  const configuredUrl = readPublicEnv("EXPO_PUBLIC_FILFINANCE_HLS_BASE_URL");
  if (configuredUrl) {
    return configuredUrl.replace(/\/$/, "");
  }

  return "";
}

export function deriveHlsBaseUrl(wsUrl: string) {
  return defaultHlsBaseUrl() || deriveApiBaseUrl(wsUrl);
}

export function buildReplayTodayPlaylistUrl(wsUrl: string) {
  return resolveHlsResourceUrl(wsUrl, "/audio/replay/today/playlist.m3u8");
}

export function buildReplayTodayMetadataUrl(wsUrl: string) {
  const baseUrl = deriveApiBaseUrl(wsUrl);
  return new URL("/api/replay/today", `${baseUrl}/`).toString();
}

type LocalDayUrlOptions = {
  localDay?: string | null;
};

export function buildDailySummaryTodayUrl(wsUrl: string, options: LocalDayUrlOptions = {}) {
  const baseUrl = deriveApiBaseUrl(wsUrl);
  const url = new URL(
    options.localDay ? "/api/calendar-day/summary" : "/api/calendar-day/today/summary",
    `${baseUrl}/`,
  );
  if (options.localDay) {
    url.searchParams.set("local_day", options.localDay);
  }
  return url.toString();
}

export function buildCalendarDayTodayUrl(wsUrl: string, options: LocalDayUrlOptions = {}) {
  const baseUrl = deriveApiBaseUrl(wsUrl);
  const url = new URL(
    options.localDay ? "/api/calendar-day" : "/api/calendar-day/today",
    `${baseUrl}/`,
  );
  if (options.localDay) {
    url.searchParams.set("local_day", options.localDay);
  }
  return url.toString();
}

export function buildAdminDailySummaryTodayUrl(wsUrl: string, options: LocalDayUrlOptions = {}) {
  const baseUrl = deriveApiBaseUrl(wsUrl);
  const url = new URL(
    options.localDay ? "/api/admin/calendar-day" : "/api/admin/calendar-day/today",
    `${baseUrl}/`,
  );
  if (options.localDay) {
    url.searchParams.set("local_day", options.localDay);
  }
  return url.toString();
}

export function buildAdminCalendarDayRunUrl(wsUrl: string) {
  const baseUrl = deriveApiBaseUrl(wsUrl);
  return new URL("/api/admin/calendar-day/run", `${baseUrl}/`).toString();
}

export function buildAdminCalendarWeekRunUrl(wsUrl: string) {
  const baseUrl = deriveApiBaseUrl(wsUrl);
  return new URL("/api/admin/calendar-week/run", `${baseUrl}/`).toString();
}

export function buildAdminDailySummaryWsUrl(
  wsUrl: string,
  options: { localDay?: string | null } = {},
) {
  const endpointUrl = new URL(wsUrl);
  endpointUrl.pathname = "/ws/admin/calendar-day";
  endpointUrl.search = "";
  endpointUrl.hash = "";

  if (options.localDay) {
    endpointUrl.searchParams.set("local_day", options.localDay);
  }

  return endpointUrl.toString();
}

export function buildHlsAccessTicketUrl(wsUrl: string) {
  const baseUrl = deriveApiBaseUrl(wsUrl);
  return new URL("/api/hls-ticket", `${baseUrl}/`).toString();
}

export function buildSourceStatusesUrl(wsUrl: string) {
  const baseUrl = deriveApiBaseUrl(wsUrl);
  return new URL("/api/sources", `${baseUrl}/`).toString();
}

export function resolveApiResourceUrl(wsUrl: string, resourceUrl: string) {
  const baseUrl = deriveApiBaseUrl(wsUrl);
  return new URL(resourceUrl, `${baseUrl}/`).toString();
}

export function resolveHlsResourceUrl(wsUrl: string, resourceUrl: string) {
  const configuredHlsBaseUrl = defaultHlsBaseUrl();
  if (!configuredHlsBaseUrl) {
    return resolveApiResourceUrl(wsUrl, resourceUrl);
  }

  const resource = new URL(resourceUrl, `${deriveApiBaseUrl(wsUrl)}/`);

  return new URL(
    `${resource.pathname}${resource.search}${resource.hash}`,
    `${configuredHlsBaseUrl}/`,
  ).toString();
}

export function buildEdgeLiveTicketUrl(wsUrl: string) {
  const baseUrl = deriveApiBaseUrl(wsUrl);
  return new URL("/api/edge/live-ticket", `${baseUrl}/`).toString();
}

export function buildFinancialJuiceRuntimeUrl(wsUrl: string) {
  const baseUrl = deriveApiBaseUrl(wsUrl);
  return new URL("/api/runtime/financial-juice", `${baseUrl}/`).toString();
}

export function buildFinancialJuiceBrowserRuntimeUrl(wsUrl: string) {
  const baseUrl = deriveApiBaseUrl(wsUrl);
  return new URL("/api/runtime/financial-juice-browser", `${baseUrl}/`).toString();
}

export function buildXRuntimeUrl(wsUrl: string) {
  const baseUrl = deriveApiBaseUrl(wsUrl);
  return new URL("/api/runtime/x", `${baseUrl}/`).toString();
}

export function buildGlossaryUrl(wsUrl: string) {
  const baseUrl = deriveApiBaseUrl(wsUrl);
  return new URL("/api/glossary", `${baseUrl}/`).toString();
}

export function buildGlossaryEntryUrl(wsUrl: string, id: number) {
  const baseUrl = deriveApiBaseUrl(wsUrl);
  return new URL(`/api/glossary/${id}`, `${baseUrl}/`).toString();
}

export function buildGlossaryReapplyTranslationsUrl(wsUrl: string) {
  const baseUrl = deriveApiBaseUrl(wsUrl);
  return new URL("/api/glossary/reapply-translations", `${baseUrl}/`).toString();
}

export function buildGlossarySystemRulesUrl(wsUrl: string) {
  const baseUrl = deriveApiBaseUrl(wsUrl);
  return new URL("/api/glossary/system-rules", `${baseUrl}/`).toString();
}

export function buildGlossaryPreviewUrl(wsUrl: string) {
  const baseUrl = deriveApiBaseUrl(wsUrl);
  return new URL("/api/glossary/preview", `${baseUrl}/`).toString();
}

export function buildNewsBlacklistUrl(wsUrl: string) {
  const baseUrl = deriveApiBaseUrl(wsUrl);
  return new URL("/api/news-blacklist", `${baseUrl}/`).toString();
}

export function buildNewsBlacklistEntryUrl(wsUrl: string, id: number) {
  const baseUrl = deriveApiBaseUrl(wsUrl);
  return new URL(`/api/news-blacklist/${id}`, `${baseUrl}/`).toString();
}

export function buildNewsBlacklistHitsUrl(wsUrl: string) {
  const baseUrl = deriveApiBaseUrl(wsUrl);
  return new URL("/api/news-blacklist/hits", `${baseUrl}/`).toString();
}

export function buildNewsBlacklistReapplyUrl(wsUrl: string) {
  const baseUrl = deriveApiBaseUrl(wsUrl);
  return new URL("/api/news-blacklist/reapply", `${baseUrl}/`).toString();
}

export function buildNewsTruncationAuditUrl(wsUrl: string) {
  const baseUrl = deriveApiBaseUrl(wsUrl);
  return new URL("/api/news-truncation-audit", `${baseUrl}/`).toString();
}

export function buildAdminLogDaysUrl(wsUrl: string) {
  const baseUrl = deriveApiBaseUrl(wsUrl);
  return new URL("/api/admin/logs/days", `${baseUrl}/`).toString();
}

export function buildAdminLogsUrl(
  wsUrl: string,
  options: {
    day: string;
    limit?: number;
    cursor?: number | null;
    level?: string | null;
    query?: string | null;
  },
) {
  const baseUrl = deriveApiBaseUrl(wsUrl);
  const endpointUrl = new URL("/api/admin/logs", `${baseUrl}/`);
  endpointUrl.searchParams.set("day", options.day);
  endpointUrl.searchParams.set("limit", String(options.limit ?? 200));

  if (options.cursor) {
    endpointUrl.searchParams.set("cursor", String(options.cursor));
  }

  if (options.level) {
    endpointUrl.searchParams.set("level", options.level);
  }

  if (options.query) {
    endpointUrl.searchParams.set("q", options.query);
  }

  return endpointUrl.toString();
}

export function buildAdminLogsTailWsUrl(
  wsUrl: string,
  options: {
    day?: string | null;
    limit?: number;
    level?: string | null;
    query?: string | null;
  },
) {
  const endpointUrl = new URL(wsUrl);
  endpointUrl.pathname = "/ws/admin/logs";
  endpointUrl.search = "";
  endpointUrl.hash = "";

  if (options.day) {
    endpointUrl.searchParams.set("day", options.day);
  }

  endpointUrl.searchParams.set("limit", String(options.limit ?? 500));

  if (options.level) {
    endpointUrl.searchParams.set("level", options.level);
  }

  if (options.query) {
    endpointUrl.searchParams.set("q", options.query);
  }

  return endpointUrl.toString();
}

export function buildTranslationsHistoryUrl(
  wsUrl: string,
  options: {
    day: string;
    historyScope: "real_feed" | "test_run";
    source?: string;
    limit?: number;
    beforeId?: number | null;
  },
) {
  const baseUrl = deriveApiBaseUrl(wsUrl);
  const endpointUrl = new URL("/api/translations", `${baseUrl}/`);
  endpointUrl.searchParams.set("day", options.day);
  endpointUrl.searchParams.set("history_scope", options.historyScope);
  endpointUrl.searchParams.set("limit", String(options.limit ?? 100));

  if (options.source) {
    endpointUrl.searchParams.set("source", options.source);
  }

  if (options.beforeId) {
    endpointUrl.searchParams.set("before_id", String(options.beforeId));
  }

  return endpointUrl.toString();
}

export function buildTranslationDetailUrl(wsUrl: string, id: number) {
  const baseUrl = deriveApiBaseUrl(wsUrl);
  return new URL(`/api/translations/${id}`, `${baseUrl}/`).toString();
}
