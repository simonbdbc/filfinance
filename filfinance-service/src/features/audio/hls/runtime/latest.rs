//! Dernieres mesures HLS volatiles exposees via l'API runtime operateur.

/// Dernieres mesures HLS volatiles exposees via l'API runtime operateur.
#[derive(Debug, Clone, Default)]
pub(in crate::features::audio::hls) struct HlsRuntimeLatest {
    /// Derniere duree totale de publication HLS.
    pub(in crate::features::audio::hls) last_publish_duration_ms: Option<u64>,
    /// Derniere duree passee dans `ffmpeg`.
    pub(in crate::features::audio::hls) last_render_duration_ms: Option<u64>,
    /// Nombre de segments du dernier stream publie.
    pub(in crate::features::audio::hls) last_segment_count: Option<u64>,
    /// Identifiant du dernier stream publie.
    pub(in crate::features::audio::hls) last_stream_id: Option<String>,
    /// Taille WAV du dernier stream recu par HLS.
    pub(in crate::features::audio::hls) last_total_bytes: Option<u64>,
    /// Horodatage RFC3339 de la derniere publication reussie.
    pub(in crate::features::audio::hls) last_success_at: Option<String>,
    /// Horodatage RFC3339 de la derniere erreur HLS.
    pub(in crate::features::audio::hls) last_error_at: Option<String>,
    /// Derniere erreur HLS conservee pour l'operateur.
    pub(in crate::features::audio::hls) last_error: Option<String>,
    /// Fenetre recente des durees totales de publication.
    pub(in crate::features::audio::hls) publish_duration_samples_ms: Vec<u64>,
    /// Fenetre recente des durees d'encodage `ffmpeg`.
    pub(in crate::features::audio::hls) render_duration_samples_ms: Vec<u64>,
}
