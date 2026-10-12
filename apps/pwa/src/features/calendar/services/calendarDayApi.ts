import type { AuthSessionClient } from "@shared/api/authSession";
import type { CalendarDayToday } from "@shared/api/contracts";
import { cachedJsonGet } from "@shared/api/requestCache";
import { buildCalendarDayTodayUrl } from "@shared/api/urls";

type CalendarDayTodayRequestOptions = {
  authSession: AuthSessionClient | null;
  localDay?: string | null;
  signal?: AbortSignal;
  wsUrl: string;
};

export async function loadCalendarDayToday({
  authSession,
  localDay,
  signal,
  wsUrl,
}: CalendarDayTodayRequestOptions) {
  return cachedJsonGet<CalendarDayToday>(buildCalendarDayTodayUrl(wsUrl, { localDay }), {
    authSession,
    context: "chargement du calendrier du jour",
    signal,
    ttlMs: 10_000,
  });
}
