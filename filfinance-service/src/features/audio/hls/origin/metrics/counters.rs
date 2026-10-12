//! Compteurs HTTP de l'origin HLS.

// `Ordering` suffit en relaxed pour des compteurs diagnostics monotones.
use std::sync::atomic::Ordering;

use super::super::HlsOriginState;

impl HlsOriginState {
    /// Compte une requete HTTP vers une playlist de clip evenementiel.
    pub(in crate::features::audio) fn record_clip_playlist_request(&self) {
        // Cette mesure sert a calculer le cout origin/CDN par news lue.
        self.clip_playlist_requests_total
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Compte une requete HTTP vers un segment de clip evenementiel.
    pub(in crate::features::audio) fn record_clip_segment_request(&self) {
        // Les segments etant immuables, ce compteur expose les hits origin avant cache CDN.
        self.clip_segment_requests_total
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Compte une requete HTTP vers la playlist replay du jour.
    pub(in crate::features::audio) fn record_replay_playlist_request(&self) {
        // Le replay est separe des clips temps reel et doit avoir sa propre observabilite.
        self.replay_playlist_requests_total
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Compte une requete HTTP vers un segment replay du jour.
    pub(in crate::features::audio) fn record_replay_segment_request(&self) {
        // Les segments replay reutilisent le stockage HLS mais pas le modele de lecture temps reel.
        self.replay_segment_requests_total
            .fetch_add(1, Ordering::Relaxed);
    }
}
