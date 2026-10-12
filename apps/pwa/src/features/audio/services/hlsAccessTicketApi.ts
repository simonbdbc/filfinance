import type { AuthSessionClient } from "@shared/api/authSession";
import type { HlsAccessTicketResponse } from "@shared/api/contracts";
import { cachedJsonGet } from "@shared/api/requestCache";

type LoadHlsAccessTicketOptions = {
  ticketUrl: string;
  authSession: AuthSessionClient | null;
};

export async function loadHlsAccessTicket({
  ticketUrl,
  authSession,
}: LoadHlsAccessTicketOptions): Promise<HlsAccessTicketResponse> {
  return cachedJsonGet<HlsAccessTicketResponse>(ticketUrl, {
    authSession,
    context: "ticket HLS",
    ttlMs: 10_000,
  });
}

export function appendHlsAccessTicket(sourceUrl: string, ticket: string) {
  try {
    const parsed = new URL(sourceUrl);
    parsed.searchParams.set("hls_ticket", ticket);
    return parsed.toString();
  } catch {
    const separator = sourceUrl.includes("?") ? "&" : "?";
    return `${sourceUrl}${separator}hls_ticket=${encodeURIComponent(ticket)}`;
  }
}
