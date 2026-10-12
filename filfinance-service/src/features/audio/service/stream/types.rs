//! Types internes des streams audio prepares.

use super::super::super::domain::AudioStartPayload;

/// Stream audio deja prepare, pret pour clip temps reel ou replay HLS.
#[derive(Debug, Clone)]
pub(in crate::features::audio::service) struct PreparedAudioStream {
    /// Metadata de clip deja correlee a la traduction source.
    pub(in crate::features::audio::service) start_payload: AudioStartPayload,
    /// Audio source complet, conserve pour produire HLS.
    pub(in crate::features::audio::service) audio_bytes: Vec<u8>,
    /// Duree de synthese du WAV, reutilisee par les traces de run de test.
    pub(in crate::features::audio::service) synthesis_duration_ms: u64,
}
