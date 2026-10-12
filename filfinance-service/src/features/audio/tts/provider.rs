//! Port provider TTS utilise par le worker audio.

mod factory;
mod kind;
mod lifecycle;
mod types;

pub(in crate::features::audio) use kind::TtsProvider;
pub(in crate::features::audio) use types::{SynthesizedAudio, TtsRequest};
