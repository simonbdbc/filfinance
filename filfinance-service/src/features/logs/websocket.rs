//! WebSocket admin pour suivre les logs backend en direct.

// `WebSocketUpgrade` realise l'upgrade HTTP, puis `WebSocket` transporte les frames texte.
use axum::{
    extract::{
        Query, State,
        ws::{CloseFrame, Message, WebSocket, WebSocketUpgrade, close_code},
    },
    response::Response,
};
// `StreamExt` et `SinkExt` pilotent les deux demi-flux WebSocket apres split.
use futures_util::{SinkExt, StreamExt, stream::SplitSink};
// `Deserialize` parse la query string et le premier message d'auth.
use serde::{Deserialize, Serialize};
// `Duration` et `interval` cadencent heartbeat et polling fichier.
use tokio::time::{Duration, MissedTickBehavior, interval, timeout};
use tracing::{info, warn};

use crate::{
    app_state::AppState,
    features::{
        auth::{self, CurrentUser},
        logs::{
            domain::{AdminLogTailAppend, AdminLogTailSnapshot},
            service::{self, AdminLogTailCursor, AdminLogTailParams},
        },
    },
    shared::time::now_rfc3339,
};

/// Delai laisse au client pour envoyer le premier message `{ type: "auth" }`.
const LOGS_WS_AUTH_TIMEOUT_SECS: u64 = 5;
/// Intervalle de heartbeat applicatif pour rendre l'etat de connexion explicite cote frontend.
const LOGS_WS_HEARTBEAT_SECS: u64 = 20;
/// Polling court suffisant pour un tail de fichier local sans watcher OS dedie.
const LOGS_WS_TAIL_POLL_MS: u64 = 1000;
/// Version du protocole WebSocket logs.
const LOGS_WS_PROTOCOL_VERSION: u8 = 1;

/// Query string acceptee par `/ws/admin/logs`.
#[derive(Debug, Clone, Deserialize)]
pub struct AdminLogTailQuery {
    /// Jour UTC demande; absent signifie aujourd'hui.
    pub day: Option<String>,
    /// Limite du snapshot initial, bornee par `NEWS_LOG_ADMIN_MAX_LINES`.
    pub limit: Option<usize>,
    /// Filtre de niveau optionnel.
    pub level: Option<String>,
    /// Recherche texte optionnelle.
    pub q: Option<String>,
}

/// Premier message attendu pour authentifier une socket admin.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientAuthCommand {
    /// Le token passe dans une frame car le navigateur ne permet pas un header WS custom fiable.
    Auth { access_token: String },
}

/// Enveloppe JSON commune aux messages du flux logs.
#[derive(Debug, Serialize)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
enum AdminLogWsMessage {
    /// Confirme l'ouverture logique apres authentification moderator.
    Hello(AdminLogWsHello),
    /// Snapshot initial des dernieres lignes.
    Snapshot(AdminLogTailSnapshot),
    /// Append incremental depuis la derniere position fichier.
    Append(AdminLogTailAppend),
    /// Heartbeat periodique.
    Heartbeat(AdminLogWsHeartbeat),
    /// Erreur fonctionnelle envoyee avant fermeture ou retry client.
    Error(AdminLogWsError),
}

/// Payload du message initial.
#[derive(Debug, Serialize)]
struct AdminLogWsHello {
    /// Version du contrat de frames.
    protocol_version: u8,
    /// Heure serveur d'ouverture.
    server_time: String,
    /// Utilisateur authentifie, utile en debug admin sans exposer le token.
    user_id: String,
}

/// Payload heartbeat minimal.
#[derive(Debug, Serialize)]
struct AdminLogWsHeartbeat {
    /// Heure serveur d'emission.
    server_time: String,
}

/// Payload d'erreur affichable cote admin.
#[derive(Debug, Serialize)]
struct AdminLogWsError {
    /// Message court et non sensible.
    message: String,
}

/// Ouvre le WebSocket admin de lecture live des logs backend.
pub async fn ws_admin_logs(
    State(state): State<AppState>,
    Query(query): Query<AdminLogTailQuery>,
    ws: WebSocketUpgrade,
) -> Response {
    // La vraie boucle demarre apres l'upgrade HTTP -> WebSocket.
    ws.on_upgrade(move |socket| handle_admin_log_socket(socket, state, query))
}

