//! Facade de preparation des streams audio et payloads HLS.

mod id;
mod payload;
mod prepare;
mod text;
mod types;

// Les sous-modules audio conservent les chemins historiques via `service::stream`.
pub(in crate::features::audio) use id::next_audio_stream_id;
pub(in crate::features::audio::service) use payload::audio_start_payload_for_clip;
pub(in crate::features::audio::service) use prepare::prepare_audio_stream;
pub(in crate::features::audio) use text::build_live_clip_body_text;
pub(in crate::features::audio::service) use types::PreparedAudioStream;
