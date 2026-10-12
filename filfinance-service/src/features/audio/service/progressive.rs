//! Facade des clips HLS progressifs et evenements audio live.

mod control_plane;
mod failure;
mod finalize;
mod intro;
mod timing;
mod types;

pub(super) use failure::publish_progressive_clip_failed;
pub(super) use finalize::finalize_progressive_clip_body;
pub(super) use intro::publish_progressive_clip_intro;
pub(in crate::features::audio::service) use types::ProgressiveClipContext;
