import { authenticatedFetch } from "@shared/api/authSession";
import type { AuthSessionClient } from "@shared/api/authSession";
import type { XRuntimeStatus } from "@shared/api/contracts";
import {
  buildBearerHeaders,
  buildJsonBearerHeaders,
} from "@shared/api/authTransport";
import { buildXRuntimeUrl } from "@shared/api/urls";

type XRuntimeApiOptions = {
  authSession: AuthSessionClient | null;
  wsUrl: string;
};

type XRuntimeUpdateRequest = {
  enabled?: boolean;
  account_usernames?: string[];
  keywords?: string[];
};

export async function loadXRuntimeStatus({
  authSession,
  wsUrl,
}: XRuntimeApiOptions) {
  const response = await authenticatedFetch(authSession, buildXRuntimeUrl(wsUrl), {
    headers: buildBearerHeaders(authSession),
  });

  if (!response.ok) {
    throw new Error(
      await responseErrorMessage(response, "Chargement X impossible"),
    );
  }

  return response.json() as Promise<XRuntimeStatus>;
}

export async function updateXRuntimeStatus({
  authSession,
  request,
  wsUrl,
}: XRuntimeApiOptions & { request: XRuntimeUpdateRequest }) {
  const response = await authenticatedFetch(authSession, buildXRuntimeUrl(wsUrl), {
    method: "PATCH",
    headers: buildJsonBearerHeaders(authSession),
    body: JSON.stringify(request),
  });

  if (!response.ok) {
    throw new Error(
      await responseErrorMessage(response, "Mise à jour X impossible"),
    );
  }

  return response.json() as Promise<XRuntimeStatus>;
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
