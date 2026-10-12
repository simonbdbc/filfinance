import type { AuthSessionClient } from "@shared/api/authSession";
import type { NewsTruncationAuditsResponse } from "@shared/api/contracts";
import { cachedJsonGet } from "@shared/api/requestCache";
import { buildNewsTruncationAuditUrl } from "@shared/api/urls";

type LoadNewsTruncationAuditsOptions = {
  authSession: AuthSessionClient | null;
  wsUrl: string;
};

export async function loadNewsTruncationAudits({
  authSession,
  wsUrl,
}: LoadNewsTruncationAuditsOptions) {
  return cachedJsonGet<NewsTruncationAuditsResponse>(buildNewsTruncationAuditUrl(wsUrl), {
    authSession,
    context: "Chargement des troncatures impossible",
    ttlMs: 30_000,
  });
}
