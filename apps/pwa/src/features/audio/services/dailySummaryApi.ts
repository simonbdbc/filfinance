import type { AuthSessionClient } from "@shared/api/authSession";
import type { CalendarDaySummaryToday } from "@shared/api/contracts";
import { cachedJsonGet } from "@shared/api/requestCache";

type DailySummaryRequestOptions = {
  dailySummaryTodayUrl: string;
  authSession: AuthSessionClient | null;
  signal?: AbortSignal;
};

export async function loadDailySummaryToday({
  dailySummaryTodayUrl,
  authSession,
  signal,
}: DailySummaryRequestOptions) {
  return cachedJsonGet<CalendarDaySummaryToday>(dailySummaryTodayUrl, {
    authSession,
    context: "résumé IA du jour",
    signal,
    ttlMs: 10_000,
  });
}
