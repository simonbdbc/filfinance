//! Etat mutable de l'origin HLS.

use super::{
    helpers::{current_local_day, load_existing_segments_for_day},
    types::HlsOriginInner,
};
use crate::{config::HlsConfig, features::audio::hls::runtime::HlsRuntimeLatest};

// `PathBuf` représente la racine locale des fichiers HLS.
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
// `AtomicU64` garde des compteurs runtime sans bloquer le hot path.
// `Mutex` protège l'index des segments HLS et les dernières métriques exposées aux opérateurs.
use std::sync::{Mutex, atomic::AtomicU64};

/// Origin HLS partage entre le worker TTS et les routes HTTP.
#[derive(Debug)]
pub struct HlsOriginState {
    /// Flag de migration progressive: le code peut être présent sans produire de HLS.
    pub(in crate::features::audio::hls::origin) enabled: bool,
    /// Racine locale où les segments `.ts` sont stockés.
    pub(in crate::features::audio::hls::origin) segments_dir: PathBuf,
    /// Binaire d'encodage configuré, généralement `ffmpeg`.
    pub(in crate::features::audio::hls::origin) ffmpeg_command: String,
    /// Durée cible des segments replay.
    pub(in crate::features::audio::hls::origin) replay_segment_target_duration_secs: u32,
    /// Durée cible des clips par news, optimisée pour démarrer plus vite.
    pub(in crate::features::audio::hls::origin) clip_segment_target_duration_secs: u32,
    /// Mutex court qui protège seulement l'index en mémoire.
    pub(in crate::features::audio) inner: Mutex<HlsOriginInner>,
    /// Nombre de publications HLS replay abouties.
    pub(in crate::features::audio::hls::origin) replay_published_total: AtomicU64,
    /// Compteur diagnostic des hits playlist sur les clips HLS.
    pub(in crate::features::audio::hls::origin) clip_playlist_requests_total: AtomicU64,
    /// Compteur diagnostic des hits segment sur les clips HLS.
    pub(in crate::features::audio::hls::origin) clip_segment_requests_total: AtomicU64,
    /// Compteur diagnostic des hits playlist replay du jour.
    pub(in crate::features::audio::hls::origin) replay_playlist_requests_total: AtomicU64,
    /// Compteur diagnostic des hits segment replay du jour.
    pub(in crate::features::audio::hls::origin) replay_segment_requests_total: AtomicU64,
    /// Nombre de publications HLS échouées.
    pub(in crate::features::audio::hls::origin) failed_total: AtomicU64,
    /// Nombre d'échecs imputables au process `ffmpeg`.
    pub(in crate::features::audio::hls::origin) ffmpeg_failed_total: AtomicU64,
    /// Dernières métriques HLS lues par l'API runtime.
    pub(in crate::features::audio::hls::origin) latest: Mutex<HlsRuntimeLatest>,
}

impl HlsOriginState {
    /// Construit l'origin HLS depuis la configuration runtime.
    pub fn from_config(config: &HlsConfig) -> Self {
        // Le dossier est possédé une seule fois pour être réutilisé par le bootstrap et l'état.
        let segments_dir = PathBuf::from(&config.segments_dir);
        // Le jour courant est capturé au démarrage pour recharger les segments persistants du jour.
        let local_day = current_local_day();
        // Le replay du jour peut survivre à un redémarrage grâce au volume Docker `data/hls`.
        let existing_segments = if config.enabled {
            load_existing_segments_for_day(
                &segments_dir,
                &local_day,
                config.replay_segment_target_duration_secs,
            )
        } else {
            VecDeque::new()
        };
        // La prochaine séquence repart après la dernière séquence déjà trouvée sur disque.
        let next_sequence = existing_segments
            .back()
            .map(|segment| segment.sequence + 1)
            .unwrap_or(1);
        // L'état existe même si `enabled=false`, ce qui simplifie `AppState` et les tests.
        Self {
            enabled: config.enabled,
            segments_dir,
            ffmpeg_command: config.ffmpeg_command.clone(),
            replay_segment_target_duration_secs: config.replay_segment_target_duration_secs,
            clip_segment_target_duration_secs: config.clip_segment_target_duration_secs,
            inner: Mutex::new(HlsOriginInner {
                next_sequence,
                segments: existing_segments,
                clips: HashMap::new(),
            }),
            replay_published_total: AtomicU64::new(0),
            clip_playlist_requests_total: AtomicU64::new(0),
            clip_segment_requests_total: AtomicU64::new(0),
            replay_playlist_requests_total: AtomicU64::new(0),
            replay_segment_requests_total: AtomicU64::new(0),
            failed_total: AtomicU64::new(0),
            ffmpeg_failed_total: AtomicU64::new(0),
            latest: Mutex::new(HlsRuntimeLatest::default()),
        }
    }
}
