//! Handlers HTTP exposes pour le debug et les premiers consommateurs du service.

// `Json`, `Path`, `Query` et `State` sont les extracteurs `axum` qui transforment la requete HTTP
// en types Rust deja parsees pour les handlers.
use axum::{
    Json,
    extract::{Path, Query, State},
};
// `Deserialize` parse les query strings HTTP dans des DTO locaux aux handlers.
use serde::Deserialize;
// On importe ici les erreurs, les types de reponse et l'etat partage necessaires aux handlers.
use crate::{
    app_state::AppState,
    errors::AppError,
    features::news::{
        domain::{NewsListParams, NewsListResponse, PersistedNewsEvent, SourceStatus},
        status::admin_source_statuses,
    },
};

/// Parametres de lecture exposes par `GET /api/news`.
#[derive(Debug, Deserialize)]
pub struct NewsQuery {
    /// Limite demandee par le client HTTP.
    pub limit: Option<i64>,
    /// Filtre de source optionnel.
    pub source: Option<String>,
    /// Filtre de type d'evenement optionnel.
    pub event_kind: Option<String>,
    /// Curseur de pagination optionnel.
    pub before_id: Option<i64>,
}

/// Expose la configuration runtime des sources declarees.
pub async fn list_sources(State(state): State<AppState>) -> Json<Vec<SourceStatus>> {
    Json(admin_source_statuses(&state))
}

/// Retourne l'historique recent avec un filtrage MVP par source et limite.
pub async fn list_news(
    State(state): State<AppState>,
    Query(query): Query<NewsQuery>,
) -> Result<Json<NewsListResponse>, AppError> {
    // On separe la query brute du repository params pour garder une frontiere nette:
    // l'API accepte un format souple, puis le modele interne normalise les bornes.
    let params = NewsListParams::new(query.limit, query.source, query.event_kind, query.before_id);
    // `await?` appelle le storage asynchrone et remonte proprement toute erreur `AppError`.
    let items = state.news.list_news(&params).await?;
    // `Ok(Json(...))` signale explicitement la voie nominale du `Result`.
    Ok(Json(items))
}

/// Retourne un evenement individuel par identifiant pour les debugs ponctuels.
pub async fn get_news(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<PersistedNewsEvent>, AppError> {
    // Le `match` force ici a traiter explicitement le cas "trouve" et le cas "absent".
    match state.news.get_news_by_id(id).await? {
        Some(item) => Ok(Json(item)),
        // On convertit l'absence metier en `404` applicatif, pas en `500`.
        None => Err(AppError::not_found(format!("news event {id}"))),
    }
}
