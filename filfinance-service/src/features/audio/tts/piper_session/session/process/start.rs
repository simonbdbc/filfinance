//! Demarrage du process helper Piper.

use std::process::Stdio;

use tokio::{io::BufReader, process::Command};

use crate::errors::AppError;

use super::super::PiperLocalSessionProvider;

impl PiperLocalSessionProvider {
    /// Lance le helper Piper si aucune session n'est encore ouverte.
    pub(in crate::features::audio::tts::piper_session::session::process) async fn ensure_session_started(
        &mut self,
    ) -> Result<(), AppError> {
        crate::features::calendar_logs::operation(
            "ensure_session_started",
            serde_json::json!({}),
            async {
                if self.stdin.is_some() && self.stdout.is_some() {
                    return Ok(());
                }

                crate::features::calendar_logs::event(
                    "piper.process_start",
                    serde_json::json!({"program":self.program,"args":self.args}),
                );
                let mut child = Command::new(&self.program)
                    .args(&self.args)
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .kill_on_drop(true)
                    .spawn()
                    .map_err(|error| {
                        AppError::downstream(format!(
                            "failed to spawn piper helper for {}: {}",
                            self.provider_name, error
                        ))
                    })?;

                let stdin = child.stdin.take().ok_or_else(|| {
                    AppError::downstream(format!(
                        "piper session for {} did not expose stdin",
                        self.provider_name
                    ))
                })?;
                let stdout = child.stdout.take().ok_or_else(|| {
                    AppError::downstream(format!(
                        "piper session for {} did not expose stdout",
                        self.provider_name
                    ))
                })?;

                // Le provider garde les trois handles ensemble pour pouvoir relancer une session apres erreur.
                self.stdin = Some(stdin);
                self.stdout = Some(BufReader::new(stdout));
                self.child = Some(child);
                Ok(())
            },
        )
        .await
    }
}
