//! Handlers HTTP du viewer de logs admin.

// `Json`, `Query` et `State` adaptent la requete HTTP en DTO locaux.
use axum::{
    Json,
    extract::{Query, State},
};
// `Deserialize` parse la query string de pagination.
use serde::Deserialize;

use crate::{
    app_state::AppState,
    errors::AppError,
    features::logs::{
        domain::{AdminLogDaysResponse, AdminLogPage},
        service::{self, AdminLogReadParams},
    },
};

/// Query string de `GET /api/admin/logs`.
#[derive(Debug, Deserialize)]
pub struct AdminLogQuery {
    /// Jour UTC demande, au format `YYYY-MM-DD`.
    pub day: String,
    /// Limite de lignes avant bornage par `NEWS_LOG_ADMIN_MAX_LINES`.
    pub limit: Option<usize>,
    /// Curseur zero-based retourne par la page precedente.
    pub cursor: Option<usize>,
    /// Filtre de niveau optionnel: info, warn, error, debug ou trace.
    pub level: Option<String>,
    /// Recherche texte optionnelle.
    pub q: Option<String>,
}

/// Liste les fichiers de logs disponibles dans la retention J/J-1.
pub async fn list_log_days(
    State(state): State<AppState>,
) -> Result<Json<AdminLogDaysResponse>, AppError> {
    Ok(Json(service::list_log_days(&state.config.logging).await?))
}

/// Retourne une page de lignes de logs pour un jour conserve.
pub async fn list_logs(
    State(state): State<AppState>,
    Query(query): Query<AdminLogQuery>,
) -> Result<Json<AdminLogPage>, AppError> {
    let params = AdminLogReadParams {
        day: query.day,
        limit: query.limit,
        cursor: query.cursor,
        level: query.level,
        query: query.q,
    };

    Ok(Json(
        service::read_log_page(&state.config.logging, params).await?,
    ))
}
