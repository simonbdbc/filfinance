//! Publication non bloquante des evenements audio vers le Worker edge.

use super::publisher::EdgePublisher;

use tracing::{debug, warn};

use crate::{
    config::EdgeConfig,
    features::live::dto::{
        EdgeAudioFailedPayload, EdgeAudioFinalizedPayload, EdgeAudioReadyPayload,
        EdgeCalendarAlertReadyPayload, EdgeLiveMessage,
    },
};

/// Construit un identifiant monotone pour l'annonce d'un clip audio pret.
pub(crate) fn edge_event_id_for_audio_ready(translation_id: i64) -> String {
    format!("audio_ready:{:020}", translation_id.max(0))
}

/// Construit un identifiant monotone pour l'echec d'un clip audio.
pub(crate) fn edge_event_id_for_audio_failed(translation_id: Option<i64>) -> String {
    format!(
        "audio_failed:{:020}",
        translation_id.unwrap_or_default().max(0)
    )
}

/// Construit un identifiant monotone pour la finalisation d'un clip audio.
pub(crate) fn edge_event_id_for_audio_finalized(translation_id: i64) -> String {
    format!("audio_finalized:{:020}", translation_id.max(0))
}

/// Construit un identifiant stable pour l'annonce audio d'une alerte calendrier.
pub(crate) fn edge_event_id_for_calendar_alert(alert_id: i64) -> String {
    format!("calendar_alert_ready:{:020}", alert_id.max(0))
}

/// Publie `audio_ready` vers l'edge sans bloquer le worker TTS.
pub fn publish_audio_ready(config: &EdgeConfig, payload: EdgeAudioReadyPayload) {
    publish_audio_message_async(
        config,
        payload.event_id.clone(),
        EdgeLiveMessage::AudioReady { payload },
    );
}

/// Publie `audio_failed` vers l'edge sans bloquer le worker TTS.
pub fn publish_audio_failed(config: &EdgeConfig, payload: EdgeAudioFailedPayload) {
    publish_audio_message_async(
        config,
        payload.event_id.clone(),
        EdgeLiveMessage::AudioFailed { payload },
    );
}

/// Publie `audio_finalized` vers l'edge sans bloquer le worker TTS.
pub fn publish_audio_finalized(config: &EdgeConfig, payload: EdgeAudioFinalizedPayload) {
    publish_audio_message_async(
        config,
        payload.event_id.clone(),
        EdgeLiveMessage::AudioFinalized { payload },
    );
}

/// Publie `calendar_alert_ready` vers l'edge sans bloquer le scheduler d'alertes.
pub fn publish_calendar_alert_ready(config: &EdgeConfig, payload: EdgeCalendarAlertReadyPayload) {
    publish_audio_message_async(
        config,
        payload.event_id.clone(),
        EdgeLiveMessage::CalendarAlertReady { payload },
    );
}

/// Detache une publication audio edge pour que l'audio reste non critique.
fn publish_audio_message_async(config: &EdgeConfig, event_id: String, message: EdgeLiveMessage) {
    if !config.publish_enabled {
        debug!(%event_id, "edge audio metadata publish skipped because publisher is disabled");
        return;
    }

    let config = config.clone();
    crate::features::calendar_logs::event(
        "edge.publication_queued",
        serde_json::json!({"event_id":event_id, "message":message}),
    );
    tokio::spawn(crate::features::calendar_logs::carry(async move {
        let publisher = match EdgePublisher::from_config(&config) {
            Ok(publisher) => publisher,
            Err(error) => {
                warn!(%error, %event_id, "edge audio metadata publisher failed to initialize");
                return;
            }
        };

        match publisher.publish_with_retry(&message).await {
            Ok(()) => debug!(%event_id, "edge audio metadata message published"),
            Err(error) => warn!(%error, %event_id, "edge audio metadata publish failed"),
        }
    }));
}
