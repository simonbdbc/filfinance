import type { AuthSessionClient } from "./authSession";

export function buildBearerHeaders(session: AuthSessionClient | null) {
  const token = session?.getToken();
  return token ? { Authorization: `Bearer ${token}` } : undefined;
}
export function buildJsonBearerHeaders(session: AuthSessionClient | null) {
  return { "Content-Type": "application/json", ...buildBearerHeaders(session) };
}
export function sendWebSocketAuth(socket: WebSocket, session: AuthSessionClient | null) {
  const token = session?.getToken();
  if (token) socket.send(JSON.stringify({ type: "auth", access_token: token }));
}
export function isWebSocketAuthClose(event: CloseEvent) {
  const reason = event.reason.toLowerCase();
  return event.code === 1008 && (reason.includes("auth") || reason.includes("token") || reason.includes("insufficient role"));
}
export function isExpiredWebSocketAuthClose(event: CloseEvent) {
  return event.code === 1008 && event.reason.toLowerCase().includes("token expired");
}
export const websocketAuthRejectedMessage = "Session refusée par le backend. Déconnectez-vous puis reconnectez-vous.";
