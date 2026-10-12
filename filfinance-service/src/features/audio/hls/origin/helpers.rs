//! Facade des helpers fichiers, temps et parsing de segments HLS.

mod clock;
mod files;
mod replay;
mod sidecar;
mod wav;

pub(super) use clock::looks_like_local_day;
pub(in crate::features::audio) use clock::{
    current_local_day, local_day_for_rfc3339, local_day_for_utc, now_unix_ms,
};
pub(super) use files::{collect_segment_files, sanitize_file_component};
pub(in crate::features::audio) use files::{parse_clip_segment_name, parse_segment_sequence};
pub(super) use replay::{load_existing_segments_for_day, replay_segments_for_day};
pub(super) use sidecar::write_segment_sidecar;
pub(super) use wav::estimate_segment_duration;
pub(in crate::features::audio) use wav::estimate_wav_duration_secs;
