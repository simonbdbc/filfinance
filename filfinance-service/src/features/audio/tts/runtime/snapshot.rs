//! DTO et dernier etat mutable du runtime TTS.

// Le snapshot runtime TTS est expose en JSON via l'API de debug.
use serde::Serialize;

/// Vue serialisable de l'etat runtime TTS exposee a l'API de debug.
#[derive(Debug, Clone, Serialize)]
pub struct TtsRuntimeSnapshot {
    /// Indique si le worker TTS est active dans la configuration courante.
    pub enabled: bool,
    /// Provider TTS actif, generalement `piper_local_session`.
    pub provider: String,
    /// Voix Piper effectivement utilisee au moment du snapshot.
    pub current_voice: String,
    /// Indique si le provider TTS est precharge au demarrage.
    pub warmup_enabled: bool,
    /// Nombre de workers TTS configures.
    pub worker_count: usize,
    /// Frequence de sortie attendue par les clients audio.
    pub output_sample_rate_hz: u32,
    /// Nombre de traductions FR entrees dans le worker TTS.
    pub received_total: u64,
    /// Nombre d'items ignores avant synthese.
    pub skipped_total: u64,
    /// Nombre de WAV generes avec succes.
    pub generated_total: u64,
    /// Nombre d'echecs de synthese.
    pub failed_total: u64,
    /// Estimation instantanee des items TTS encore en cours.
    pub pending_total: u64,
    /// Derniere duree de synthese Piper observee.
    pub last_generation_duration_ms: Option<u64>,
    /// Taille du dernier WAV produit.
    pub last_total_bytes: Option<u64>,
    /// Nombre de chunks du dernier stream produit.
    pub last_chunk_count: Option<u64>,
    /// Identifiant du dernier stream audio genere.
    pub last_stream_id: Option<String>,
    /// Horodatage RFC3339 de la derniere synthese reussie.
    pub last_success_at: Option<String>,
    /// Horodatage RFC3339 de la derniere erreur TTS.
    pub last_error_at: Option<String>,
    /// Derniere erreur TTS utile au diagnostic.
    pub last_error: Option<String>,
    /// Horodatage RFC3339 de fin de warmup.
    pub warmup_completed_at: Option<String>,
    /// Duree du warmup TTS en millisecondes.
    pub warmup_duration_ms: Option<u64>,
    /// Horodatage RFC3339 du dernier changement de voix.
    pub voice_changed_at: Option<String>,
    /// Nombre de samples conserves pour les percentiles de synthese.
    pub generation_duration_sample_count: usize,
    /// Latence p50 de synthese sur la fenetre recente.
    pub generation_duration_p50_ms: Option<u64>,
    /// Latence p95 de synthese sur la fenetre recente.
    pub generation_duration_p95_ms: Option<u64>,
}

/// Etat mutable le plus recent du worker TTS.
#[derive(Debug, Clone, Default)]
pub(super) struct TtsRuntimeLatest {
    pub(super) last_generation_duration_ms: Option<u64>,
    pub(super) last_total_bytes: Option<u64>,
    pub(super) last_chunk_count: Option<u64>,
    pub(super) last_stream_id: Option<String>,
    pub(super) last_success_at: Option<String>,
    pub(super) last_error_at: Option<String>,
    pub(super) last_error: Option<String>,
    pub(super) warmup_completed_at: Option<String>,
    pub(super) warmup_duration_ms: Option<u64>,
    pub(super) voice_changed_at: Option<String>,
    pub(super) generation_duration_samples_ms: Vec<u64>,
}
