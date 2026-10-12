//! Fermeture de la session Piper locale.

use super::super::PiperLocalSessionProvider;

impl PiperLocalSessionProvider {
    /// Ferme proprement la session Piper et tue le child si besoin.
    pub(in crate::features::audio::tts) async fn shutdown(&mut self) {
        self.stdin = None;
        self.stdout = None;

        if let Some(mut child) = self.child.take() {
            // `start_kill` evite d'attendre une cooperation du helper Python avant `wait`.
            let _ = child.start_kill();
            let _ = child.wait().await;
        }
    }
}
