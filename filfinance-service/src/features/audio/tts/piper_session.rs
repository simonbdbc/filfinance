//! Adapter Piper pilote par une session locale JSONL.

mod args;
mod protocol;
mod session;
mod voices;

/// Provider TTS nominal base sur un helper Piper garde en session locale.
pub(super) const PIPER_LOCAL_SESSION_PROVIDER: &str = "piper_local_session";

// La facade conserve les chemins utilises par le provider TTS, le runtime et les tests audio.
pub(in crate::features::audio) use session::PiperLocalSessionProvider;
pub(in crate::features::audio::tts) use voices::resolve_initial_tts_voice;
#[allow(unused_imports)]
pub(in crate::features::audio) use voices::{
    PiperVoiceSelection, canonicalize_piper_voice_selection, discover_piper_french_voice_selections,
};
