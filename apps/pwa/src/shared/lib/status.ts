import type { AudioPlaybackEngine, ConnectionState } from "../api/contracts";

export const WS_ERROR_MESSAGE = "Connexion WebSocket instable";
export const WS_ERROR_DELAY_MS = 5_000;

export function connectionLabel(state: ConnectionState) {
  switch (state) {
    case "open":
      return "Connecté";
    case "waiting":
      return "En attente";
    case "connecting":
      return "Connexion";
    case "error":
      return "Erreur";
    case "closed":
      return "Fermé";
    default:
      return "Inactif";
  }
}

export function formatWsCloseMessage(event: { code?: number; reason?: string }) {
  const code = typeof event.code === "number" ? event.code : undefined;
  const reason = event.reason?.trim();

  if (code && reason) {
    return `WebSocket fermé (${code}) : ${reason}`;
  }

  if (code) {
    return `WebSocket fermé (${code})`;
  }

  return WS_ERROR_MESSAGE;
}

export function traceStatusLabel(status: string) {
  switch (status) {
    case "ok":
      return "OK";
    case "warning":
      return "WARN";
    case "error":
      return "ERR";
    default:
      return "INFO";
  }
}

export function audioPlaybackEngineLabel(engine: AudioPlaybackEngine) {
  switch (engine) {
    case "hls_js":
      return "HLS.js";
    case "hls_native":
      return "HLS natif";
    default:
      return "Initialisation";
  }
}
