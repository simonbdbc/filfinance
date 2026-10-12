//! Types internes aux clips HLS progressifs.

use std::time::Instant;

/// Contexte d'un clip HLS progressif en cours de finalisation.
#[derive(Debug, Clone)]
pub(in crate::features::audio::service) struct ProgressiveClipContext {
    /// Identifiant stable du clip HLS fini par news.
    pub(in crate::features::audio::service) stream_id: String,
    /// Playlist publique annoncee au client apres finalisation.
    pub(in crate::features::audio::service::progressive) playlist_url: String,
    /// Instant local de demarrage du clip, utile pour mesurer le pipeline complet.
    pub(in crate::features::audio::service::progressive) started_at: Instant,
    /// Instant local de publication de l'intro, utile pour mesurer l'ecart intro -> corps.
    pub(in crate::features::audio::service::progressive) intro_ready_at: Instant,
}

impl ProgressiveClipContext {
    /// Construit le contexte opaque retourne au worker TTS.
    pub(in crate::features::audio::service::progressive) fn new(
        stream_id: String,
        playlist_url: String,
        started_at: Instant,
        intro_ready_at: Instant,
    ) -> Self {
        Self {
            stream_id,
            playlist_url,
            started_at,
            intro_ready_at,
        }
    }
}
