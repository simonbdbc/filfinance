import { authenticatedFetch } from "@shared/api/authSession";
import type { AuthSessionClient } from "@shared/api/authSession";
import type {
  FinancialJuiceBrowserRuntimeStatus,
  FinancialJuiceRuntimeStatus,
} from "@shared/api/contracts";
import {
  buildBearerHeaders,
  buildJsonBearerHeaders,
} from "@shared/api/authTransport";
import {
  buildFinancialJuiceBrowserRuntimeUrl,
  buildFinancialJuiceRuntimeUrl,
} from "@shared/api/urls";

type FinancialJuiceRuntimeApiOptions = {
  authSession: AuthSessionClient | null;
  wsUrl: string;
};

export async function loadFinancialJuiceRuntimeStatus({
  authSession,
  wsUrl,
}: FinancialJuiceRuntimeApiOptions) {
  const response = await authenticatedFetch(authSession, buildFinancialJuiceRuntimeUrl(wsUrl), {
    headers: buildBearerHeaders(authSession),
  });

  if (!response.ok) {
    throw new Error(`Chargement FinancialJuice impossible : HTTP ${response.status}`);
  }

  return response.json() as Promise<FinancialJuiceRuntimeStatus>;
}

export async function updateFinancialJuiceRuntimeStatus({
  authSession,
  enabled,
  wsUrl,
}: FinancialJuiceRuntimeApiOptions & { enabled: boolean }) {
  const response = await authenticatedFetch(authSession, buildFinancialJuiceRuntimeUrl(wsUrl), {
    method: "PATCH",
    headers: buildJsonBearerHeaders(authSession),
    body: JSON.stringify({ enabled }),
  });

  if (!response.ok) {
    throw new Error(`Mise à jour FinancialJuice impossible : HTTP ${response.status}`);
  }

  return response.json() as Promise<FinancialJuiceRuntimeStatus>;
}

export async function loadFinancialJuiceBrowserRuntimeStatus({
  authSession,
  wsUrl,
}: FinancialJuiceRuntimeApiOptions) {
  const response = await authenticatedFetch(authSession, buildFinancialJuiceBrowserRuntimeUrl(wsUrl), {
    headers: buildBearerHeaders(authSession),
  });

  if (!response.ok) {
    throw new Error(`Chargement FinancialJuice browser impossible : HTTP ${response.status}`);
  }

  return response.json() as Promise<FinancialJuiceBrowserRuntimeStatus>;
}
