//! Publication d'une traduction prête vers le canal edge metadata.

// Les logs gardent le downstream edge observable sans interrompre le pipeline principal.
use tracing::{debug, warn};

use crate::{
    app_state::AppState,
    features::{
        calendar_day::public_calendar_day_summary_status,
        live::dto::{EdgeLiveMessage, EdgeSourceStatusPayload, LiveNewsItem},
        news::{domain::PersistedNewsEvent, status::public_source_statuses},
        translation::domain::PersistedNewsTranslation,
    },
    shared::time::now_rfc3339,
};

use super::super::{
    publisher::EdgePublisher,
    translation::{
        build_translated_news_item_message, edge_event_id_for_translation_id,
        is_publishable_translation,
    },
};

/// Publie l'etat public des sources vers l'edge et le fallback backend `/ws/news`.
pub(super) async fn publish_source_status(state: &AppState, publisher: Option<&EdgePublisher>) {
    let server_time = now_rfc3339();
    let event_id = format!("source_status:{server_time}");
    let message = EdgeLiveMessage::SourceStatus {
        payload: EdgeSourceStatusPayload {
            event_id: event_id.clone(),
            server_time,
            sources: public_source_statuses(state),
            calendar_day_summary: Some(public_calendar_day_summary_status(state).await),
        },
    };

    state.publish_live_metadata(message.clone());

    if let Some(publisher) = publisher {
        match publisher.publish_with_retry(&message).await {
            Ok(()) => debug!(%event_id, "edge source status message published"),
            Err(error) => warn!(%error, %event_id, "edge source status publish failed"),
        }
    }
}

/// Publie une traduction prête vers l'edge en reconstruisant le DTO live existant.
pub(super) async fn publish_translation(
    state: &AppState,
    publisher: Option<&EdgePublisher>,
    translation: PersistedNewsTranslation,
) {
    // Le canal edge live public ne doit recevoir que la langue cible prête à afficher.
    if !is_publishable_translation(&translation, &state.config.translation.target_language) {
        debug!(
            translation_id = translation.id,
            language = %translation.language,
            status = %translation.status,
            "edge metadata publisher skipped non publishable translation"
        );
        return;
    }

    // La source est relue pour enrichir `LiveNewsItem` avec URL, `news_level` et run de test.
    let source_event = load_source_event(state, translation.news_event_id).await;
    let live_item = LiveNewsItem::from_translation(
        translation,
        source_event.as_ref(),
        state.config.translation.max_body_chars,
    );
    let event_id = edge_event_id_for_translation_id(live_item.id);
    let message = build_translated_news_item_message(event_id.clone(), live_item);
    state.publish_live_metadata(message.clone());

    if let Some(publisher) = publisher {
        match publisher.publish_with_retry(&message).await {
            Ok(()) => debug!(%event_id, "edge metadata message published"),
            Err(error) => warn!(%error, %event_id, "edge metadata publish failed"),
        }
    }
}

/// Recharge l'événement source sans rendre le downstream edge critique pour le pipeline.
async fn load_source_event(state: &AppState, news_event_id: i64) -> Option<PersistedNewsEvent> {
    match state.news.get_news_by_id(news_event_id).await {
        Ok(source_event) => source_event,
        Err(error) => {
            // Un échec SQLite local dégrade seulement des metadata optionnelles du payload edge.
            warn!(
                %error,
                news_event_id,
                "failed to load source event for edge metadata publish"
            );
            None
        }
    }
}
