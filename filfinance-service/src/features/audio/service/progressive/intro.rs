//! Facade de publication de l'intro d'un clip HLS progressif.

mod synthesis;
mod trace;
mod workflow;

pub(in crate::features::audio::service) use workflow::publish_progressive_clip_intro;
