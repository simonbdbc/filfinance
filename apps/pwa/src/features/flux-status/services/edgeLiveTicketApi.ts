import type { AuthSessionClient } from "@shared/api/authSession";
import type { EdgeLiveTicketResponse } from "@shared/api/contracts";
import { cachedJsonGet } from "@shared/api/requestCache";

type LoadEdgeLiveTicketOptions = {
  ticketUrl: string;
  authSession: AuthSessionClient | null;
};

export async function loadEdgeLiveTicket({
  ticketUrl,
  authSession,
}: LoadEdgeLiveTicketOptions): Promise<EdgeLiveTicketResponse> {
  return cachedJsonGet<EdgeLiveTicketResponse>(ticketUrl, {
    authSession,
    context: "ticket edge",
    ttlMs: 10_000,
  });
}

export function appendEdgeLiveTicket(wsUrl: string, ticket: string) {
  try {
    const parsed = new URL(wsUrl);
    parsed.searchParams.set("ticket", ticket);
    return parsed.toString();
  } catch {
    const separator = wsUrl.includes("?") ? "&" : "?";
    return `${wsUrl}${separator}ticket=${encodeURIComponent(ticket)}`;
  }
}
