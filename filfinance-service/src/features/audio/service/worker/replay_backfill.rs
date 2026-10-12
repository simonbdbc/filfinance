//! Rattrapage audio replay pour les traductions deja persistees.

use std::{collections::HashSet, time::Instant};

use time::{Duration as TimeDuration, OffsetDateTime, format_description::well_known::Rfc3339};
use tracing::{info, warn};

use crate::{
    app_state::AppState,
    features::{
        audio::{
            hls::origin::{current_local_day, local_day_for_rfc3339},
            service::stream::{build_live_clip_body_text, prepare_audio_stream},
            tts::provider::TtsProvider,
        },
        live::dto::LiveNewsItem,
        news::domain::PersistedNewsEvent,
        translation::domain::{PersistedNewsTranslation, TranslationHistoryParams},
    },
};

const REPLAY_BACKFILL_LIMIT_PER_UTC_DAY: i64 = 500;
const REPLAY_BACKFILL_LIVE_GRACE_SECS: i64 = 90;

/// Genere les segments replay manquants sans rediffuser la news sur le flux live.
pub(super) async fn backfill_daily_replay_audio(state: &AppState, provider: &mut TtsProvider) {
    if !state.config.hls.enabled {
        return;
    }

    let current_day = current_local_day();
    let replay_items = match load_current_day_replay_items(state, &current_day).await {
        Ok(items) => items,
        Err(error) => {
            warn!(%error, "failed to load translations for hls replay backfill");
            return;
        }
    };

    if replay_items.is_empty() {
        return;
    }

    if state.hls_origin.replay_today_has_unmapped_segments() {
        info!(
            local_day = %current_day,
            "hls replay contains legacy segments without metadata; rebuilding current day replay"
        );
        state.hls_origin.reset_replay_today_for_rebuild().await;
    }

    let mut covered_translation_ids = state.hls_origin.replay_today_translation_ids();
    let mut backfilled_count = 0usize;

    for replay_item in replay_items {
        if covered_translation_ids.contains(&replay_item.live_item.id) {
            continue;
        }

        if !is_outside_live_audio_grace(&replay_item.live_item) {
            continue;
        }

        if backfill_replay_item(state, provider, &replay_item.live_item).await {
            covered_translation_ids.insert(replay_item.live_item.id);
            backfilled_count += 1;
        }
    }

    if backfilled_count > 0 {
        info!(
            local_day = %current_day,
            backfilled_count,
            "hls replay audio backfill completed"
        );
    }
}

async fn backfill_replay_item(
    state: &AppState,
    provider: &mut TtsProvider,
    live_item: &LiveNewsItem,
) -> bool {
    let tts_received_at = Instant::now();
    let audio_text = build_live_clip_body_text(live_item);
    let prepared_stream = match prepare_audio_stream(state, provider, live_item, &audio_text).await
    {
        Ok(Some(prepared_stream)) => prepared_stream,
        Ok(None) => {
            state.tts_runtime.record_skipped();
            return false;
        }
        Err(error) => {
            warn!(
                %error,
                translation_id = live_item.id,
                news_event_id = live_item.news_event_id,
                "failed to synthesize hls replay backfill item"
            );
            return false;
        }
    };

    let metrics = match state
        .hls_origin
        .publish_replay_stream(&prepared_stream.start_payload, &prepared_stream.audio_bytes)
        .await
    {
        Ok(metrics) => metrics,
        Err(error) => {
            warn!(
                %error,
                translation_id = live_item.id,
                news_event_id = live_item.news_event_id,
                stream_id = %prepared_stream.start_payload.stream_id,
                "failed to publish hls replay backfill item"
            );
            return false;
        }
    };

    if !metrics.hls_enabled || metrics.segment_count == 0 {
        return false;
    }

    info!(
        translation_id = live_item.id,
        news_event_id = live_item.news_event_id,
        stream_id = %prepared_stream.start_payload.stream_id,
        publish_duration_ms = metrics.publish_duration_ms,
        render_duration_ms = metrics.render_duration_ms,
        segment_count = metrics.segment_count,
        tts_to_playlist_ms = tts_received_at.elapsed().as_millis() as u64,
        "hls replay backfill item published"
    );
    true
}

async fn load_current_day_replay_items(
    state: &AppState,
    current_day: &str,
) -> Result<Vec<ReplayBackfillItem>, crate::errors::AppError> {
    let mut seen_translation_ids = HashSet::new();
    let mut items = Vec::new();

    for utc_day in recent_utc_day_keys() {
        let params = TranslationHistoryParams::new(
            Some(REPLAY_BACKFILL_LIMIT_PER_UTC_DAY),
            None,
            Some(utc_day),
            Some("real_feed".to_owned()),
            None,
        );
        let page = state
            .news
            .list_translation_history(&params, &state.config.translation.target_language)
            .await?;

        for translation in page.items {
            if !seen_translation_ids.insert(translation.id) {
                continue;
            }

            let source_event = state.news.get_news_by_id(translation.news_event_id).await?;
            let live_item = live_item_from_translation(state, translation, source_event.as_ref());
            if local_day_for_rfc3339(&live_item.timestamp).as_deref() == Some(current_day) {
                items.push(ReplayBackfillItem { live_item });
            }
        }
    }

    items.sort_by(|left, right| {
        left.live_item
            .timestamp
            .cmp(&right.live_item.timestamp)
            .then_with(|| left.live_item.id.cmp(&right.live_item.id))
    });
    Ok(items)
}

fn live_item_from_translation(
    state: &AppState,
    translation: PersistedNewsTranslation,
    source_event: Option<&PersistedNewsEvent>,
) -> LiveNewsItem {
    LiveNewsItem::from_translation(
        translation,
        source_event,
        state.config.translation.max_body_chars,
    )
}

fn recent_utc_day_keys() -> Vec<String> {
    let today = OffsetDateTime::now_utc().date();
    let yesterday = today.previous_day().unwrap_or(today);

    vec![today.to_string(), yesterday.to_string()]
}

fn is_outside_live_audio_grace(live_item: &LiveNewsItem) -> bool {
    let Ok(updated_at) = OffsetDateTime::parse(&live_item.updated_at, &Rfc3339) else {
        return true;
    };

    updated_at <= OffsetDateTime::now_utc() - TimeDuration::seconds(REPLAY_BACKFILL_LIVE_GRACE_SECS)
}

struct ReplayBackfillItem {
    live_item: LiveNewsItem,
}
