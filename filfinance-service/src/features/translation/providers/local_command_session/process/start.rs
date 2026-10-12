//! Demarrage du helper de traduction long-vivant.

use std::process::Stdio;

use tokio::{io::BufReader, process::Command as TokioCommand};

use crate::errors::AppError;

use super::super::provider::LocalCommandSessionTranslationProvider;

impl LocalCommandSessionTranslationProvider {
    /// Demarre le helper long-vivant si la session n'existe pas encore.
    pub(in crate::features::translation::providers::local_command_session) async fn ensure_session_started(
        &mut self,
    ) -> Result<(), AppError> {
        if self.stdin.is_some() && self.stdout.is_some() {
            return Ok(());
        }

        // `TokioCommand` est la version async de `Command`; elle s'integre proprement au runtime.
        let mut child = TokioCommand::new(&self.program)
            .args(&self.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|error| {
                AppError::downstream(format!(
                    "failed to spawn translation session for {}: {}",
                    self.provider_name, error
                ))
            })?;

        let stdin = child.stdin.take().ok_or_else(|| {
            AppError::downstream(format!(
                "translation session for {} did not expose stdin",
                self.provider_name
            ))
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            AppError::downstream(format!(
                "translation session for {} did not expose stdout",
                self.provider_name
            ))
        })?;

        self.stdin = Some(stdin);
        self.stdout = Some(BufReader::new(stdout));
        self.child = Some(child);

        Ok(())
    }
}
