//! Handlers HTTP du glossaire metier.

// `Json`, `Path`, `Query` et `State` sont les extracteurs Axum des endpoints CRUD.
use axum::{
    Json,
    extract::{Path, Query, State},
};

use crate::{
    app_state::AppState,
    errors::AppError,
    features::{
        audio::tts::normalization::normalize_narration_text_for_tts,
        translation::{
            domain::TranslationRequest,
            provider::TranslationProvider,
            service::{ReapplyTranslationsReport, reapply_current_day_translations},
        },
    },
    shared::text::normalize_whitespace,
};

use super::domain::{
    CreateGlossaryEntryRequest, GlossaryEntry, GlossaryListParams, GlossaryListQuery,
    GlossaryPreviewRequest, GlossaryPreviewResponse, GlossarySystemRule,
    UpdateGlossaryEntryRequest, glossary_system_rules,
};
use super::pipeline::{apply_display_glossary, apply_source_glossary, apply_spoken_glossary};

/// Reponse de liste du glossaire.
#[derive(Debug, serde::Serialize)]
pub(crate) struct GlossaryListResponse {
    /// Entrees retournees dans l'ordre d'application.
    pub items: Vec<GlossaryEntry>,
}

/// Reponse des regles systeme non editables.
#[derive(Debug, serde::Serialize)]
pub(crate) struct GlossarySystemRulesResponse {
    /// Regles integrees au normaliseur TTS.
    pub items: Vec<GlossarySystemRule>,
}

/// Reponse de retraitement des traductions apres changement de glossaire.
pub(crate) type GlossaryReapplyTranslationsResponse = ReapplyTranslationsReport;

/// Liste les entrees du glossaire avec filtres operateur.
pub(crate) async fn list_glossary_entries(
    State(state): State<AppState>,
    Query(query): Query<GlossaryListQuery>,
) -> Result<Json<GlossaryListResponse>, AppError> {
    let params = GlossaryListParams::from_query(query);
    let items = state.news.list_glossary_entries(&params).await?;
    Ok(Json(GlossaryListResponse { items }))
}

/// Liste les regles systeme TTS non editables.
pub(crate) async fn list_glossary_system_rules() -> Json<GlossarySystemRulesResponse> {
    Json(GlossarySystemRulesResponse {
        items: glossary_system_rules(),
    })
}

/// Produit un aperçu textuel du pipeline dictionnaire/TTS sans persistence.
pub(crate) async fn preview_glossary_text(
    State(state): State<AppState>,
    Json(payload): Json<GlossaryPreviewRequest>,
) -> Result<Json<GlossaryPreviewResponse>, AppError> {
    let mut warnings = Vec::new();
    let mut entries = state
        .news
        .list_glossary_entries(&GlossaryListParams::active_for_pipeline())
        .await?;

    if let Some(draft) = payload.draft_entry {
        let normalized = draft.normalize()?;
        if normalized.enabled {
            entries.push(GlossaryEntry::preview_from_normalized(normalized));
        } else {
            warnings.push("draft_entry is disabled and was not applied".to_owned());
        }
    }

    let input_text = normalize_whitespace(&payload.input_text);
    if input_text.is_empty() {
        return Err(AppError::Config("input_text cannot be empty".to_owned()));
    }

    let (display_text, source_hits) = match payload.input_stage.trim() {
        "display_fr" => (apply_display_glossary(&input_text, &entries), 0),
        "source_en" => {
            let source_application = apply_source_glossary(&input_text, &entries);
            let request = TranslationRequest {
                source_title: source_application.text.clone(),
                source_body: None,
                source_language: state.config.translation.source_language.clone(),
                target_language: state.config.translation.target_language.clone(),
            };
            let mut provider = TranslationProvider::from_config(&state.config.translation);
            match provider.translate(&request).await {
                Ok(translated) => (
                    normalize_whitespace(
                        &source_application
                            .restore_display_text(&translated.translated_title, &entries),
                    ),
                    source_application.hits,
                ),
                Err(error) => {
                    warnings.push(format!(
                        "translation provider unavailable; display_text uses source glossary only: {error}"
                    ));
                    (
                        normalize_whitespace(
                            &source_application
                                .restore_display_text(&source_application.text, &entries),
                        ),
                        source_application.hits,
                    )
                }
            }
        }
        _ => {
            return Err(AppError::Config(
                "input_stage must be source_en or display_fr".to_owned(),
            ));
        }
    };

    let (spoken_text, spoken_hits) = apply_spoken_glossary(&display_text, &entries);
    let normalized_tts_text = normalize_narration_text_for_tts(&spoken_text);

    Ok(Json(GlossaryPreviewResponse {
        display_text,
        spoken_text,
        normalized_tts_text,
        glossary_hits: source_hits + spoken_hits,
        warnings,
    }))
}

/// Cree une entree du glossaire.
pub(crate) async fn create_glossary_entry(
    State(state): State<AppState>,
    Json(payload): Json<CreateGlossaryEntryRequest>,
) -> Result<Json<GlossaryEntry>, AppError> {
    let normalized = payload.normalize()?;
    let entry = state.news.create_glossary_entry(&normalized).await?;
    state
        .translation_runtime
        .record_glossary_loaded(state.news.active_glossary_count().await?);
    Ok(Json(entry))
}

/// Met a jour partiellement une entree du glossaire.
pub(crate) async fn update_glossary_entry(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<UpdateGlossaryEntryRequest>,
) -> Result<Json<GlossaryEntry>, AppError> {
    let normalized = payload.normalize()?;
    let entry = state.news.update_glossary_entry(id, &normalized).await?;
    state
        .translation_runtime
        .record_glossary_loaded(state.news.active_glossary_count().await?);
    Ok(Json(entry))
}

/// Desactive une entree du glossaire sans supprimer son historique.
pub(crate) async fn delete_glossary_entry(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<GlossaryEntry>, AppError> {
    let entry = state.news.disable_glossary_entry(id).await?;
    state
        .translation_runtime
        .record_glossary_loaded(state.news.active_glossary_count().await?);
    Ok(Json(entry))
}

/// Retraduit les news du jour courant avec les regles actives du glossaire.
pub(crate) async fn reapply_glossary_translations(
    State(state): State<AppState>,
) -> Result<Json<GlossaryReapplyTranslationsResponse>, AppError> {
    let report = reapply_current_day_translations(&state).await?;
    Ok(Json(report))
}
