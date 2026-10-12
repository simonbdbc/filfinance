//! Commandes acceptées par le WebSocket de trace E2E.

use axum::extract::ws::{Message, WebSocket};
use futures_util::stream::SplitSink;
use serde::Deserialize;

use crate::{app_state::AppState, features::live::dto::WsTestTraceMessage};

use super::super::{send_text, serialize_test_trace_message};
use super::trigger::trigger_test_run;

/// Commande minimale acceptee depuis le client de debug pour lancer un run de test.
#[derive(Debug, Deserialize)]
// `serde(tag = "type")` dit que le champ JSON `type` selectionne la variante de l'enum.
// C'est plus robuste qu'un `match` manuel sur des chaines dans chaque handler.
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientTestTraceCommand {
    // La commande `auth` est ignoree si elle arrive apres le handshake, pour tolerer un client reconnecte.
    Auth { access_token: String },
    // Cette commande declenche volontairement un evenement synthetique dans le pipeline canonique.
    TriggerTestRun,
}

/// Traite une commande de test reçue depuis le socket de debug sans quitter le hot path principal.
pub(super) async fn handle_test_trace_command(
    sender: &mut SplitSink<WebSocket, Message>,
    state: &AppState,
    payload: &str,
) -> Result<(), ()> {
    // Le socket debug suit le meme principe que le socket audio: parser d'abord, agir ensuite.
    let command = match serde_json::from_str::<ClientTestTraceCommand>(payload) {
        Ok(command) => command,
        Err(error) => {
            if let Some(error_payload) = serialize_test_trace_message(WsTestTraceMessage::error(
                format!("Commande de debug invalide: {error}"),
            )) {
                let _ = send_text(sender, error_payload).await;
            }
            return Ok(());
        }
    };

    match command {
        ClientTestTraceCommand::Auth { access_token } => {
            // Le vrai handshake est deja passe; ce cas garde le protocole tolerant aux doublons client.
            let _ = access_token.len();
        }
        ClientTestTraceCommand::TriggerTestRun => {
            trigger_test_run(sender, state).await?;
        }
    }

    Ok(())
}
