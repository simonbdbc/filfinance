//! Etat runtime partage pour l'observabilite de la traduction locale.

mod counters;
mod records;
mod snapshot_builder;

// `Mutex` protege les dernieres valeurs runtime et `AtomicU64` garde les compteurs hot path.
use std::sync::{Mutex, atomic::AtomicU64};

// La configuration initialise le snapshot runtime expose aux operateurs.
use crate::config::TranslationConfig;

use super::latest::TranslationRuntimeLatest;

/// Etat runtime partage pour l'observabilite de la traduction locale.
#[derive(Debug)]
pub struct TranslationRuntimeState {
    enabled: bool,
    provider: String,
    profile: String,
    source_language: String,
    target_language: String,
    worker_count: usize,
    warmup_enabled: bool,
    received_total: AtomicU64,
    translated_total: AtomicU64,
    skipped_total: AtomicU64,
    fallback_total: AtomicU64,
    failed_total: AtomicU64,
    glossary_entries_loaded: AtomicU64,
    glossary_hits_total: AtomicU64,
    latest: Mutex<TranslationRuntimeLatest>,
}

impl TranslationRuntimeState {
    /// Construit l'etat runtime partage a partir de la configuration chargee.
    pub fn new(config: &TranslationConfig) -> Self {
        Self {
            enabled: config.enabled,
            provider: config.provider.clone(),
            profile: config.profile.clone(),
            source_language: config.source_language.clone(),
            target_language: config.target_language.clone(),
            worker_count: config.worker_count,
            warmup_enabled: config.warmup_enabled,
            received_total: AtomicU64::new(0),
            translated_total: AtomicU64::new(0),
            skipped_total: AtomicU64::new(0),
            fallback_total: AtomicU64::new(0),
            failed_total: AtomicU64::new(0),
            glossary_entries_loaded: AtomicU64::new(0),
            glossary_hits_total: AtomicU64::new(0),
            latest: Mutex::new(TranslationRuntimeLatest::default()),
        }
    }
}
