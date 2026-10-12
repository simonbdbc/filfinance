//! Composition des routes HTTP et WebSocket exposees par les features.

// `cors` porte le middleware HTTP transverse, `routes` porte les groupes publics/listener/moderator.
mod cors;
mod routes;
mod slow_requests;

// Router compose les routes Axum exposees par les features.
use axum::Router;
// middleware applique le CORS global.
use axum::middleware;

use crate::app_state::AppState;

/// Compose toutes les routes HTTP et WebSocket du service.
pub fn router(state: AppState) -> Router<AppState> {
    // Le routeur final garde les chemins publics identiques a l'architecture precedente.
    routes::public_routes()
        .merge(routes::admin_routes(&state))
        .merge(routes::moderator_routes(&state))
        .merge(routes::listener_routes(&state))
        .layer(middleware::from_fn_with_state(
            state,
            slow_requests::log_slow_requests,
        ))
        .layer(middleware::from_fn(cors::add_cors_headers))
}
