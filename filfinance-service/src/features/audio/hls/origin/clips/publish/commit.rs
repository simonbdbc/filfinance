//! Commit mémoire des clips HLS après écriture disque réussie.

use crate::errors::AppError;

use super::super::super::{
    HlsOriginState,
    helpers::now_unix_ms,
    types::{HlsClip, HlsClipSegment},
};

impl HlsOriginState {
    /// Insère l'intro en mémoire uniquement après écriture des segments.
    pub(in crate::features::audio::hls::origin::clips::publish) fn commit_clip_intro(
        &self,
        stream_id: &str,
        clip_segments: Vec<HlsClipSegment>,
        durable_recovery: bool,
    ) -> Result<(), AppError> {
        if clip_segments.is_empty() {
            return Err(AppError::downstream(
                "hls clip intro did not produce any segment".to_owned(),
            ));
        }

        let mut inner = self.inner.lock().expect("hls origin mutex poisoned");
        if let Some(existing) = inner.clips.get(stream_id) {
            // Le meme stream peut etre reconsomme apres une coupure entre
            // l'effet HLS et le commit SQLite. Accepter seulement le prefixe
            // identique preserve l'idempotence sans masquer une collision de
            // `stream_id` vers un autre contenu.
            if clip_prefix_matches(existing, &clip_segments) {
                return Ok(());
            }
            return Err(AppError::downstream(format!(
                "hls clip stream conflicts with existing segments: {stream_id}"
            )));
        }

        inner.clips.insert(
            stream_id.to_owned(),
            HlsClip {
                stream_id: stream_id.to_owned(),
                target_duration_secs: self.clip_segment_target_duration_secs,
                segments: clip_segments,
                finalized: false,
                durable_recovery,
                updated_at_ms: now_unix_ms(),
            },
        );

        Ok(())
    }

    /// Ajoute le corps puis marque le clip comme définitif.
    pub(in crate::features::audio::hls::origin::clips::publish) fn commit_clip_body(
        &self,
        stream_id: &str,
        body_segments: Vec<HlsClipSegment>,
    ) -> Result<usize, AppError> {
        if body_segments.is_empty() {
            return Err(AppError::downstream(
                "hls clip body did not produce any segment".to_owned(),
            ));
        }

        let mut inner = self.inner.lock().expect("hls origin mutex poisoned");
        let clip = inner
            .clips
            .get_mut(stream_id)
            .ok_or_else(|| AppError::not_found(format!("hls clip not found: {stream_id}")))?;
        if clip.finalized {
            // La persistence de l'etat HLS precede le commit memoire. Un
            // second consumer fence peut donc retrouver le corps deja
            // finalise: le resultat est deja atteint, sans second clip.
            if clip_suffix_matches(clip, &body_segments) {
                return Ok(clip.segments.len());
            }
            return Err(AppError::downstream(format!(
                "hls finalized clip conflicts with body segments: {stream_id}"
            )));
        }

        clip.segments.extend(body_segments);
        clip.finalized = true;
        clip.updated_at_ms = now_unix_ms();
        Ok(clip.segments.len())
    }
}

fn clip_prefix_matches(existing: &HlsClip, candidate: &[HlsClipSegment]) -> bool {
    existing.segments.len() >= candidate.len()
        && existing
            .segments
            .iter()
            .zip(candidate)
            .all(clip_segment_matches)
}

fn clip_suffix_matches(existing: &HlsClip, candidate: &[HlsClipSegment]) -> bool {
    existing.segments.len() >= candidate.len()
        && existing.segments[existing.segments.len() - candidate.len()..]
            .iter()
            .zip(candidate)
            .all(clip_segment_matches)
}

fn clip_segment_matches(left: (&HlsClipSegment, &HlsClipSegment)) -> bool {
    let (existing, candidate) = left;
    existing.name == candidate.name
        && existing.file_path == candidate.file_path
        && existing.discontinuity_before == candidate.discontinuity_before
        && (existing.duration_secs - candidate.duration_secs).abs() < f64::EPSILON
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::config::HlsConfig;

    fn segment(index: usize, discontinuity_before: bool) -> HlsClipSegment {
        HlsClipSegment {
            name: format!("segment-{index:03}.ts"),
            duration_secs: 1.0,
            file_path: PathBuf::from(format!("/tmp/calendar-hls-test/segment-{index:03}.ts")),
            discontinuity_before,
        }
    }

    #[test]
    fn reserved_calendar_stream_commits_are_idempotent_for_all_audio_kinds() {
        // Ces commits sont executes apres les effets HLS mais avant l'ecriture
        // SQLite fence. Une reprise du meme Item/Alert/Summary doit donc
        // conserver un seul clip finalise au lieu d'echouer ou d'en ajouter un.
        let origin = HlsOriginState::from_config(&HlsConfig::disabled_for_tests());

        for stream_id in [
            "calendar-item-reserved-stream",
            "calendar-alert-reserved-stream",
            "calendar-summary-reserved-stream",
        ] {
            let intro = segment(0, false);
            origin
                .commit_clip_intro(stream_id, vec![intro.clone()], true)
                .expect("initial intro commit should succeed");
            origin
                .commit_clip_intro(stream_id, vec![intro], true)
                .expect("retrying the same intro must be idempotent");

            let body = segment(1, true);
            assert_eq!(
                origin
                    .commit_clip_body(stream_id, vec![body.clone()])
                    .expect("initial body commit should succeed"),
                2
            );
            assert_eq!(
                origin
                    .commit_clip_body(stream_id, vec![body])
                    .expect("retrying the finalized body must be idempotent"),
                2
            );
        }
    }
}
