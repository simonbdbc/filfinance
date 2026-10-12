import { authenticatedFetch, SessionExpiredError } from "@shared/api/authSession";
import type { AuthSessionClient } from "@shared/api/authSession";
import { buildBearerHeaders, sendWebSocketAuth } from "@shared/api/authTransport";
import { deriveApiBaseUrl } from "@shared/api/urls";
import { mergeReaderHeaders } from "./headers";

export class ApiError extends Error {
  constructor(public code: string, message: string) { super(message); }
}

/** Same reader protocol, transported through Filfinance's authenticated service. */
export class Api {
  token = "";
  serverId = "";
  readonly liveUrl: string;
  private readonly base: string;
  private queue: Promise<unknown> = Promise.resolve();
  private waiting = 0;
  private closed = false;
  private readonly abort = new AbortController();
  constructor(wsUrl: string, private readonly authSession: AuthSessionClient | null) {
    this.base = deriveApiBaseUrl(wsUrl);
    const live = new URL(wsUrl);
    live.pathname = "/ws/admin/calendar-day/logs";
    live.search = "";
    live.hash = "";
    this.liveUrl = live.toString();
  }
  async prepareConnection() { if (!this.authSession) throw new SessionExpiredError(); return this.authSession.validToken(); }
  async recoverConnection(token: string | null) { if (!this.authSession) throw new SessionExpiredError(); return this.authSession.recover(token); }
  authenticate(socket: WebSocket) {
    if (this.authSession) sendWebSocketAuth(socket, this.authSession);
    return this.authSession?.getToken() ?? null;
  }
  async initialize() {
    const data = await this.request<{ token: string; serverId: string }>("/api/v1/session", { method: "POST" });
    this.token = data.token;
    this.serverId = data.serverId;
    if (this.closed) await this.close();
  }
  request<T>(url: string, options: RequestInit = {}): Promise<T> {
    if (this.closed) return Promise.reject(new ApiError("CLOSED", "Lecteur fermé."));
    if (this.waiting >= 16) return Promise.reject(new ApiError("LIMIT", "Trop de requêtes en attente."));
    this.waiting++;
    const next = this.queue.catch(() => {}).then(async () => {
      for (let attempt = 0; attempt < 20; attempt++) {
        if (this.closed) throw new ApiError("CLOSED", "Lecteur fermé.");
        let response: Response;
        try {
          response = await authenticatedFetch(this.authSession, this.url(url), {
            ...options,
            signal: this.abort.signal,
            cache: "no-store",
            headers: mergeReaderHeaders(options.headers, {
              ...buildBearerHeaders(this.authSession),
              "Content-Type": "application/json",
              "x-local-session": this.token,
            }),
          });
        } catch (error) {
          if (error instanceof DOMException && error.name === "AbortError") {
            throw new ApiError("CLOSED", "Lecteur fermé.");
          }
          throw new ApiError("NETWORK", "Le service de lecture est indisponible.");
        }
        const data = await response.json().catch(() => ({}));
        if (response.status === 429 && attempt < 19) {
          await new Promise((resolve) => setTimeout(resolve, 50));
          continue;
        }
        if (!response.ok) throw new ApiError(data.code ?? `HTTP_${response.status}`, data.message ?? data.error ?? "Lecture refusée.");
        return data as T;
      }
      throw new ApiError("UNAVAILABLE", "Lecture indisponible.");
    }).finally(() => { this.waiting--; });
    this.queue = next;
    return next;
  }
  private url(path: string) {
    return this.base + path.replace("/api/v1", "/api/admin/calendar-day/logs");
  }
  async download(path: string): Promise<Blob> {
    if (this.closed) throw new ApiError("CLOSED", "Lecteur fermé.");
    let response: Response;
    try {
      response = await authenticatedFetch(this.authSession, this.url(path), {
        signal: this.abort.signal,
        cache: "no-store",
        headers: {
          ...buildBearerHeaders(this.authSession),
          "x-local-session": this.token,
        },
      });
    } catch {
      throw new ApiError("NETWORK", "Le téléchargement est indisponible.");
    }
    if (!response.ok) {
      const data = await response.json().catch(() => ({}));
      throw new ApiError(data.code ?? `HTTP_${response.status}`, data.message ?? "Téléchargement refusé.");
    }
    return response.blob();
  }
  async close() {
    this.closed = true;
    this.abort.abort();
    if (this.token) await authenticatedFetch(this.authSession, this.url("/api/v1/session"), {
      method: "DELETE", keepalive: true,
      headers: { ...buildBearerHeaders(this.authSession), "x-local-session": this.token },
    }).catch(() => {});
  }
}
