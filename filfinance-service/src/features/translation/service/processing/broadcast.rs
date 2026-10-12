//! Broadcast des traductions persistées vers les downstreams.

use crate::{
    app_state::AppState,
    features::{news::domain::PersistedNewsEvent, translation::domain::PersistedNewsTranslation},
};

/// Diffuse une traduction persistée quand le caller le demande.
pub(super) fn broadcast_translation_if_requested(
    state: &AppState,
    item: &PersistedNewsEvent,
    stored: &PersistedNewsTranslation,
    broadcast_after_persist: bool,
) {
    if !broadcast_after_persist {
        return;
    }

    match state.translated_broadcast_tx.send(stored.clone()) {
        Ok(subscriber_count) => {
            // Ce broadcast alimente Telegram, `/ws/news` et le worker TTS.
            state.emit_test_trace_for_event(
                item,
                "translation_broadcast_emitted",
                "ok",
                "La traduction a ete emise vers Telegram et le WebSocket user-facing.",
                Some(format!("subscriber_count={subscriber_count}")),
            );
        }
        Err(_) => {
            state.emit_test_trace_for_event(
                item,
                "translation_broadcast_emitted",
                "warning",
                "La traduction a ete produite sans consommateur actif.",
                None,
            );
        }
    }

    emit_server_audio_unavailable_trace_if_needed(state, item);
}

fn emit_server_audio_unavailable_trace_if_needed(state: &AppState, item: &PersistedNewsEvent) {
    if state.config.tts.enabled && state.config.hls.enabled {
        return;
    }

    state.emit_test_trace_for_event(
        item,
        "server_audio_unavailable",
        "warning",
        "Audio serveur indisponible: Direct Audio ecoute le WebSocket, mais aucun vocal ne sera genere.",
        Some(format!(
            "NEWS_TTS_ENABLED={} NEWS_HLS_ENABLED={}",
            state.config.tts.enabled, state.config.hls.enabled
        )),
    );
}

#[cfg(test)]
mod tests {
    use crate::features::{
        test_support::{app_state, seed_test_run_headline},
        translation::domain::NewsTranslationDraft,
    };

    use super::broadcast_translation_if_requested;

    #[tokio::test]
    async fn test_run_trace_warns_when_server_audio_is_disabled() {
        let state = app_state().await;
        let _translated_rx = state.translated_broadcast_tx.subscribe();
        let mut trace_rx = state.test_trace_tx.subscribe();
        let event = seed_test_run_headline(&state, "test_run", "audio smoke").await;
        let translation = state
            .news
            .upsert_translation(
                &event,
                &NewsTranslationDraft {
                    language: "fr".to_owned(),
                    status: "ready".to_owned(),
                    translated_title: "Smoke audio".to_owned(),
                    translated_body: None,
                    narration_text: "18:10. Smoke audio.".to_owned(),
                    translation_mode: "heuristic_fr".to_owned(),
                    error_message: None,
                },
            )
            .await
            .expect("translation should persist");

        broadcast_translation_if_requested(&state, &event, &translation, true);

        let first_trace = trace_rx
            .recv()
            .await
            .expect("first trace should be emitted");
        let second_trace = trace_rx
            .recv()
            .await
            .expect("second trace should be emitted");

        assert_eq!(first_trace.stage, "translation_broadcast_emitted");
        assert_eq!(second_trace.stage, "server_audio_unavailable");
        assert_eq!(second_trace.status, "warning");
        assert_eq!(
            second_trace.detail.as_deref(),
            Some("NEWS_TTS_ENABLED=false NEWS_HLS_ENABLED=false")
        );
    }
}
