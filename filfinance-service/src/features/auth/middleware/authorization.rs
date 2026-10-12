//! Middlewares Axum d'autorisation HTTP.

// Les extracteurs et types `axum` permettent d'ecrire un middleware compatible `route_layer`.
use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};

use crate::{
    app_state::AppState,
    errors::AppError,
    features::auth::{better_auth_jwt::verify_better_auth_jwt, domain::CurrentUser},
};

use super::{bearer::bearer_token_from_headers, profile::sync_current_user_profile};

/// Niveau d'autorisation applicative attendu par une route HTTP.
enum RequiredAccess {
    Listener,
    Moderator,
    Admin,
}

/// Middleware pour les routes qui exigent seulement une session utilisateur valide.
pub async fn require_listener(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    // Le listener est le niveau minimal: un JWT valide cree ou relit un profil local `listener`.
    authorize_request(&state, &mut request, RequiredAccess::Listener).await?;
    // Une fois le contexte insere, on laisse le handler final traiter la requete.
    Ok(next.run(request).await)
}

/// Middleware pour les routes reservees aux roles moderateur.
pub async fn require_moderator(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    // Les endpoints runtime/debug peuvent modifier l'etat observe par l'utilisateur et exigent un role fort.
    authorize_request(&state, &mut request, RequiredAccess::Moderator).await?;
    // Le handler peut ensuite recuperer `CurrentUser` depuis les extensions si besoin.
    Ok(next.run(request).await)
}

/// Middleware pour les routes reservees au role admin.
pub async fn require_admin(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    // Les surfaces admin sensibles exigent explicitement le role applicatif `admin`.
    authorize_request(&state, &mut request, RequiredAccess::Admin).await?;
    Ok(next.run(request).await)
}

/// Authentifie la requete, applique l'autorisation demandee et stocke `CurrentUser`.
async fn authorize_request(
    state: &AppState,
    request: &mut Request,
    required_access: RequiredAccess,
) -> Result<(), AppError> {
    // Le flag de config conserve un mode local ouvert pour les developpements hors production.
    let user = if state.config.auth.enabled {
        // On copie le token hors des headers avant le `await` pour eviter d'emprunter la requete.
        let token = bearer_token_from_headers(request.headers())?.to_owned();
        // La verification couvre signature, expiration, issuer, audience et subject.
        let identity = verify_better_auth_jwt(&state.config.auth, &token).await?;
        // Le role est signe par Better Auth et le profil metier reste separe.
        sync_current_user_profile(state, &identity).await?
    } else {
        // En dev sans auth, on injecte quand meme un contexte coherent pour les futurs handlers.
        CurrentUser::development()
    };

    match required_access {
        RequiredAccess::Listener => {}
        RequiredAccess::Moderator => {
            // Un token valide mais sans role moderateur ne doit pas acceder aux surfaces d'exploitation.
            if !user.can_access_moderator_routes(&state.config.auth) {
                return Err(AppError::forbidden(
                    "authenticated user does not have a moderator role",
                ));
            }
        }
        RequiredAccess::Admin => {
            // Le role admin vient du JWT Better Auth signe.
            if !user.can_access_admin_routes() {
                return Err(AppError::forbidden(
                    "authenticated user does not have an admin role",
                ));
            }
        }
    }

    // Le log debug lit le contexte sans exposer le JWT; il aide a verifier l'auth en dev.
    tracing::debug!(
        user_id = %user.id,
        user_role = %user.role,
        user_has_email = user.email.is_some(),
        "authenticated HTTP request authorized"
    );

    // `extensions_mut` est la poche type-safe d'`axum` pour transporter du contexte jusqu'au handler.
    request.extensions_mut().insert(user);
    Ok(())
}
