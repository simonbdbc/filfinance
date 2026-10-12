//! Métriques replay HLS.

// `Serialize` expose les metadata replay et les metriques de publication.
use serde::Serialize;

/// Mesures d'une publication HLS replay retournees au worker TTS pour les traces E2E.
#[derive(Debug, Clone, Serialize)]
pub struct HlsReplayPublishMetrics {
    /// Indique si HLS etait active au moment de l'appel.
    pub hls_enabled: bool,
    /// Duree totale entre l'admission du WAV et la mise a jour du replay.
    pub publish_duration_ms: u64,
    /// Duree passee dans `ffmpeg` pour produire les segments `.ts`.
    pub render_duration_ms: u64,
    /// Nombre de segments ajoutes a la playlist replay.
    pub segment_count: u64,
}

impl HlsReplayPublishMetrics {
    /// Construit une mesure neutre quand HLS est desactive mais que la TTS continue.
    pub(in crate::features::audio::hls::origin) fn disabled() -> Self {
        Self {
            hls_enabled: false,
            publish_duration_ms: 0,
            render_duration_ms: 0,
            segment_count: 0,
        }
    }

    /// Construit une mesure neutre quand le stream ne doit pas entrer dans le replay courant.
    pub(in crate::features::audio::hls::origin) fn skipped_current_day() -> Self {
        Self {
            hls_enabled: true,
            publish_duration_ms: 0,
            render_duration_ms: 0,
            segment_count: 0,
        }
    }
}

/// Metadata HTTP minimale du replay HLS du jour.
#[derive(Debug, Clone, Serialize)]
pub struct HlsReplayTodayMetadata {
    /// Indique si l'origin HLS est active cote backend.
    pub enabled: bool,
    /// Indique si au moins un segment du jour est actuellement disponible.
    pub available: bool,
    /// Jour local couvert par le replay, au format `YYYY-MM-DD`.
    pub local_day: String,
    /// Playlist HLS publique du replay du jour.
    pub playlist_url: String,
    /// Nombre de segments connus pour le jour courant.
    pub segment_count: usize,
    /// Premiere sequence du replay, si un segment existe.
    pub first_sequence: Option<u64>,
    /// Derniere sequence du replay, si un segment existe.
    pub latest_sequence: Option<u64>,
    /// Date de construction de la metadata.
    pub updated_at: String,
}
