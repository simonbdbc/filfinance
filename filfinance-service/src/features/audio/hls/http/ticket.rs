//! Handler HTTP d'emission des tickets d'acces HLS.

use axum::{Json, extract::State};

use crate::{
    app_state::AppState,
    errors::AppError,
    features::audio::hls::access::{HlsAccessTicketResponse, create_hls_access_ticket},
};

/// Genere un ticket court pour lire les playlists et segments HLS publics.
pub async fn create_hls_access_ticket_handler(
    State(state): State<AppState>,
) -> Result<Json<HlsAccessTicketResponse>, AppError> {
    // Le middleware listener a deja verifie la session. Le ticket ne porte pas l'identite utilisateur
    // afin de rester partageable par bucket temporel et compatible avec le cache CDN.
    Ok(Json(create_hls_access_ticket(&state.config.hls)?))
}
