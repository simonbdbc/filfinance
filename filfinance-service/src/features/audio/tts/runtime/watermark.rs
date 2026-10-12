//! Watermark monotone du live audio TTS.

use std::cmp::Ordering as CmpOrdering;

// OffsetDateTime sert a comparer les watermarks RFC3339 du live audio.
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use crate::features::live::dto::LiveNewsItem;

/// Repere metier monotone qui decide si une news peut encore entrer dans l'audio live.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LiveAudioWatermark {
    /// Timestamp metier de la news, pas timestamp technique de generation audio.
    pub(super) timestamp: String,
    /// Id de traduction utilise comme tie-breaker stable pour deux news au meme timestamp.
    pub(super) translation_id: i64,
}

impl LiveAudioWatermark {
    /// Fabrique le repere depuis l'item live deja expose aux clients.
    pub(super) fn from_live_item(item: &LiveNewsItem) -> Self {
        Self {
            timestamp: item.timestamp.clone(),
            translation_id: item.id,
        }
    }
}

pub(super) fn compare_live_audio_watermarks(
    left: &LiveAudioWatermark,
    right: &LiveAudioWatermark,
) -> CmpOrdering {
    let timestamp_order = compare_live_audio_timestamps(&left.timestamp, &right.timestamp);
    if timestamp_order != CmpOrdering::Equal {
        return timestamp_order;
    }

    left.translation_id.cmp(&right.translation_id)
}

fn compare_live_audio_timestamps(left: &str, right: &str) -> CmpOrdering {
    // Les sources devraient produire du RFC3339, mais le fallback lexical garde un ordre stable
    // si un collecteur livre temporairement une valeur non parseable.
    let left_timestamp = OffsetDateTime::parse(left, &Rfc3339).ok();
    let right_timestamp = OffsetDateTime::parse(right, &Rfc3339).ok();
    match (left_timestamp, right_timestamp) {
        (Some(left_timestamp), Some(right_timestamp)) => left_timestamp
            .unix_timestamp_nanos()
            .cmp(&right_timestamp.unix_timestamp_nanos()),
        _ => left.cmp(right),
    }
}
