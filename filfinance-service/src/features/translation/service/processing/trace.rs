//! Traces E2E du worker de traduction.

use crate::{app_state::AppState, features::news::domain::PersistedNewsEvent};

/// Trace la réception d'un événement par le worker de traduction.
pub(super) fn emit_received(state: &AppState, item: &PersistedNewsEvent) {
    state.emit_test_trace_for_event(
        item,
        "translation_received",
        "info",
        "Evenement recu par le worker de traduction.",
        Some(format!("source={} news_event_id={}", item.source, item.id)),
    );
}

/// Trace un item ignoré avant appel provider.
pub(super) fn emit_skipped(state: &AppState, item: &PersistedNewsEvent) {
    state.emit_test_trace_for_event(
        item,
        "translation_skipped",
        "warning",
        "La traduction a ete ignoree car le titre est vide apres normalisation.",
        None,
    );
}

/// Trace le fallback provider vers la traduction heuristique.
pub(super) fn emit_provider_fallback(state: &AppState, item: &PersistedNewsEvent, detail: String) {
    state.emit_test_trace_for_event(
        item,
        "translation_provider_fallback",
        "warning",
        "La traduction locale a echoue; fallback heuristique utilise.",
        Some(detail),
    );
}

/// Trace une traduction persistée avec succès.
pub(super) fn emit_persisted(state: &AppState, item: &PersistedNewsEvent, translation_id: i64) {
    state.emit_test_trace_for_event(
        item,
        "translation_persisted",
        "ok",
        "La traduction FR a ete persistee.",
        Some(format!("translation_id={translation_id}")),
    );
}

/// Trace un échec de persistence de traduction.
pub(super) fn emit_persist_failed(state: &AppState, item: &PersistedNewsEvent, detail: String) {
    state.emit_test_trace_for_event(
        item,
        "translation_persist_failed",
        "error",
        "La persistence de la traduction a echoue.",
        Some(detail),
    );
}
