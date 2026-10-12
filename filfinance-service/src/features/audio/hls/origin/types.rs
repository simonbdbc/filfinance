//! Types de l'origin HLS.

mod clip;
mod inner;
mod metrics;
mod render;
mod replay;

// La facade conserve les imports internes existants `origin::types::*`.
pub(in crate::features::audio) use clip::{
    CalendarSummaryHlsManifest, HlsClip, HlsClipPlaylist, HlsClipSegment,
};
pub(in crate::features::audio) use inner::HlsOriginInner;
pub use metrics::{
    HlsClipFinalizeMetrics, HlsClipInitialPublish, HlsReplayPublishMetrics, HlsReplayTodayMetadata,
};
pub(in crate::features::audio::hls::origin) use render::{PlannedSegment, RenderedSegment};
pub(in crate::features::audio) use replay::HlsSegment;
