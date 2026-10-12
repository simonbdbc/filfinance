//! Commande d'authentification WebSocket.

use serde::Deserialize;

/// Message d'authentification attendu en premier sur les WebSockets proteges.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientAuthCommand {
    // Le token est envoye dans le corps car les navigateurs ne permettent pas toujours un header WS custom.
    Auth { access_token: String },
}

/// Parse le JSON d'auth WebSocket et retourne un token nettoye.
pub(in crate::features::live) fn parse_websocket_auth_token(
    payload: &str,
) -> Result<String, &'static str> {
    // Le payload texte doit etre un JSON taggue `type=auth`.
    let command =
        serde_json::from_str::<ClientAuthCommand>(payload).map_err(|_| "invalid auth payload")?;
    match command {
        ClientAuthCommand::Auth { access_token } => {
            // Un token vide n'est jamais exploitable pour Better Auth.
            let token = access_token.trim();
            if token.is_empty() {
                return Err("empty auth token");
            }
            Ok(token.to_owned())
        }
    }
}
