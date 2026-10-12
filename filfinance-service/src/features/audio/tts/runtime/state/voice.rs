//! Lecture de la voix active du runtime TTS.

use super::TtsRuntimeState;

impl TtsRuntimeState {
    /// Retourne la voix active au moment courant.
    pub fn current_voice(&self) -> String {
        // `RwLock::read()` peut etre empoisonne si un panic a eu lieu pendant une ecriture.
        // Ici on recupere quand meme la valeur pour ne pas casser tout le service audio.
        self.current_voice
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }
}
