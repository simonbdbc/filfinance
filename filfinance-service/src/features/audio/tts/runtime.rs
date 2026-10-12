//! Facade de l'etat runtime TTS.

mod http;
mod metrics;
mod snapshot;
mod state;
mod watermark;

// La facade conserve le handler et l'etat runtime sur le chemin historique du module runtime.
pub use http::tts_runtime;
pub use state::TtsRuntimeState;
