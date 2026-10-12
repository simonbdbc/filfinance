//! État mutable interne de l'origin HLS.

// `HashMap` indexe les clips finis par `stream_id`; `VecDeque` garde l'ordre replay.
use std::collections::{HashMap, VecDeque};

use super::{clip::HlsClip, replay::HlsSegment};

/// Etat mutable protege par mutex pour garder les handlers synchrones et rapides.
#[derive(Debug)]
pub(in crate::features::audio) struct HlsOriginInner {
    /// Prochain numero de sequence a affecter a un segment replay publie.
    pub(in crate::features::audio) next_sequence: u64,
    /// Segments replay de la journee courante, gardes dans l'ordre ancien vers recent.
    pub(in crate::features::audio) segments: VecDeque<HlsSegment>,
    /// Clips progressifs indexes par `stream_id`.
    pub(in crate::features::audio) clips: HashMap<String, HlsClip>,
}
