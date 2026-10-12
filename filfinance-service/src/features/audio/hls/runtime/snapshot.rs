//! DTO serialisable des metriques runtime de l'origin HLS.

// `Serialize` expose le snapshot runtime via l'API operateur.
use serde::Serialize;

/// Vue serialisable des metriques runtime de l'origin HLS.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct HlsRuntimeSnapshot {
    /// Indique si l'origin HLS produit actuellement des clips/replay.
    pub enabled: bool,
    /// Commande `ffmpeg` configuree dans l'environnement serveur.
    pub ffmpeg_command: String,
    /// Duree cible des segments HLS replay.
    pub replay_segment_target_duration_secs: u32,
    /// Duree cible des segments HLS clip progressifs.
    pub clip_segment_target_duration_secs: u32,
    /// Nombre de clips HLS progressifs conserves en memoire.
    pub clip_count: usize,
    /// Nombre de segments replay du jour conserves en memoire.
    pub replay_segment_count: usize,
    /// Premiere sequence replay du jour, si un segment existe.
    pub first_replay_sequence: Option<u64>,
    /// Derniere sequence replay du jour, si un segment existe.
    pub latest_replay_sequence: Option<u64>,
    /// Nombre de streams TTS publies avec succes dans le replay HLS.
    pub replay_published_total: u64,
    /// Nombre total de requetes playlist sur les clips HLS evenementiels.
    pub clip_playlist_requests_total: u64,
    /// Nombre total de requetes segment sur les clips HLS evenementiels.
    pub clip_segment_requests_total: u64,
    /// Nombre total de requetes playlist sur le replay HLS du jour.
    pub replay_playlist_requests_total: u64,
    /// Nombre total de requetes segment sur le replay HLS du jour.
    pub replay_segment_requests_total: u64,
    /// Nombre de publications HLS echouees.
    pub failed_total: u64,
    /// Nombre d'echecs attribues au lancement ou au resultat de `ffmpeg`.
    pub ffmpeg_failed_total: u64,
    /// Derniere duree totale de publication HLS.
    pub last_publish_duration_ms: Option<u64>,
    /// Derniere duree d'encodage `ffmpeg`.
    pub last_render_duration_ms: Option<u64>,
    /// Nombre de segments du dernier stream publie.
    pub last_segment_count: Option<u64>,
    /// Identifiant du dernier stream publie.
    pub last_stream_id: Option<String>,
    /// Taille WAV du dernier stream recu par HLS.
    pub last_total_bytes: Option<u64>,
    /// Horodatage RFC3339 de la derniere publication reussie.
    pub last_success_at: Option<String>,
    /// Horodatage RFC3339 de la derniere erreur HLS.
    pub last_error_at: Option<String>,
    /// Derniere erreur HLS utile au diagnostic.
    pub last_error: Option<String>,
    /// Nombre de samples conserves pour les percentiles de publication.
    pub publish_duration_sample_count: usize,
    /// Latence p50 de publication sur la fenetre recente.
    pub publish_duration_p50_ms: Option<u64>,
    /// Latence p95 de publication sur la fenetre recente.
    pub publish_duration_p95_ms: Option<u64>,
    /// Nombre de samples conserves pour les percentiles `ffmpeg`.
    pub render_duration_sample_count: usize,
    /// Latence p50 d'encodage `ffmpeg` sur la fenetre recente.
    pub render_duration_p50_ms: Option<u64>,
    /// Latence p95 d'encodage `ffmpeg` sur la fenetre recente.
    pub render_duration_p95_ms: Option<u64>,
}
