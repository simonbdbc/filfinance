//! Orchestration auth et autorisation des WebSockets.

use axum::extract::ws::WebSocket;
use tokio::time::{Duration, timeout};
use tracing::{info, warn};

use crate::{
    app_state::AppState,
    features::auth::{self, CurrentUser},
};

use super::{close::close_socket_with_policy, reader::read_websocket_auth_token};

/// Delai laisse au client pour envoyer son premier message d'auth WebSocket.
const WEBSOCKET_AUTH_TIMEOUT_SECS: u64 = 5;

/// Authentifie le socket avant d'envoyer la moindre donnee metier.
pub(in crate::features::live::websocket) async fn authenticate_socket(
    socket: &mut WebSocket,
    state: &AppState,
    stream_name: &'static str,
    require_moderator_role: bool,
) -> Result<CurrentUser, ()> {
    // Quand l'auth serveur est desactivee, on conserve le mode local historique sans handshake obligatoire.
    if !state.config.auth.enabled {
        return Ok(CurrentUser::development());
    }

    // Le client doit envoyer `{ type: "auth", access_token }` juste apres l'ouverture du WebSocket.
    let token = match timeout(
        Duration::from_secs(WEBSOCKET_AUTH_TIMEOUT_SECS),
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

    // La verification JWT produit l'identite et le role signes par Better Auth.
    let identity = match auth::authenticate_access_token(&state.config.auth, &token).await {
        Ok(identity) => identity,
        Err(error) => {
            warn!(%error, stream = stream_name, "websocket auth rejected invalid bearer token");
            close_socket_with_policy(socket, "invalid auth token").await;
            return Err(());
        }
    };

    // Un WebSocket peut etre la premiere surface touchee apres login; on resout donc le profil ici.
    let user = match auth::sync_current_user_profile(state, &identity).await {
        Ok(user) => user,
        Err(error) => {
            warn!(%error, stream = stream_name, "websocket auth failed to sync user profile");
            close_socket_with_policy(socket, "profile persistence failed").await;
            return Err(());
        }
    };

    // Le canal de trace peut declencher des runs de test et reste donc reserve aux moderators/admins.
    if require_moderator_role && !user.can_access_moderator_routes(&state.config.auth) {
        warn!(
            user_id = %user.id,
            user_role = %user.role,
            stream = stream_name,
            "websocket auth rejected insufficient role"
        );
        close_socket_with_policy(socket, "insufficient role").await;
        return Err(());
    }

    // Le log confirme l'identite sans jamais exposer le token.
    info!(
        user_id = %user.id,
        user_role = %user.role,
        user_has_email = user.email.is_some(),
        stream = stream_name,
        "websocket auth accepted"
    );

    Ok(user)
}