async fn handle_admin_log_socket(mut socket: WebSocket, state: AppState, query: AdminLogTailQuery) {
    let user = match authenticate_log_socket(&mut socket, &state).await {
        Ok(user) => user,
        Err(()) => return,
    };
    let access_token_expires_at = user.access_token_expires_at;

    let params = AdminLogTailParams {
        day: query.day,
        limit: query.limit,
        level: query.level,
        query: query.q,
    };
    let snapshot_result = auth::complete_before_access_token_expiration(
        access_token_expires_at,
        service::read_tail_snapshot(&state.config.logging, params),
    )
    .await;
    let (snapshot, mut cursor) = match snapshot_result {
        None => {
            let _ = socket
                .send(auth::access_token_expired_close_message())
                .await;
            return;
        }
        Some(Ok(result)) => result,
        Some(Err(error)) => {
            warn!(%error, "admin logs websocket snapshot failed");
            let _ =
                send_socket_error_before_split(&mut socket, "impossible de lire les logs").await;
            return;
        }
    };

    let (mut sender, mut receiver) = socket.split();
    let completed = auth::complete_before_access_token_expiration(
        access_token_expires_at,
        async {
            if send_ws_message(
                &mut sender,
                &AdminLogWsMessage::Hello(AdminLogWsHello {
                    protocol_version: LOGS_WS_PROTOCOL_VERSION,
                    server_time: now_rfc3339(),
                    user_id: user.id,
                }),
            )
            .await
            .is_err()
            {
                return;
            }

            if send_ws_message(&mut sender, &AdminLogWsMessage::Snapshot(snapshot))
                .await
                .is_err()
            {
                return;
            }

            info!(
                day = %cursor.day,
                "admin logs websocket tail opened"
            );

            let mut heartbeat_interval = interval(Duration::from_secs(LOGS_WS_HEARTBEAT_SECS));
            heartbeat_interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
            let mut tail_interval = interval(Duration::from_millis(LOGS_WS_TAIL_POLL_MS));
            tail_interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

            loop {
                tokio::select! {
                    _ = heartbeat_interval.tick() => {
                        if send_ws_message(
                            &mut sender,
                            &AdminLogWsMessage::Heartbeat(AdminLogWsHeartbeat {
                                server_time: now_rfc3339(),
                            }),
                        )
                        .await
                        .is_err()
                        {
                            break;
                        }
                    }
                    _ = tail_interval.tick() => {
                        match poll_log_append(cursor).await {
                            Ok((append, next_cursor)) => {
                                cursor = next_cursor;
                                if append.items.is_empty() {
                                    continue;
                                }
                                if send_ws_message(&mut sender, &AdminLogWsMessage::Append(append)).await.is_err() {
                                    break;
                                }
                            }
                            Err(error) => {
                                warn!(%error, "admin logs websocket tail poll failed");
                                let _ = send_ws_message(
                                    &mut sender,
                                    &AdminLogWsMessage::Error(AdminLogWsError {
                                        message: "lecture live des logs impossible".to_owned(),
                                    }),
                                )
                                .await;
                                break;
                            }
                        }
                    }
                    incoming = receiver.next() => {
                        match incoming {
                            Some(Ok(Message::Ping(payload))) => {
                                if sender.send(Message::Pong(payload)).await.is_err() {
                                    break;
                                }
                            }
                            Some(Ok(Message::Pong(_))) | Some(Ok(Message::Text(_))) => {}
                            Some(Ok(Message::Close(_))) | None => break,
                            Some(Ok(Message::Binary(_))) => {
                                let _ = send_ws_message(
                                    &mut sender,
                                    &AdminLogWsMessage::Error(AdminLogWsError {
                                        message: "les commandes logs doivent etre textuelles".to_owned(),
                                    }),
                                )
                                .await;
                            }
                            Some(Err(error)) => {
                                warn!(%error, "admin logs websocket receive failed");
                                break;
                            }
                        }
                    }
                }
            }
        },
    )
    .await;

    if completed.is_none() {
        let _ = sender
            .send(auth::access_token_expired_close_message())
            .await;
    }
}

