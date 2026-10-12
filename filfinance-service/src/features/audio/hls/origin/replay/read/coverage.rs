//! Couverture applicative de l'index replay HLS.

use std::collections::HashSet;

use tracing::warn;

use super::super::super::{
    HlsOriginState,
    helpers::{current_local_day, replay_segments_for_day},
};

impl HlsOriginState {
    /// Retourne les traductions deja presentes dans le replay du jour courant.
    pub(in crate::features::audio) fn replay_today_translation_ids(&self) -> HashSet<i64> {
        let local_day = current_local_day();
        let inner = self.inner.lock().expect("hls origin mutex poisoned");

        replay_segments_for_day(&inner.segments, &local_day)
            .into_iter()
            .filter_map(|segment| segment.translation_id)
            .collect()
    }

    /// Indique si le replay courant contient encore des segments sans metadata persistante.
    pub(in crate::features::audio) fn replay_today_has_unmapped_segments(&self) -> bool {
        let local_day = current_local_day();
        let inner = self.inner.lock().expect("hls origin mutex poisoned");

        replay_segments_for_day(&inner.segments, &local_day)
            .iter()
            .any(|segment| segment.translation_id.is_none())
    }

    /// Supprime le replay courant pour reconstruire un index complet depuis la base.
    pub(in crate::features::audio) async fn reset_replay_today_for_rebuild(&self) {
        let local_day = current_local_day();
        let day_dir = self.segments_dir.join(&local_day);

        match tokio::fs::remove_dir_all(&day_dir).await {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                warn!(
                    %error,
                    local_day = %local_day,
                    "failed to remove legacy hls replay day directory before rebuild"
                );
            }
        }

        let mut inner = self.inner.lock().expect("hls origin mutex poisoned");
        inner
            .segments
            .retain(|segment| segment.local_day != local_day);
        inner.next_sequence = 1;
    }
}
