//! Watermark chronologique du flux audio live TTS.

use std::cmp::Ordering as CmpOrdering;
use time::{Duration, OffsetDateTime, format_description::well_known::Rfc3339};

use crate::features::live::dto::LiveNewsItem;

use super::{
    super::watermark::{LiveAudioWatermark, compare_live_audio_watermarks},
    TtsRuntimeState,
};

impl TtsRuntimeState {
    /// Accepte une news dans le live audio seulement si elle ne recule pas la chronologie.
    pub fn try_accept_live_audio_item(&self, item: &LiveNewsItem) -> bool {
        self.try_accept_live_audio_item_at(item, OffsetDateTime::now_utc())
    }

    pub(crate) fn try_accept_live_audio_item_at(
        &self,
        item: &LiveNewsItem,
        now_utc: OffsetDateTime,
    ) -> bool {
        let candidate = LiveAudioWatermark::from_live_item(item);
        if live_audio_watermark_is_future(&candidate, now_utc) {
            return false;
        }

        let mut watermark = self
            .live_audio_watermark
            .lock()
            .expect("tts live audio watermark mutex poisoned");

        if let Some(current) = watermark.as_ref()
            && !live_audio_watermark_is_future(current, now_utc)
            && compare_live_audio_watermarks(&candidate, current) == CmpOrdering::Less
        {
            return false;
        }

        *watermark = Some(candidate);
        true
    }

    #[cfg(test)]
    pub(crate) fn force_live_audio_watermark_for_test(&self, timestamp: &str, translation_id: i64) {
        let mut watermark = self
            .live_audio_watermark
            .lock()
            .expect("tts live audio watermark mutex poisoned");
        *watermark = Some(LiveAudioWatermark {
            timestamp: timestamp.to_owned(),
            translation_id,
        });
    }
}

fn live_audio_watermark_is_future(watermark: &LiveAudioWatermark, now_utc: OffsetDateTime) -> bool {
    let Ok(timestamp) = OffsetDateTime::parse(&watermark.timestamp, &Rfc3339) else {
        return false;
    };

    timestamp > now_utc + Duration::minutes(2)
}
