//! Worker de synthese vocale serveur et publication des clips HLS audio.

// `normalization` reserve la normalisation de texte avant synthese.
pub(crate) mod normalization;
// `piper_session` reserve l'adapter Piper long-vivant.
pub(crate) mod piper_session;
// `provider` reserve le port provider TTS.
pub(crate) mod provider;
// `runtime` reserve l'etat d'observabilite TTS.
pub(crate) mod runtime;

// La facade conserve le chemin public `features::audio::tts::TtsRuntimeState`.
pub(crate) use runtime::TtsRuntimeState;
// La facade garde un point d'entree court pour le worker TTS.
pub(crate) use crate::features::audio::service::spawn_tts_worker as spawn;
