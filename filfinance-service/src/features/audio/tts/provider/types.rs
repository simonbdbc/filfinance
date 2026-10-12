//! DTO internes du port provider TTS.

/// Requete minimale envoyee au provider TTS.
#[derive(Debug, Clone)]
pub(in crate::features::audio) struct TtsRequest {
    pub(in crate::features::audio) text: String,
}

/// Reponse normalisee retournee par le provider TTS.
#[derive(Debug, Clone)]
pub(in crate::features::audio) struct SynthesizedAudio {
    pub(in crate::features::audio) bytes: Vec<u8>,
    pub(in crate::features::audio) audio_format: String,
    pub(in crate::features::audio) mime_type: String,
    pub(in crate::features::audio) voice: String,
}
