//! Etat runtime TTS partage entre le worker audio et les handlers d'observabilite.

use std::sync::{Mutex, RwLock, atomic::AtomicU64};

use crate::config::TtsConfig;

use super::{snapshot::TtsRuntimeLatest, watermark::LiveAudioWatermark};

mod counters;
mod live_audio;
mod records;
mod snapshot_builder;
mod voice;

/// Etat runtime mutable du provider TTS partage entre le worker et les handlers temps reel.
#[derive(Debug)]
pub struct TtsRuntimeState {
    // La voix change peu souvent, mais plusieurs lecteurs peuvent la consulter en parallele.
    current_voice: RwLock<String>,
    enabled: bool,
    provider: String,
    warmup_enabled: bool,
    worker_count: usize,
    output_sample_rate_hz: u32,
    received_total: AtomicU64,
    skipped_total: AtomicU64,
    generated_total: AtomicU64,
    failed_total: AtomicU64,
    latest: Mutex<TtsRuntimeLatest>,
    live_audio_watermark: Mutex<Option<LiveAudioWatermark>>,
}

impl TtsRuntimeState {
    /// Initialise l'etat runtime mutable a partir de la configuration chargee au demarrage.
    pub fn new(config: &TtsConfig) -> Self {
        Self {
            // La voix runtime est canonisee des l'initialisation pour qu'une ancienne config
            // `fr_FR-upmc-medium` bascule proprement vers le premier speaker public expose.
            current_voice: RwLock::new(
                crate::features::audio::tts::piper_session::resolve_initial_tts_voice(config),
            ),
            enabled: config.enabled,
            provider: config.provider.clone(),
            warmup_enabled: config.warmup_enabled,
            worker_count: config.worker_count,
            output_sample_rate_hz: config.output_sample_rate_hz,
            received_total: AtomicU64::new(0),
            skipped_total: AtomicU64::new(0),
            generated_total: AtomicU64::new(0),
            failed_total: AtomicU64::new(0),
            latest: Mutex::new(TtsRuntimeLatest::default()),
            live_audio_watermark: Mutex::new(None),
        }
    }
}
