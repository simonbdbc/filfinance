import { authenticatedFetch } from "@shared/api/authSession";
import type { AuthSessionClient } from "@shared/api/authSession";
import type {
  TranslationGlossaryCreatePayload,
  TranslationGlossaryEntry,
  TranslationGlossaryListResponse,
  TranslationGlossaryPreviewRequest,
  TranslationGlossaryPreviewResponse,
  TranslationGlossaryReapplyResponse,
  TranslationGlossarySystemRulesResponse,
  TranslationGlossaryUpdatePayload,
} from "@shared/api/contracts";
import {
  buildBearerHeaders,
  buildJsonBearerHeaders,
} from "@shared/api/authTransport";
import {
  buildGlossaryEntryUrl,
  buildGlossaryPreviewUrl,
  buildGlossaryReapplyTranslationsUrl,
  buildGlossarySystemRulesUrl,
  buildGlossaryUrl,
} from "@shared/api/urls";

type GlossaryApiOptions = {
  authSession: AuthSessionClient | null;
  wsUrl: string;
};

export async function loadGlossaryEntries({
  authSession,
  wsUrl,
}: GlossaryApiOptions) {
  const response = await authenticatedFetch(authSession, buildGlossaryUrl(wsUrl), {
    headers: buildBearerHeaders(authSession),
  });

  if (!response.ok) {
    throw new Error(
      await responseErrorMessage(response, "Chargement du glossaire impossible"),
    );
  }

  return response.json() as Promise<TranslationGlossaryListResponse>;
}

export async function createGlossaryEntry({
  authSession,
  payload,
  wsUrl,
}: GlossaryApiOptions & { payload: TranslationGlossaryCreatePayload }) {
  const response = await authenticatedFetch(authSession, buildGlossaryUrl(wsUrl), {
    method: "POST",
    headers: buildJsonBearerHeaders(authSession),
    body: JSON.stringify(payload),
  });

  if (!response.ok) {
    throw new Error(
      await responseErrorMessage(response, "Création du glossaire impossible"),
    );
  }

  return response.json() as Promise<TranslationGlossaryEntry>;
}

export async function loadGlossarySystemRules({
  authSession,
  wsUrl,
}: GlossaryApiOptions) {
  const response = await authenticatedFetch(authSession, buildGlossarySystemRulesUrl(wsUrl), {
    headers: buildBearerHeaders(authSession),
  });

  if (!response.ok) {
    throw new Error(
      await responseErrorMessage(response, "Chargement des règles système impossible"),
    );
  }

  return response.json() as Promise<TranslationGlossarySystemRulesResponse>;
}

export async function previewGlossaryPipeline({
  authSession,
  payload,
  wsUrl,
}: GlossaryApiOptions & { payload: TranslationGlossaryPreviewRequest }) {
  const response = await authenticatedFetch(authSession, buildGlossaryPreviewUrl(wsUrl), {
    method: "POST",
    headers: buildJsonBearerHeaders(authSession),
    body: JSON.stringify(payload),
  });

  if (!response.ok) {
    throw new Error(
      await responseErrorMessage(response, "Aperçu diction TTS impossible"),
    );
  }

  return response.json() as Promise<TranslationGlossaryPreviewResponse>;
}

export async function updateGlossaryEntry({
  authSession,
  id,
  payload,
  wsUrl,
}: GlossaryApiOptions & {
  id: number;
  payload: TranslationGlossaryUpdatePayload;
}) {
  const response = await authenticatedFetch(authSession, buildGlossaryEntryUrl(wsUrl, id), {
    method: "PATCH",
    headers: buildJsonBearerHeaders(authSession),
    body: JSON.stringify(payload),
  });

  if (!response.ok) {
    throw new Error(
      await responseErrorMessage(response, "Mise à jour du glossaire impossible"),
    );
  }

  return response.json() as Promise<TranslationGlossaryEntry>;
}

export async function disableGlossaryEntry({
  authSession,
  id,
  wsUrl,
}: GlossaryApiOptions & { id: number }) {
  const response = await authenticatedFetch(authSession, buildGlossaryEntryUrl(wsUrl, id), {
    method: "DELETE",
    headers: buildBearerHeaders(authSession),
  });

  if (!response.ok) {
    throw new Error(
      await responseErrorMessage(response, "Désactivation du glossaire impossible"),
    );
  }

  return response.json() as Promise<TranslationGlossaryEntry>;
}

export async function reapplyGlossaryTranslations({
  authSession,
  wsUrl,
}: GlossaryApiOptions) {
  const response = await authenticatedFetch(authSession, buildGlossaryReapplyTranslationsUrl(wsUrl), {
    method: "POST",
    headers: buildBearerHeaders(authSession),
  });

  if (!response.ok) {
    throw new Error(
      await responseErrorMessage(
        response,
        "Mise à jour des traductions impossible",
      ),
    );
  }

  return response.json() as Promise<TranslationGlossaryReapplyResponse>;
}

async function responseErrorMessage(response: Response, fallback: string) {
  try {
    const payload = (await response.json()) as { error?: string };
    if (payload.error) {
      return `${fallback}: ${payload.error}`;
    }
  } catch {
    // Le backend renvoie normalement `{ error }`; si ce n'est pas le cas, on garde le status HTTP.
  }

  return `${fallback}: HTTP ${response.status}`;
}
