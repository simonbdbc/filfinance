//! Traces E2E du pipeline d'ingestion.

use crate::{
    app_state::AppState,
    features::news::domain::{NewsEvent, PersistedNewsEvent},
};

/// Trace l'entrée d'un événement brut dans le hot path.
pub(super) fn emit_received(state: &AppState, event: &NewsEvent) {
    state.emit_test_trace_for_raw_ref(
        event.raw_payload_ref.as_deref(),
        "pipeline_received",
        "info",
        "Evenement recu par le pipeline d'ingestion.",
        Some(format!(
            "source={} signature={}",
            event.source, event.signature
        )),
    );
}

/// Trace un doublon rejeté par la déduplication mémoire.
pub(super) fn emit_memory_duplicate(state: &AppState, event: &NewsEvent) {
    state.emit_test_trace_for_raw_ref(
        event.raw_payload_ref.as_deref(),
        "pipeline_duplicate_dropped",
        "warning",
        "Evenement ignore par la deduplication memoire.",
        Some(format!("signature={}", event.signature)),
    );
}

/// Trace une insertion durable réussie.
pub(super) fn emit_persisted(state: &AppState, stored: &PersistedNewsEvent) {
    state.emit_test_trace_for_event(
        stored,
        "pipeline_persisted",
        "ok",
        "Evenement persiste en base.",
        Some(format!("news_event_id={}", stored.id)),
    );
}

/// Trace un doublon rejeté par l'idempotence durable.
pub(super) fn emit_storage_deduplicated(state: &AppState, event: &NewsEvent) {
    state.emit_test_trace_for_raw_ref(
        event.raw_payload_ref.as_deref(),
        "pipeline_storage_deduplicated",
        "warning",
        "Le stockage a ignore un doublon durable.",
        Some(format!("signature={}", event.signature)),
    );
}

/// Trace un échec de persistence durable.
pub(super) fn emit_persist_failed(state: &AppState, event: &NewsEvent, detail: String) {
    state.emit_test_trace_for_raw_ref(
        event.raw_payload_ref.as_deref(),
        "pipeline_persist_failed",
        "error",
        "La persistence de l'evenement a echoue.",
        Some(detail),
    );
}
