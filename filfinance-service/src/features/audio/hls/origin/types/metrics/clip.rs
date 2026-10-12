//! Métriques des clips HLS progressifs.

// `Serialize` expose les metriques de publication au runtime et aux traces.
use serde::Serialize;

use crate::features::audio::hls::playlist::clip_playlist_url;

/// Resultat de publication initiale d'un clip HLS progressif.
#[derive(Debug, Clone, Serialize)]
pub struct HlsClipInitialPublish {
    /// Indique si l'origin HLS clip etait active.
    pub hls_enabled: bool,
    /// Identifiant stable du clip annonce ensuite par `audio_ready`.
    pub stream_id: String,
    /// Playlist publique du clip progressif.
    pub playlist_url: String,
    /// Nombre de segments intro disponibles au moment de l'annonce.
    pub segment_count: u64,
    /// Duree totale de la publication initiale.
    pub publish_duration_ms: u64,
    /// Duree passee dans `ffmpeg` pour l'intro.
    pub render_duration_ms: u64,
}

impl HlsClipInitialPublish {
    /// Construit un resultat neutre quand HLS est desactive.
    pub(in crate::features::audio::hls::origin) fn disabled(stream_id: impl Into<String>) -> Self {
        let stream_id = stream_id.into();
        Self {
            hls_enabled: false,
            playlist_url: clip_playlist_url(&stream_id),
            stream_id,
            segment_count: 0,
            publish_duration_ms: 0,
            render_duration_ms: 0,
        }
    }
}

/// Resultat de finalisation d'un clip apres ajout du corps de news.
#[derive(Debug, Clone, Serialize)]
pub struct HlsClipFinalizeMetrics {
    /// Indique si l'origin HLS clip etait active.
    pub hls_enabled: bool,
    /// Identifiant stable du clip finalise.
    pub stream_id: String,
    /// Playlist publique qui contient maintenant `#EXT-X-ENDLIST`.
    pub playlist_url: String,
    /// Nombre de segments ajoutes pour le corps.
    pub appended_segment_count: u64,
    /// Nombre total de segments dans le clip final.
    pub total_segment_count: u64,
    /// Duree totale de l'append/finalisation.
    pub publish_duration_ms: u64,
    /// Duree passee dans `ffmpeg` pour le corps.
    pub render_duration_ms: u64,
}

impl HlsClipFinalizeMetrics {
    /// Construit un resultat neutre quand HLS est desactive.
    pub(in crate::features::audio::hls::origin) fn disabled(stream_id: impl Into<String>) -> Self {
        let stream_id = stream_id.into();
        Self {
            hls_enabled: false,
            playlist_url: clip_playlist_url(&stream_id),
            stream_id,
            appended_segment_count: 0,
            total_segment_count: 0,
            publish_duration_ms: 0,
            render_duration_ms: 0,
        }
    }
}
