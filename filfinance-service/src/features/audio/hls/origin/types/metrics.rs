//! Facade des métriques et metadata publiques de l'origin HLS.

mod clip;
mod replay;

pub use clip::{HlsClipFinalizeMetrics, HlsClipInitialPublish};
pub use replay::{HlsReplayPublishMetrics, HlsReplayTodayMetadata};
