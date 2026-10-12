import type { AuthSessionClient } from "@shared/api/authSession";
import type { TranslatedNewsItem } from "@shared/api/contracts";
import { cachedJsonGet } from "@shared/api/requestCache";
import { buildTranslationDetailUrl } from "@shared/api/urls";

export async function loadTranslatedNewsDetail({
  authSession,
  id,
  wsUrl,
}: {
  authSession: AuthSessionClient | null;
  id: number;
  wsUrl: string;
}) {
  return cachedJsonGet<TranslatedNewsItem>(buildTranslationDetailUrl(wsUrl, id), {
    authSession,
    context: "détail de l’actualité traduite",
    ttlMs: 30_000,
  });
}
