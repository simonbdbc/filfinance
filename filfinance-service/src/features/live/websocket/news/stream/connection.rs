//! Boucle principale WebSocket user-facing des news traduites.

// `WebSocket` expose la socket Axum apres upgrade HTTP.
use axum::extract::ws::WebSocket;
// `StreamExt` lit les messages entrants et `SinkExt` ferme explicitement a l'expiration JWT.
use futures_util::{SinkExt, StreamExt};
// `interval` fabrique un ticker async pour le heartbeat applicatif.
use tokio::time::{Duration, MissedTickBehavior, interval};

use crate::app_state::AppState;

use super::super::super::{HEARTBEAT_INTERVAL_SECS, authenticate_socket};
use super::super::delivery::replay_recent_translations;
use super::{
    broadcasts::{handle_live_metadata_broadcast, handle_translated_broadcast},
    heartbeat::{send_heartbeat, send_hello},
    input::handle_client_message,
};

/// Pousse les evenements EN+FR vers un client user-facing jusqu'a fermeture ou lag irrecuperable.
pub(in crate::features::live::websocket) async fn handle_news_socket(
    mut socket: WebSocket,
    state: AppState,
) {
    // Aucun `hello`, replay ou item live ne part tant que le token n'est pas valide.
    let user = match authenticate_socket(&mut socket, &state, "news", false).await {
        Ok(user) => user,
        Err(()) => return,
    };
    let access_token_expires_at = user.access_token_expires_at;

    // `split()` separe lecture et ecriture pour pouvoir lire le client et pousser le flux en parallele.
    let (mut sender, mut receiver) = socket.split();
    // Chaque client recoit son propre curseur `broadcast`; un abonne lent n'arrete pas les autres.
    let mut translated_rx = state.translated_broadcast_tx.subscribe();
    // Le fallback backend transporte aussi les metadata audio `audio_ready` quand l'edge n'est pas configure.
    let mut live_metadata_rx = state.live_metadata_tx.subscribe();
    // Le heartbeat applicatif garde les proxys et navigateurs mobiles reveilles meme sans nouvelle news.
    let mut heartbeat_ticker = interval(Duration::from_secs(HEARTBEAT_INTERVAL_SECS));
    heartbeat_ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

    let completed = crate::features::auth::complete_before_access_token_expiration(
        access_token_expires_at,
        async {
            if send_hello(&mut sender, &state).await.is_err() {
                return;
            }

            if replay_recent_translations(&mut sender, &state)
                .await
                .is_err()
            {
                return;
            }

            // `interval` emet un premier tick immediat par design; on le consomme pour attendre le vrai delai.
            heartbeat_ticker.tick().await;

            loop {
                // `tokio::select!` multiplexe ici quatre sources:
                // 1. ce que le client nous envoie,
                // 2. le heartbeat periodique,
                // 3. le flux broadcast des traductions,
                // 4. le control plane live fallback compatible edge.
                tokio::select! {
                    client_message = receiver.next() => {
                        if !handle_client_message(&mut sender, client_message).await {
                            break;
                        }
                    }
                    _ = heartbeat_ticker.tick() => {
                        if send_heartbeat(&mut sender, &state).await.is_err() {
                            break;
                        }
                    }
                    translated_message = translated_rx.recv() => {
                        if !handle_translated_broadcast(&mut sender, &state, translated_message).await {
                            break;
                        }
                    }
                    live_metadata_message = live_metadata_rx.recv() => {
                        if !handle_live_metadata_broadcast(&mut sender, live_metadata_message).await {
                            break;
                        }
                    }
                }
            }
        },
    )
    .await;

    if completed.is_none() {
        let _ = sender
            .send(crate::features::auth::access_token_expired_close_message())
            .await;
    }
}
