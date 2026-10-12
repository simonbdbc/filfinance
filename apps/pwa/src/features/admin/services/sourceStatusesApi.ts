import type { AuthSessionClient } from "@shared/api/authSession";
import type { SourceStatus } from "@shared/api/contracts";
import { cachedJsonGet } from "@shared/api/requestCache";
import { buildSourceStatusesUrl } from "@shared/api/urls";

type LoadAdminSourceStatusesOptions = {
  authSession: AuthSessionClient | null;
  signal: AbortSignal;
  wsUrl: string;
};

export async function loadAdminSourceStatuses({
  authSession,
  signal,
  wsUrl,
}: LoadAdminSourceStatusesOptions) {
  return cachedJsonGet<SourceStatus[]>(buildSourceStatusesUrl(wsUrl), {
    authSession,
    context: "statut admin des sources",
    signal,
    ttlMs: 5_000,
  });
}
