//! Admission monotone des traductions dans le live audio.

// Les logs rendent visibles les skips de watermark sans arrêter le worker.
use tracing::info;

use crate::{app_state::AppState, features::live::dto::LiveNewsItem};

/// Tente d'admettre un item dans le flux audio live.
pub(super) fn accept_live_audio_item(state: &AppState, live_item: &LiveNewsItem) -> bool {
    if state.tts_runtime.try_accept_live_audio_item(live_item) {
        return true;
    }

    // La watermark vit au niveau worker, avant HLS, pour que le live audio ne puisse jamais
    // rejouer une news plus ancienne qu'une news déjà admise.
    state.tts_runtime.record_skipped();
    info!(
        translation_id = live_item.id,
        news_event_id = live_item.news_event_id,
        timestamp = %live_item.timestamp,
        "tts live audio skipped by monotone watermark"
    );
    false
}
