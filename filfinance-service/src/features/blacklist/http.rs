//! Handlers HTTP blacklist news et audits associes.

use axum::{
    Json,
    extract::{Path, Query, State},
};

use crate::{app_state::AppState, errors::AppError};

use super::domain::{
    CreateNewsBlacklistEntryRequest, NewsAuditListParams, NewsAuditListQuery, NewsBlacklistEntry,
    NewsBlacklistHit, NewsBlacklistListParams, NewsBlacklistListQuery, NewsTruncationAudit,
    UpdateNewsBlacklistEntryRequest,
};
use super::reapply::{NewsBlacklistReapplyReport, reapply_current_day_blacklist};

/// Reponse de liste des regles blacklist.
#[derive(Debug, serde::Serialize)]
pub(crate) struct NewsBlacklistListResponse {
    /// Regles retournees dans l'ordre d'application.
    pub items: Vec<NewsBlacklistEntry>,
}

/// Reponse de liste des hits blacklist.
#[derive(Debug, serde::Serialize)]
pub(crate) struct NewsBlacklistHitsResponse {
    /// Dernieres actions operateur.
    pub items: Vec<NewsBlacklistHit>,
}

/// Reponse de liste des troncatures upstream detectees.
#[derive(Debug, serde::Serialize)]
pub(crate) struct NewsTruncationAuditsResponse {
    /// Derniers audits de troncature upstream.
    pub items: Vec<NewsTruncationAudit>,
}

/// Applique les regles blacklist actives aux news du jour deja stockees.
pub(crate) async fn reapply_news_blacklist(
    State(state): State<AppState>,
) -> Result<Json<NewsBlacklistReapplyReport>, AppError> {
    let report = reapply_current_day_blacklist(&state).await?;
    Ok(Json(report))
}

/// Liste les regles blacklist avec filtres operateur.
pub(crate) async fn list_news_blacklist_entries(
    State(state): State<AppState>,
    Query(query): Query<NewsBlacklistListQuery>,
) -> Result<Json<NewsBlacklistListResponse>, AppError> {
    let params = NewsBlacklistListParams::from_query(query);
    let items = state.news.list_news_blacklist_entries(&params).await?;
    Ok(Json(NewsBlacklistListResponse { items }))
}

/// Cree une regle blacklist.
pub(crate) async fn create_news_blacklist_entry(
    State(state): State<AppState>,
    Json(payload): Json<CreateNewsBlacklistEntryRequest>,
) -> Result<Json<NewsBlacklistEntry>, AppError> {
    let normalized = payload.normalize()?;
    let entry = state.news.create_news_blacklist_entry(&normalized).await?;
    Ok(Json(entry))
}

/// Met a jour partiellement une regle blacklist.
pub(crate) async fn update_news_blacklist_entry(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<UpdateNewsBlacklistEntryRequest>,
) -> Result<Json<NewsBlacklistEntry>, AppError> {
    let normalized = payload.normalize()?;
    let entry = state
        .news
        .update_news_blacklist_entry(id, &normalized)
        .await?;
    Ok(Json(entry))
}

/// Desactive une regle blacklist sans supprimer l'historique.
pub(crate) async fn delete_news_blacklist_entry(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<NewsBlacklistEntry>, AppError> {
    let entry = state.news.disable_news_blacklist_entry(id).await?;
    Ok(Json(entry))
}

/// Liste les derniers rejets blacklist.
pub(crate) async fn list_news_blacklist_hits(
    State(state): State<AppState>,
    Query(query): Query<NewsAuditListQuery>,
) -> Result<Json<NewsBlacklistHitsResponse>, AppError> {
    let params = NewsAuditListParams::from_query(query);
    let items = state.news.list_news_blacklist_hits(&params).await?;
    Ok(Json(NewsBlacklistHitsResponse { items }))
}

/// Liste les troncatures upstream detectees.
pub(crate) async fn list_news_truncation_audits(
    State(state): State<AppState>,
    Query(query): Query<NewsAuditListQuery>,
) -> Result<Json<NewsTruncationAuditsResponse>, AppError> {
    let params = NewsAuditListParams::from_query(query);
    let items = state.news.list_news_truncation_audits(&params).await?;
    Ok(Json(NewsTruncationAuditsResponse { items }))
}
