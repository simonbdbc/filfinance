//! Facade de decouverte et canonisation des voix Piper locales.

mod discovery;
mod selection;
mod types;

// La facade conserve les chemins utilises par les tests audio et le provider TTS.
pub(in crate::features::audio) use discovery::discover_piper_french_voice_selections;
pub(in crate::features::audio) use selection::canonicalize_piper_voice_selection;
pub(in crate::features::audio::tts) use selection::resolve_initial_tts_voice;
pub(in crate::features::audio) use types::PiperVoiceSelection;
