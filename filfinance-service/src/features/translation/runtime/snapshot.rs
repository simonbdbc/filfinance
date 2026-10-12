//! DTO serialisable des metriques runtime de traduction.

// `Serialize` garde le payload HTTP runtime stable.
use serde::Serialize;

/// Vue serialisable des metriques runtime de traduction exposees au debug HTTP.
#[derive(Debug, Clone, Serialize)]
pub struct TranslationRuntimeSnapshot {
    /// Indique si le worker traduction est active dans la configuration courante.
    pub enabled: bool,
    /// Provider configure, utile pour distinguer fallback heuristique et OPUS-MT local.
    pub provider: String,
    /// Profil qualite/latence applique au provider OPUS-MT local.
    pub profile: String,
    /// Langue source attendue par le pipeline de traduction.
    pub source_language: String,
    /// Langue cible produite pour le client et la TTS.
    pub target_language: String,
    /// Nombre de workers traduction configures.
    pub worker_count: usize,
    /// Indique si un warmup provider est execute au demarrage.
    pub warmup_enabled: bool,
    /// Nombre d'evenements entres dans le worker traduction.
    pub received_total: u64,
    /// Nombre de traductions persistees avec succes.
    pub translated_total: u64,
    /// Nombre d'evenements ignores avant appel provider.
    pub skipped_total: u64,
    /// Nombre de traductions ayant bascule vers le fallback heuristique.
    pub fallback_total: u64,
    /// Nombre d'echecs persistants visibles par l'operateur.
    pub failed_total: u64,
    /// Estimation instantanee des items encore en traitement ou en attente.
    pub pending_total: u64,
    /// Derniere latence de traduction observee.
    pub last_translation_duration_ms: Option<u64>,
    /// Mode renvoye par le provider pour la derniere traduction reussie.
    pub last_translation_mode: Option<String>,
    /// Horodatage RFC3339 de la derniere traduction reussie.
    pub last_success_at: Option<String>,
    /// Horodatage RFC3339 de la derniere erreur runtime.
    pub last_error_at: Option<String>,
    /// Derniere erreur runtime utile au diagnostic.
    pub last_error: Option<String>,
    /// Horodatage RFC3339 de fin de warmup.
    pub warmup_completed_at: Option<String>,
    /// Duree du warmup provider en millisecondes.
    pub warmup_duration_ms: Option<u64>,
    /// Nombre de samples conserves pour calculer les percentiles.
    pub translation_duration_sample_count: usize,
    /// Nombre courant d'entrees glossaire actives.
    pub glossary_entries_loaded: u64,
    /// Nombre cumule de remplacements glossaire observes.
    pub glossary_hits_total: u64,
    /// Derniere erreur glossaire non bloquante.
    pub glossary_last_error: Option<String>,
    /// Latence p50 des traductions reussies sur la fenetre recente.
    pub translation_duration_p50_ms: Option<u64>,
    /// Latence p95 des traductions reussies sur la fenetre recente.
    pub translation_duration_p95_ms: Option<u64>,
}
