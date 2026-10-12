//! Handler HTTP d'emission des tickets edge.

use axum::{
    Json,
    extract::{Extension, State},
};

use crate::{app_state::AppState, errors::AppError, features::auth::CurrentUser};

use super::signing::{EdgeLiveTicketResponse, create_live_ticket};

/// Genere un ticket court pour ouvrir le WebSocket metadata edge.
pub async fn create_edge_live_ticket(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
) -> Result<Json<EdgeLiveTicketResponse>, AppError> {
    // Le middleware listener a deja verifie la session et synchronise le profil applicatif.
    let ticket = create_live_ticket(&state.config.edge, &user)?;
    Ok(Json(ticket))
}
