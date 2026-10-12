//! Workflow de publication de l'intro HLS progressive.

use std::time::Instant;

use tracing::warn;

use crate::{
    app_state::AppState,
    features::{
        audio::{
            service::stream::{audio_start_payload_for_clip, next_audio_stream_id},
            tts::provider::TtsProvider,
        },
        live::dto::LiveNewsItem,
    },
};

use super::super::{ProgressiveClipContext, failure::publish_progressive_clip_failed};
use super::{synthesis::synthesize_intro_audio, trace::trace_intro_published};

/// Publie l'intro d'un clip HLS progressif, sans annoncer le clip avant sa finalisation.
pub(in crate::features::audio::service) async fn publish_progressive_clip_intro(
    state: &AppState,
    provider: &mut TtsProvider,
    live_item: &LiveNewsItem,
) -> Option<ProgressiveClipContext> {
    if !state.config.hls.enabled {
        // Sans origin HLS, le live audio evenementiel est desactive: aucun transport legacy ne prend
        // le relais.
        return None;
    }

    let clip_started_at = Instant::now();
    let stream_id = next_audio_stream_id();
    let synthesis_started_at = Instant::now();
    let intro_audio = synthesize_intro_audio(state, provider, live_item, &stream_id).await?;

    let start_payload =
        audio_start_payload_for_clip(live_item, &stream_id, &intro_audio, intro_audio.bytes.len());
    let initial_publish = match state
        .hls_origin
        .publish_clip_intro(&start_payload, &intro_audio.bytes)
        .await
    {
        Ok(initial_publish) if initial_publish.hls_enabled => initial_publish,
        Ok(_) => return None,
        Err(error) => {
            publish_progressive_clip_failed(
                state,
                Some(live_item.id),
                Some(stream_id),
                format!("intro hls failed: {error}"),
            );
            warn!(
                %error,
                translation_id = live_item.id,
                news_event_id = live_item.news_event_id,
                "failed to publish progressive hls clip intro"
            );
            return None;
        }
    };

    trace_intro_published(
        live_item,
        &initial_publish,
        clip_started_at,
        synthesis_started_at,
    );

    Some(ProgressiveClipContext::new(
        initial_publish.stream_id,
        initial_publish.playlist_url,
        clip_started_at,
        Instant::now(),
    ))
}
