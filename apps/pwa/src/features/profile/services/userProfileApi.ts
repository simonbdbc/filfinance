import { authenticatedFetch } from "@shared/api/authSession";
import type { AuthSessionClient } from "@shared/api/authSession";
import type { CurrentUserProfileResponse } from "@shared/api/contracts";
import { buildJsonBearerHeaders } from "@shared/api/authTransport";
import { throwHttpError } from "@shared/api/httpErrors";
import {
  cachedJsonGet,
  invalidateCachedJsonGet,
} from "@shared/api/requestCache";
import { defaultApiBaseUrl } from "@shared/api/urls";

type LoadCurrentUserProfileOptions = {
  wsUrl: string;
  authSession: AuthSessionClient | null;
  signal?: AbortSignal;
};

type UpdateUserPreferencesOptions = {
  wsUrl: string;
  authSession: AuthSessionClient | null;
  preferredTtsVoice?: string | null;
};

function buildMeUrl(_wsUrl: string) {
  return new URL("/api/me", `${defaultApiBaseUrl()}/`).toString();
}

export async function loadCurrentUserProfile({
  wsUrl,
  authSession,
  signal,
}: LoadCurrentUserProfileOptions) {
  return cachedJsonGet<CurrentUserProfileResponse>(buildMeUrl(wsUrl), {
    authSession,
    context: "profil utilisateur",
    signal,
    ttlMs: 10_000,
  });
}

export async function updateUserPreferences({
  wsUrl,
  authSession,
  preferredTtsVoice,
}: UpdateUserPreferencesOptions) {
  const payload: Record<string, string | null> = {};

  if (preferredTtsVoice !== undefined) {
    payload.preferred_tts_voice = preferredTtsVoice;
  }

  const response = await authenticatedFetch(authSession, buildMeUrl(wsUrl), {
    body: JSON.stringify(payload),
    headers: buildJsonBearerHeaders(authSession),
    method: "PATCH",
  });

  if (!response.ok) {
    await throwHttpError(response, "préférences de l’utilisateur");
  }

  invalidateCachedJsonGet(buildMeUrl(wsUrl), authSession);
  return (await response.json()) as CurrentUserProfileResponse;
}
