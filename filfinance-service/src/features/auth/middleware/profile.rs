//! Verification token hors HTTP et construction du contexte utilisateur.

use crate::{
    app_state::AppState,
    config::AuthConfig,
    errors::AppError,
    features::auth::{
        better_auth_jwt::verify_better_auth_jwt,
        domain::{AuthenticatedIdentity, CurrentUser},
    },
};

/// Verifie un JWT Better Auth brut pour les transports hors middleware HTTP.
pub async fn authenticate_access_token(
    config: &AuthConfig,
    token: &str,
) -> Result<AuthenticatedIdentity, AppError> {
    // Les WebSockets reutilisent exactement la meme verification cryptographique que les routes HTTP.
    verify_better_auth_jwt(config, token).await
}

/// Construit le contexte utilisateur applicatif depuis les claims verifies.
pub async fn sync_current_user_profile(
    state: &AppState,
    identity: &AuthenticatedIdentity,
) -> Result<CurrentUser, AppError> {
    // Better Auth porte identite et role; les preferences sont gerees par l'API Node/PostgreSQL.
    let _ = state;
    Ok(CurrentUser::from_identity(identity))
}
