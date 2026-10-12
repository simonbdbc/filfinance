//! Points d'entrée WebSocket pour la diffusion user-facing et les traces de test.

// `axum` fournit ici les extracteurs HTTP/WebSocket.
use axum::{
    extract::{Query, State, ws::WebSocketUpgrade},
    response::Response,
};

use crate::app_state::AppState;
use crate::{errors::AppError, features::live::edge::verify_live_ticket};
use serde::Deserialize;

// `auth` réserve le handshake d'authentification WebSocket protégé.
mod auth;
// `edge` garde le contrat `/ws/live` quand Bunny transporte les WebSockets DEV.
mod edge;
// `news` réserve le flux user-facing `/ws/news`.
mod news;
// `serialization` réserve les conversions DTO WebSocket -> JSON texte.
mod serialization;
// `test_trace` réserve le WebSocket de debug et ses commandes de run de test.
mod test_trace;
// `transport` réserve l'écriture texte commune vers un socket splitté.
mod transport;

use auth::authenticate_socket;
#[allow(unused_imports)]
pub(in crate::features::live) use auth::parse_websocket_auth_token;
#[allow(unused_imports)]
pub(in crate::features::live) use serialization::{
    serialize_live_metadata_message, serialize_message, serialize_test_trace_message,
};
use transport::send_text;

/// Intervalle de heartbeat applicatif pour limiter les expirations de sockets inactives.
const HEARTBEAT_INTERVAL_SECS: u64 = 20;
/// Nombre maximal de traductions FR rejouées à l'ouverture d'une connexion.
const TRANSLATION_REPLAY_LIMIT: i64 = 3;

/// Ouvre le flux WebSocket user-facing alimenté par le broadcast des traductions.
pub async fn ws_news(State(state): State<AppState>, ws: WebSocketUpgrade) -> Response {
    // `on_upgrade(...)` diffère le vrai traitement après le handshake HTTP -> WebSocket.
    ws.on_upgrade(move |socket| news::handle_news_socket(socket, state))
}

#[derive(Deserialize)]
pub struct EdgeLiveQuery {
    ticket: String,
}

/// Ouvre `/ws/live?ticket=...` pour Bunny DEV après vérification locale du JWT HMAC court.
pub async fn ws_live(
    State(state): State<AppState>,
    Query(query): Query<EdgeLiveQuery>,
    ws: WebSocketUpgrade,
) -> Result<Response, AppError> {
    verify_live_ticket(&state.config.edge, query.ticket.trim())?;
    Ok(ws.on_upgrade(move |socket| edge::handle_edge_live_socket(socket, state)))
}

/// Ouvre le flux WebSocket de debug et accepte les commandes de run de test.
pub async fn ws_test_runs(State(state): State<AppState>, ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(move |socket| test_trace::handle_test_trace_socket(socket, state))
}
