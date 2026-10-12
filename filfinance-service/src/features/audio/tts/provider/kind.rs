//! Enum concrete des providers TTS supportes.

use crate::features::audio::tts::piper_session::PiperLocalSessionProvider;

/// Abstraction des providers TTS pour garder le worker stable.
#[derive(Debug)]
pub(in crate::features::audio) enum TtsProvider {
    PiperLocalSession(PiperLocalSessionProvider),
}
