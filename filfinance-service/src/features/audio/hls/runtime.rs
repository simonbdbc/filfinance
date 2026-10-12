//! Facade d'observabilite runtime HLS.

mod http;
mod latest;
mod samples;
mod snapshot;

pub use http::hls_runtime;
pub(in crate::features::audio::hls) use latest::HlsRuntimeLatest;
pub(in crate::features::audio::hls) use samples::{percentile_duration_ms, push_duration_sample};
pub(crate) use snapshot::HlsRuntimeSnapshot;
