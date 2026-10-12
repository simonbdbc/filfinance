//! Playlist HTTP du replay HLS du jour.

use crate::{errors::AppError, features::audio::hls::playlist::playlist_target_duration};

use super::super::super::{
    HlsOriginState,
    helpers::{current_local_day, replay_segments_for_day},
    types::HlsSegment,
};

impl HlsOriginState {
    /// Génère la playlist replay du jour au format HLS `EVENT`.
    pub(in crate::features::audio) fn replay_today_playlist(
        &self,
        start_translation_id: Option<i64>,
    ) -> Result<String, AppError> {
        // HLS désactivé donne un 404 cohérent avec le replay explicite.
        if !self.enabled {
            return Err(AppError::not_found("hls replay origin is disabled"));
        }

        // Le replay du jour est toujours calé sur le jour local courant.
        let local_day = current_local_day();
        // Le lock reste court: on clone les segments du jour avant de formatter le manifest.
        let (next_sequence, replay_segments) = {
            let inner = self.inner.lock().expect("hls origin mutex poisoned");
            let segments = replay_segments_for_day(&inner.segments, &local_day);
            (
                inner.next_sequence,
                replay_segments_from_translation(segments, start_translation_id),
            )
        };

        // Une playlist replay vide reste valide tant que la journée n'a pas encore de segment.
        let media_sequence = replay_segments
            .first()
            .map(|segment| segment.sequence)
            .unwrap_or(next_sequence);
        let target_duration = playlist_target_duration(
            self.replay_segment_target_duration_secs,
            replay_segments.iter().map(|segment| segment.duration_secs),
        );
        let mut playlist = String::new();
        playlist.push_str("#EXTM3U\n");
        playlist.push_str("#EXT-X-VERSION:3\n");
        playlist.push_str("#EXT-X-PLAYLIST-TYPE:EVENT\n");
        playlist.push_str(&format!("#EXT-X-TARGETDURATION:{target_duration}\n"));
        playlist.push_str(&format!("#EXT-X-MEDIA-SEQUENCE:{media_sequence}\n"));
        if !replay_segments.is_empty() {
            // Le replay du jour est un resume depuis minuit; ce marqueur evite que
            // les lecteurs HLS traitent la playlist EVENT comme un live au bord courant.
            playlist.push_str("#EXT-X-START:TIME-OFFSET=0,PRECISE=YES\n");
        }

        for segment in replay_segments {
            playlist.push_str(&format!("#EXTINF:{:.3},\n", segment.duration_secs));
            playlist.push_str(&format!(
                "/audio/replay/today/segments/{}/{}.ts\n",
                segment.local_day, segment.sequence
            ));
        }

        Ok(playlist)
    }
}

fn replay_segments_from_translation(
    mut segments: Vec<HlsSegment>,
    start_translation_id: Option<i64>,
) -> Vec<HlsSegment> {
    sort_replay_segments_for_playback(&mut segments);

    let Some(start_translation_id) = start_translation_id else {
        return segments;
    };

    // Si la premiere news du jour a produit plusieurs segments, on coupe au premier segment associe.
    // Si elle n'a pas d'audio mais qu'une news suivante en a, on prend le premier segment posterieur.
    // Les segments reconstruits depuis disque n'ont pas encore cette metadata: si l'index est
    // partiel avant le point trouve, on conserve la playlist complete pour ne pas demarrer trop tard
    // apres un redemarrage.
    let Some(start_index) = segments.iter().position(|segment| {
        segment
            .translation_id
            .is_some_and(|id| id >= start_translation_id)
    }) else {
        return segments;
    };

    if segments[..start_index]
        .iter()
        .any(|segment| segment.translation_id.is_none())
    {
        return segments;
    }

    segments.into_iter().skip(start_index).collect()
}

fn sort_replay_segments_for_playback(segments: &mut [HlsSegment]) {
    if segments.iter().all(|segment| segment.timestamp.is_some()) {
        segments.sort_by(|left, right| {
            left.timestamp
                .as_deref()
                .unwrap_or_default()
                .cmp(right.timestamp.as_deref().unwrap_or_default())
                .then_with(|| left.translation_id.cmp(&right.translation_id))
                .then_with(|| left.sequence.cmp(&right.sequence))
        });
        return;
    }

    segments.sort_by(|left, right| left.sequence.cmp(&right.sequence));
}
