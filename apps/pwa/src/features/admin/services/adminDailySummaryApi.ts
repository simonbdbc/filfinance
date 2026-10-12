import { authenticatedFetch } from "@shared/api/authSession";
import type { AuthSessionClient } from "@shared/api/authSession";
import type {
  AdminCalendarDayToday,
  AdminCalendarWeekRun,
} from "@shared/api/contracts";
import {
  buildJsonBearerHeaders,
  buildBearerHeaders,
} from "@shared/api/authTransport";
import { throwHttpError } from "@shared/api/httpErrors";
import {
  buildAdminCalendarDayRunUrl,
  buildAdminCalendarWeekRunUrl,
  buildAdminDailySummaryTodayUrl,
} from "@shared/api/urls";

type AdminDailySummaryApiOptions = {
  authSession: AuthSessionClient | null;
  localDay?: string | null;
  wsUrl: string;
};

export async function loadAdminDailySummaryToday({
  authSession,
  localDay,
  wsUrl,
}: AdminDailySummaryApiOptions) {
  const response = await authenticatedFetch(authSession, buildAdminDailySummaryTodayUrl(wsUrl, { localDay }), {
    headers: buildBearerHeaders(authSession),
  });

  if (!response.ok) {
    await throwHttpError(response, "chargement du calendrier du jour pour l’administration");
  }

  return response.json() as Promise<AdminCalendarDayToday>;
}

export async function runAdminDailySummaryToday({
  authSession,
  force,
  localDay,
  reason,
  wsUrl,
}: AdminDailySummaryApiOptions & {
  force: boolean;
  reason: string;
}) {
  const response = await authenticatedFetch(authSession, buildAdminCalendarDayRunUrl(wsUrl), {
    method: "POST",
    headers: buildJsonBearerHeaders(authSession),
    body: JSON.stringify({ force, localDay, reason }),
  });

  if (!response.ok) {
    await throwHttpError(response, "relance du calendrier du jour par l’administration");
  }

  return response.json() as Promise<AdminCalendarDayToday>;
}

export async function runAdminCalendarWeek({
  authSession,
  force,
  localDay,
  reason,
  wsUrl,
}: AdminDailySummaryApiOptions & {
  force: boolean;
  reason: string;
}) {
  const response = await authenticatedFetch(authSession, buildAdminCalendarWeekRunUrl(wsUrl), {
    method: "POST",
    headers: buildJsonBearerHeaders(authSession),
    body: JSON.stringify({ force, referenceDay: localDay, reason }),
  });

  if (!response.ok) {
    await throwHttpError(response, "relance de la semaine du calendrier par l’administration");
  }

  return response.json() as Promise<AdminCalendarWeekRun>;
}