async fn poll_log_append(
    cursor: AdminLogTailCursor,
) -> Result<(AdminLogTailAppend, AdminLogTailCursor), crate::errors::AppError> {
    service::read_tail_append(cursor).await
}

async fn authenticate_log_socket(
    socket: &mut WebSocket,
    state: &AppState,
) -> Result<CurrentUser, ()> {
    if !state.config.auth.enabled {
        return Ok(CurrentUser::development());
    }

    let token = match timeout(
        Duration::from_secs(LOGS_WS_AUTH_TIMEOUT_SECS),
        read_websocket_auth_token(socket),
    )
    .await
    {
        Ok(Ok(token)) => token,
        Ok(Err(reason)) => {
            close_socket_with_policy(socket, reason).await;
            return Err(());
        }
        Err(_) => {
            close_socket_with_policy(socket, "auth timeout").await;
            return Err(());
        }
    };

    let identity = match auth::authenticate_access_token(&state.config.auth, &token).await {
        Ok(identity) => identity,
        Err(error) => {
            warn!(%error, stream = "admin_logs", "websocket auth rejected invalid bearer token");
            close_socket_with_policy(socket, "invalid auth token").await;
            return Err(());
        }
    };

    let user = match auth::sync_current_user_profile(state, &identity).await {
        Ok(user) => user,
        Err(error) => {
            warn!(%error, stream = "admin_logs", "websocket auth failed to sync user profile");
            close_socket_with_policy(socket, "profile persistence failed").await;
            return Err(());
        }
    };

    if !user.can_access_moderator_routes(&state.config.auth) {
        warn!(
            user_id = %user.id,
            user_role = %user.role,
            stream = "admin_logs",
            "websocket auth rejected insufficient role"
        );
        close_socket_with_policy(socket, "insufficient role").await;
        return Err(());
    }

    Ok(user)
}

async fn read_websocket_auth_token(socket: &mut WebSocket) -> Result<String, &'static str> {
    loop {
        let message = socket.recv().await.ok_or("auth socket closed")?;
        let message = message.map_err(|_| "auth transport error")?;

        match message {
            Message::Text(payload) => return parse_websocket_auth_token(payload.as_str()),
            Message::Ping(payload) => {
                if socket.send(Message::Pong(payload)).await.is_err() {
                    return Err("auth pong failed");
                }
            }
            Message::Pong(_) => {}
            Message::Close(_) => return Err("auth socket closed"),
            Message::Binary(_) => return Err("auth message must be text"),
        }
    }
}

fn parse_websocket_auth_token(payload: &str) -> Result<String, &'static str> {
    let command =
        serde_json::from_str::<ClientAuthCommand>(payload).map_err(|_| "invalid auth payload")?;
    match command {
        ClientAuthCommand::Auth { access_token } => {
            let token = access_token.trim();
            if token.is_empty() {
                return Err("empty auth token");
            }
            Ok(token.to_owned())
        }
    }
}

async fn close_socket_with_policy(socket: &mut WebSocket, reason: &'static str) {
    let _ = socket
        .send(Message::Close(Some(CloseFrame {
            code: close_code::POLICY,
            reason: reason.into(),
        })))
        .await;
}

async fn send_socket_error_before_split(
    socket: &mut WebSocket,
    message: impl Into<String>,
) -> Result<(), ()> {
    let payload = serde_json::to_string(&AdminLogWsMessage::Error(AdminLogWsError {
        message: message.into(),
    }))
    .map_err(|_| ())?;
    socket
        .send(Message::Text(payload.into()))
        .await
        .map_err(|_| ())
}

async fn send_ws_message(
    sender: &mut SplitSink<WebSocket, Message>,
    message: &AdminLogWsMessage,
) -> Result<(), ()> {
    let payload = serde_json::to_string(message).map_err(|_| ())?;
    sender
        .send(Message::Text(payload.into()))
        .await
        .map_err(|_| ())
}
