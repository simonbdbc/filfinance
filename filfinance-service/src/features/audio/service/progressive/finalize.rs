//! Finalisation du corps audio d'un clip HLS progressif.

use tracing::{info, warn};

use crate::{
    app_state::AppState,
    features::live::{
        dto::{EdgeAudioFinalizedPayload, LiveNewsItem},
        edge::edge_event_id_for_audio_finalized,
    },
    shared::time::now_rfc3339,
};

use super::{
    ProgressiveClipContext,
    control_plane::{
        publish_audio_finalized_control_plane, publish_finalized_audio_ready_control_plane,
    },
    failure::publish_progressive_clip_failed,
};

/// Ajoute le corps audio au clip puis annonce seulement la playlist HLS finale au client.
pub(in crate::features::audio::service) async fn finalize_progressive_clip_body(
    state: &AppState,
    clip: &ProgressiveClipContext,
    body_wav_bytes: &[u8],
    live_item: &LiveNewsItem,
) {
    match state
        .hls_origin
        .append_clip_body_and_finalize(&clip.stream_id, body_wav_bytes)
        .await
    {
        Ok(metrics) if metrics.hls_enabled => {
            let finalized_at = now_rfc3339();
            publish_finalized_audio_ready_control_plane(
                state,
                live_item,
                &metrics.stream_id,
                &metrics.playlist_url,
            );
            publish_audio_finalized_control_plane(
                state,
                EdgeAudioFinalizedPayload {
                    event_id: edge_event_id_for_audio_finalized(live_item.id),
                    server_time: finalized_at.clone(),
                    translation_id: live_item.id,
                    stream_id: metrics.stream_id.clone(),
                    playlist_url: metrics.playlist_url.clone(),
                    finalized_at,
                },
            );

            let audio_ready_pipeline_duration_ms = clip.started_at.elapsed().as_millis() as u64;
            info!(
                translation_id = live_item.id,
                stream_id = %metrics.stream_id,
                appended_segment_count = metrics.appended_segment_count,
                total_segment_count = metrics.total_segment_count,
                publish_duration_ms = metrics.publish_duration_ms,
                render_duration_ms = metrics.render_duration_ms,
                intro_to_body_gap_ms = clip.intro_ready_at.elapsed().as_millis() as u64,
                audio_ready_pipeline_duration_ms,
                "progressive hls clip body finalized"
            );

            if let Some(run_id) = live_item.test_run_id.as_deref() {
                state.emit_test_trace(
                    run_id,
                    "audio_ready_published",
                    "ok",
                    "Le clip HLS live finalise est disponible et annonce au control plane.",
                    Some(format!(
                        "translation_id={} stream_id={} playlist_url={} segment_count={} audio_ready_pipeline_duration_ms={}",
                        live_item.id,
                        metrics.stream_id,
                        metrics.playlist_url,
                        metrics.total_segment_count,
                        audio_ready_pipeline_duration_ms
                    )),
                );
            }
        }
        Ok(_) => {}
        Err(error) => {
            publish_progressive_clip_failed(
                state,
                Some(live_item.id),
                Some(clip.stream_id.clone()),
                format!("body hls failed: {error}"),
            );
            warn!(
                %error,
                translation_id = live_item.id,
                stream_id = %clip.stream_id,
                playlist_url = %clip.playlist_url,
                "failed to finalize progressive hls clip body"
            );
        }
    }
}
