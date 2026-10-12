//! Traitement d'un evenement news par le worker de traduction.

// `broadcast`, `persistence` et `trace` portent les effets secondaires de la traduction.
mod broadcast;
mod persistence;
mod trace;

use std::time::Instant;

use tracing::{error, info};

use crate::{
    app_state::AppState,
    features::{
        glossary::domain::{GlossaryEntry, GlossaryListParams},
        news::domain::PersistedNewsEvent,
        translation::provider::TranslationProvider,
    },
};

use super::build_translation_draft;
use broadcast::broadcast_translation_if_requested;
use persistence::persist_translation;

/// Traduit un evenement persiste, le stocke puis le diffuse sur le bus FR.
pub(super) async fn translate_and_store_item(
    state: &AppState,
    provider: &mut TranslationProvider,
    item: &PersistedNewsEvent,
    broadcast_after_persist: bool,
) -> bool {
    let glossary_entries = match state
        .news
        .list_glossary_entries(&GlossaryListParams::active_for_pipeline())
        .await
    {
        Ok(entries) => {
            state
                .translation_runtime
                .record_glossary_loaded(entries.len() as u64);
            state.translation_runtime.clear_glossary_error();
            entries
        }
        Err(error) => {
            state
                .translation_runtime
                .record_glossary_error(error.to_string());
            error!(%error, "failed to load active translation glossary entries");
            Vec::new()
        }
    };

    translate_and_store_item_with_glossary(
        state,
        provider,
        item,
        broadcast_after_persist,
        &glossary_entries,
    )
    .await
}

/// Traduit un evenement avec un glossaire deja charge, utile pour les traitements batch.
pub(crate) async fn translate_and_store_item_with_glossary(
    state: &AppState,
    provider: &mut TranslationProvider,
    item: &PersistedNewsEvent,
    broadcast_after_persist: bool,
    glossary_entries: &[GlossaryEntry],
) -> bool {
    state.translation_runtime.record_received();
    // Chaque item brut garde la meme signature/event_id que le reste du pipeline;
    // cela facilite les traces E2E entre persistence, Telegram, WS et TTS.
    trace::emit_received(state, item);

    let translation_started_at = Instant::now();
    let Some(outcome) =
        build_translation_draft(item, &state.config.translation, provider, &glossary_entries).await
    else {
        state.translation_runtime.record_skipped();
        trace::emit_skipped(state, item);
        return false;
    };

    state
        .translation_runtime
        .record_glossary_hits(outcome.glossary_hits);

    if let Some(detail) = outcome.fallback_detail.clone() {
        state.translation_runtime.record_fallback();
        trace::emit_provider_fallback(state, item, detail);
    }

    match persist_translation(state, item, &outcome).await {
        Ok(stored) => {
            let duration_ms = translation_started_at.elapsed().as_millis() as u64;
            state
                .translation_runtime
                .record_success(&stored.translation_mode, duration_ms);
            // La traduction est d'abord persistee, puis diffusee; la base reste la source de verite.
            trace::emit_persisted(state, item, stored.id);
            broadcast_translation_if_requested(state, item, &stored, broadcast_after_persist);

            info!(
                provider = provider.provider_name(),
                news_event_id = item.id,
                source = %item.source,
                translation_id = stored.id,
                translation_mode = %stored.translation_mode,
                fallback = outcome.fallback_detail.is_some(),
                duration_ms = duration_ms,
                "translated news item persisted"
            );
            true
        }
        Err(error) => {
            state.translation_runtime.record_failure(error.to_string());
            trace::emit_persist_failed(state, item, error.to_string());
            error!(
                %error,
                news_event_id = item.id,
                source = %item.source,
                "failed to persist translated news item"
            );
            false
        }
    }
}
