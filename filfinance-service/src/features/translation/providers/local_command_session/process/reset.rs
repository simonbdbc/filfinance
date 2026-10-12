//! Reset et nettoyage du process de traduction long-vivant.

use super::super::provider::LocalCommandSessionTranslationProvider;

impl LocalCommandSessionTranslationProvider {
    /// Nettoie la session courante apres un timeout ou une erreur de protocole.
    pub(in crate::features::translation::providers::local_command_session) async fn reset_session(
        &mut self,
    ) {
        // On retire d'abord les handles pour que tout futur appel voie immediatement une session absente.
        self.stdin = None;
        self.stdout = None;

        if let Some(mut child) = self.child.take() {
            let _ = child.kill().await;
            let _ = child.wait().await;
        }
    }
}
