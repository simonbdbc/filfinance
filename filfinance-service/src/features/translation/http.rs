//! Handlers HTTP de la feature traduction.

// `Json`, `Query` et `State` sont les extracteurs Axum des handlers traduction.
use axum::{
    Json,
    extract::{Path, Query, State},
};
// `Deserialize` parse la query string de l'historique traduit.
use serde::Deserialize;
// `json!` evite un DTO dedie pour ce payload d'observabilite.
use serde_json::json;

use crate::{
    app_state::AppState,
    errors::AppError,
    features::{
        live::dto::{LiveNewsHistoryResponse, LiveNewsItem},
        translation::domain::TranslationHistoryParams,
    },
};

/// Parametres de lecture exposes par `GET /api/translations`.
#[derive(Debug, Deserialize)]
pub struct TranslationHistoryQuery {
    /// Limite demandee par le client.
    pub limit: Option<i64>,
    /// Source logique optionnelle.
    pub source: Option<String>,
    /// Jour cible au format `YYYY-MM-DD`.
    pub day: Option<String>,
    /// Scope fonctionnel: `all`, `real_feed` ou `test_run`.
    pub history_scope: Option<String>,
    /// Curseur de pagination optionnel.
    pub before_id: Option<i64>,
}

/// Expose une vue synthetique des metriques runtime de traduction.
pub async fn translation_runtime(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(json!({
        "translation": state.translation_runtime.snapshot()
    }))
}

/// Retourne l'historique traduit EN+FR avec separation feed reel / test runs.
pub async fn list_translations_history(
    State(state): State<AppState>,
    Query(query): Query<TranslationHistoryQuery>,
) -> Result<Json<LiveNewsHistoryResponse>, AppError> {
    // Le client veut ici des `LiveNewsItem`, pas les lignes SQL brutes du store news.
    let params = TranslationHistoryParams::new(
        query.limit,
        query.source,
        query.day,
        query.history_scope,
        query.before_id,
    );
    let page = state
        .news
        .list_translation_history(&params, &state.config.translation.target_language)
        .await?;
    let mut items: Vec<LiveNewsItem> = Vec::with_capacity(page.items.len());

    for translation in page.items {
        // L'historique HTTP sert de diagnostic et conserve les corps complets.
        let source_event = state.news.get_news_by_id(translation.news_event_id).await?;
        items.push(LiveNewsItem::from_translation_full_text(
            translation,
            source_event.as_ref(),
        ));
    }

    Ok(Json(LiveNewsHistoryResponse::new(
        items,
        page.has_more,
        page.next_before_id,
    )))
}

/// Retourne le detail complet d'une traduction prête a partir de son identifiant.
pub async fn get_translation_detail(
    State(state): State<AppState>,
    Path(translation_id): Path<i64>,
) -> Result<Json<LiveNewsItem>, AppError> {
    let translation = state
        .news
        .get_ready_translation_by_id_and_language(
            translation_id,
            &state.config.translation.target_language,
        )
        .await?
        .ok_or_else(|| AppError::not_found(format!("translation {translation_id} not found")))?;
    let source_event = state.news.get_news_by_id(translation.news_event_id).await?;

    Ok(Json(LiveNewsItem::from_translation_full_text(
        translation,
        source_event.as_ref(),
    )))
}
