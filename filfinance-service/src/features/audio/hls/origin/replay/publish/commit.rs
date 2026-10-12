//! Commit en mémoire des segments replay après écriture disque réussie.

use super::super::super::{HlsOriginState, types::PlannedSegment};

impl HlsOriginState {
    /// Ajoute les segments à l'index replay après écriture disque réussie.
    pub(super) fn commit_segments(&self, planned_segments: Vec<PlannedSegment>, local_day: &str) {
        // Le changement de jour vide le replay courant pour ne pas annoncer les segments de la veille.
        let mut inner = self.inner.lock().expect("hls origin mutex poisoned");
        inner
            .segments
            .retain(|segment| segment.local_day == local_day);

        for planned_segment in planned_segments {
            inner.segments.push_back(planned_segment.segment);
        }
    }
}
