import { authenticatedFetch } from "@shared/api/authSession";
import type { AuthSessionClient } from "@shared/api/authSession";
import type { AdminLogDaysResponse, AdminLogPage } from "@shared/api/contracts";
import { buildBearerHeaders } from "@shared/api/authTransport";
import { throwHttpError } from "@shared/api/httpErrors";
import { buildAdminLogDaysUrl, buildAdminLogsUrl } from "@shared/api/urls";

type AdminLogsApiOptions = {
  authSession: AuthSessionClient | null;
  wsUrl: string;
};

export async function loadAdminLogDays({
  authSession,
  wsUrl,
}: AdminLogsApiOptions) {
  const response = await authenticatedFetch(authSession, buildAdminLogDaysUrl(wsUrl), {
    headers: buildBearerHeaders(authSession),
  });

  if (!response.ok) {
    await throwHttpError(response, "chargement des jours des journaux");
  }

  return response.json() as Promise<AdminLogDaysResponse>;
}

export async function loadAdminLogsPage({
  authSession,
  cursor,
  day,
  level,
  query,
  wsUrl,
}: AdminLogsApiOptions & {
  cursor?: number | null;
  day: string;
  level?: string | null;
  query?: string | null;
}) {
  const response = await authenticatedFetch(authSession,
    buildAdminLogsUrl(wsUrl, {
      cursor,
      day,
      level,
      limit: 200,
      query,
    }),
    {
      headers: buildBearerHeaders(authSession),
    },
  );

  if (!response.ok) {
    await throwHttpError(response, "chargement des journaux");
  }

  return response.json() as Promise<AdminLogPage>;
}
