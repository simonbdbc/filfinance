//! Traces E2E émises par le worker TTS.

use crate::{app_state::AppState, features::live::dto::LiveNewsItem};

/// Trace l'admission d'une traduction FR dans le worker TTS.
pub(super) fn emit_tts_received_trace(
    state: &AppState,
    live_item: &LiveNewsItem,
    test_run_id: Option<&str>,
) {
    if let Some(run_id) = test_run_id {
        state.emit_test_trace(
            run_id,
            "tts_received",
            "info",
            "La traduction FR a ete recue par le worker TTS serveur.",
            Some(format!(
                "translation_id={} news_event_id={}",
                live_item.id, live_item.news_event_id
            )),
        );
    }
}

/// Trace un echec de synthese vocale du corps audio.
pub(super) fn emit_tts_failed_trace(state: &AppState, test_run_id: Option<&str>, detail: String) {
    if let Some(run_id) = test_run_id {
        state.emit_test_trace(
            run_id,
            "tts_failed",
            "error",
            "La synthese vocale serveur a echoue.",
            Some(detail),
        );
    }
}

/// Trace la generation du corps audio pour le clip HLS progressif.
pub(super) fn emit_tts_body_generated_trace(
    state: &AppState,
    live_item: &LiveNewsItem,
    stream_id: &str,
    total_bytes: usize,
    synthesis_duration_ms: u64,
    test_run_id: Option<&str>,
) {
    if let Some(run_id) = test_run_id {
        state.emit_test_trace(
            run_id,
            "tts_clip_body_generated",
            "ok",
            "Le corps audio serveur a ete genere pour le clip HLS.",
            Some(format!(
                "translation_id={} stream_id={} total_bytes={} synthesis_duration_ms={}",
                live_item.id, stream_id, total_bytes, synthesis_duration_ms
            )),
        );
    }
}
