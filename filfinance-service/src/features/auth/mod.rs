//! Verification des JWT Better Auth et injection d'un contexte utilisateur HTTP.

// `domain` porte les types d'identite exposes aux handlers et aux WebSockets.
pub(crate) mod domain;
// `errors` reserve les erreurs propres a la feature auth si la taxonomie grossit.
pub(crate) mod errors;
// `middleware` porte les middlewares Axum et la synchronisation du profil local.
pub(crate) mod middleware;
// `websocket` applique la meme borne `exp` a tous les transports persistants authentifies.
mod websocket;
// `better_auth_jwt` isole la verification cryptographique et le cache JWKS Better Auth.
pub(crate) mod better_auth_jwt;
// Les tests auth communs restent separes des modules domaine/middleware.
#[cfg(test)]
mod tests;

// La facade expose uniquement le contexte utilisateur requis par les autres features.
pub(crate) use domain::CurrentUser;
// Les transports HTTP et WebSocket passent par ces fonctions publiques de feature.
pub(crate) use middleware::{
    authenticate_access_token, require_admin, require_listener, require_moderator,
    sync_current_user_profile,
};
pub(crate) use websocket::{
    access_token_expired_close_message, complete_before_access_token_expiration,
};
