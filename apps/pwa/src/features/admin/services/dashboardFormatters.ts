import type {
  CalendarDaySummary,
  CalendarDaySummaryToday,
  ConnectionState,
} from "@shared/api/contracts";
import type { Tone } from "@shared/ui/workspaceTypes";

type CalendarSummaryAudioStatus = Pick<
  CalendarDaySummary | CalendarDaySummaryToday,
  "currentPlaylistUrl" | "immutablePlaylistUrl" | "playlistUrl" | "status"
>;

export function booleanLabel(value: boolean | null | undefined) {
  if (value === undefined || value === null) {
    return "n/a";
  }

  return value ? "oui" : "non";
}

export function activeLabel(value: boolean | null | undefined) {
  if (value === undefined || value === null) {
    return "n/a";
  }

  return value ? "actif" : "désactivé";
}

export function availableLabel(value: boolean | null | undefined) {
  if (value === undefined || value === null) {
    return "n/a";
  }

  return value ? "disponible" : "indisponible";
}

export function runtimeTone(
  enabled: boolean | null | undefined,
  error?: string | null,
): Tone {
  if (error) {
    return "critical";
  }
  if (enabled) {
    return "info";
  }
  return "muted";
}

export function booleanTone(value: boolean | null | undefined): Tone {
  if (value === undefined || value === null) {
    return "muted";
  }

  return value ? "info" : "muted";
}

export function connectionTone(state: ConnectionState | string): Tone {
  if (state === "open") {
    return "info";
  }
  if (state === "error") {
    return "critical";
  }
  if (state === "connecting" || state === "waiting") {
    return "trace";
  }
  return "muted";
}

export function summaryStatusTone(status: string | null | undefined): Tone {
  if (status === "audio_ready" || status === "summarized") {
    return "info";
  }
  if (status === "error") {
    return "critical";
  }
  if (status === "pending" || status === "audio_generating") {
    return "trace";
  }
  return "muted";
}

export function hasCalendarSummaryAudioPlaylist(
  summary: CalendarSummaryAudioStatus | null | undefined,
) {
  return Boolean(
    summary?.playlistUrl ||
    summary?.currentPlaylistUrl ||
    summary?.immutablePlaylistUrl,
  );
}

export function calendarSummaryDisplayStatus(
  summary: CalendarSummaryAudioStatus | null | undefined,
) {
  const status = summary?.status ?? "missing";
  if (status === "audio_ready" && !hasCalendarSummaryAudioPlaylist(summary)) {
    return "audio_generating";
  }
  return status;
}

export function calendarSummaryStatusTone(
  summary: CalendarSummaryAudioStatus | null | undefined,
): Tone {
  return summaryStatusTone(calendarSummaryDisplayStatus(summary));
}

export function runStepTone(status: string | null | undefined): Tone {
  const normalized = status?.toLowerCase();
  if (
    normalized === "ok" ||
    normalized === "done" ||
    normalized === "success" ||
    normalized === "ready" ||
    normalized === "audio_ready"
  ) {
    return "info";
  }
  if (normalized === "error" || normalized === "failed") {
    return "critical";
  }
  if (
    normalized === "warning" ||
    normalized === "running" ||
    normalized === "pending" ||
    normalized === "info"
  ) {
    return "trace";
  }
  return "muted";
}

export function sourceStatusLabel(status: string | null | undefined) {
  switch (status) {
    case "ok":
      return "OK";
    case "blocked":
      return "bloqué";
    case "degraded":
      return "dégradé";
    case "stale":
      return "signal de présence expiré";
    case "stopped":
      return "arrêté";
    default:
      return status ?? "n/a";
  }
}

export function sourceStatusTone(status: string | null | undefined): Tone {
  if (status === "ok") {
    return "info";
  }
  if (status === "blocked" || status === "stopped") {
    return "critical";
  }
  if (status === "degraded" || status === "stale") {
    return "trace";
  }
  return "muted";
}

export function financialJuiceDisabledReasonLabel(reason: string) {
  if (reason === "cloudflare_managed_challenge") {
    return "Cloudflare managed challenge";
  }

  return reason;
}

export function xDisabledReasonLabel(reason: string) {
  if (reason === "payment_required" || reason === "x_payment_required") {
    return "crédits ou accès à l’API X requis";
  }

  return reason;
}

export function formatBytes(value: number | null | undefined) {
  if (value === undefined || value === null) {
    return "n/a";
  }
  if (value < 1024) {
    return `${value} o`;
  }

  return `${Math.round(value / 1024)} Ko`;
}

export function formatArtifactContent(value: string) {
  try {
    return JSON.stringify(JSON.parse(value), null, 2);
  } catch {
    return value;
  }
}

export function terminalColorForLevel(
  level: string | null,
  colors: {
    dangerText: string;
    warningText: string;
    textPrimary: string;
    successText: string;
  },
) {
  if (level === "error") {
    return colors.dangerText;
  }
  if (level === "warn") {
    return colors.warningText;
  }
  if (level === "debug" || level === "trace") {
    return colors.textPrimary;
  }
  if (level === "info") {
    return colors.textPrimary;
  }

  return colors.successText;
}

export function formatTerminalLine(item: {
  cursor: number;
  level: string | null;
  text: string;
}) {
  const level = item.level ? item.level.toUpperCase().padEnd(5, " ") : "LOG  ";
  return `${String(item.cursor).padStart(6, "0")} ${level} ${item.text}`;
}

export function normalizeXAccountInput(value: string) {
  return value.trim().replace(/^@+/, "");
}

export function normalizeXKeywordInput(value: string) {
  return value.trim().replace(/^"+|"+$/g, "");
}

export function formatXAccount(account: string) {
  return account.startsWith("@") ? account : `@${account}`;
}
