import { authenticatedFetch } from "@shared/api/authSession";
import type { AuthSessionClient } from "@shared/api/authSession";
import type {
  NewsBlacklistCreatePayload,
  NewsBlacklistEntry,
  NewsBlacklistHitsResponse,
  NewsBlacklistListResponse,
  NewsBlacklistReapplyResponse,
  NewsBlacklistUpdatePayload,
} from "@shared/api/contracts";
import {
  buildBearerHeaders,
  buildJsonBearerHeaders,
} from "@shared/api/authTransport";
import {
  buildNewsBlacklistEntryUrl,
  buildNewsBlacklistHitsUrl,
  buildNewsBlacklistReapplyUrl,
  buildNewsBlacklistUrl,
} from "@shared/api/urls";

type NewsBlacklistApiOptions = {
  authSession: AuthSessionClient | null;
  wsUrl: string;
};

export async function loadNewsBlacklistEntries({
  authSession,
  wsUrl,
}: NewsBlacklistApiOptions) {
  const response = await authenticatedFetch(authSession, buildNewsBlacklistUrl(wsUrl), {
    headers: buildBearerHeaders(authSession),
  });

  if (!response.ok) {
    throw new Error(
      await responseErrorMessage(response, "Chargement de la liste de blocage impossible"),
    );
  }

  return response.json() as Promise<NewsBlacklistListResponse>;
}

export async function createNewsBlacklistEntry({
  authSession,
  payload,
  wsUrl,
}: NewsBlacklistApiOptions & { payload: NewsBlacklistCreatePayload }) {
  const response = await authenticatedFetch(authSession, buildNewsBlacklistUrl(wsUrl), {
    method: "POST",
    headers: buildJsonBearerHeaders(authSession),
    body: JSON.stringify(payload),
  });

  if (!response.ok) {
    throw new Error(
      await responseErrorMessage(response, "Création de la liste de blocage impossible"),
    );
  }

  return response.json() as Promise<NewsBlacklistEntry>;
}

export async function updateNewsBlacklistEntry({
  authSession,
  id,
  payload,
  wsUrl,
}: NewsBlacklistApiOptions & {
  id: number;
  payload: NewsBlacklistUpdatePayload;
}) {
  const response = await authenticatedFetch(authSession, buildNewsBlacklistEntryUrl(wsUrl, id), {
    method: "PATCH",
    headers: buildJsonBearerHeaders(authSession),
    body: JSON.stringify(payload),
  });

  if (!response.ok) {
    throw new Error(
      await responseErrorMessage(response, "Mise à jour de la liste de blocage impossible"),
    );
  }

  return response.json() as Promise<NewsBlacklistEntry>;
}

export async function disableNewsBlacklistEntry({
  authSession,
  id,
  wsUrl,
}: NewsBlacklistApiOptions & { id: number }) {
  const response = await authenticatedFetch(authSession, buildNewsBlacklistEntryUrl(wsUrl, id), {
    method: "DELETE",
    headers: buildBearerHeaders(authSession),
  });

  if (!response.ok) {
    throw new Error(
      await responseErrorMessage(response, "Désactivation de la liste de blocage impossible"),
    );
  }

  return response.json() as Promise<NewsBlacklistEntry>;
}

export async function loadNewsBlacklistHits({
  authSession,
  wsUrl,
}: NewsBlacklistApiOptions) {
  const response = await authenticatedFetch(authSession, buildNewsBlacklistHitsUrl(wsUrl), {
    headers: buildBearerHeaders(authSession),
  });

  if (!response.ok) {
    throw new Error(
      await responseErrorMessage(response, "Chargement des rejets impossible"),
    );
  }

  return response.json() as Promise<NewsBlacklistHitsResponse>;
}

export async function reapplyNewsBlacklist({
  authSession,
  wsUrl,
}: NewsBlacklistApiOptions) {
  const response = await authenticatedFetch(authSession, buildNewsBlacklistReapplyUrl(wsUrl), {
    method: "POST",
    headers: buildBearerHeaders(authSession),
  });

  if (!response.ok) {
    throw new Error(
      await responseErrorMessage(response, "Mise à jour de la liste de blocage impossible"),
    );
  }

  return response.json() as Promise<NewsBlacklistReapplyResponse>;
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
