//! Echange JSONL avec le helper de traduction long-vivant.

use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

use crate::{
    errors::AppError, features::translation::providers::local_command::LocalCommandResponse,
};

use super::super::provider::LocalCommandSessionTranslationProvider;

impl LocalCommandSessionTranslationProvider {
    /// Echange une requete/reponse JSONL avec le helper de traduction.
    pub(in crate::features::translation::providers::local_command_session) async fn exchange_line(
        &mut self,
        payload_line: String,
    ) -> Result<LocalCommandResponse, AppError> {
        // On emprunte les handles mutables internes; si la session manque, c'est une erreur de protocole.
        let stdin = self.stdin.as_mut().ok_or_else(|| {
            AppError::downstream(format!(
                "translation session stdin is not available for {}",
                self.provider_name
            ))
        })?;
        stdin.write_all(payload_line.as_bytes()).await?;
        stdin.flush().await?;

        let stdout = self.stdout.as_mut().ok_or_else(|| {
            AppError::downstream(format!(
                "translation session stdout is not available for {}",
                self.provider_name
            ))
        })?;
        let mut line = String::new();
        let bytes_read = stdout.read_line(&mut line).await?;
        if bytes_read == 0 {
            // `0` signifie EOF: le helper a ferme sa sortie de maniere inattendue.
            return Err(AppError::downstream(format!(
                "translation session for {} closed stdout unexpectedly",
                self.provider_name
            )));
        }

        // `trim()` retire le `\n` terminal du protocole JSONL avant le parse JSON.
        let response: LocalCommandResponse = serde_json::from_str(line.trim())?;
        if let Some(error) = response.error.clone() {
            return Err(AppError::downstream(format!(
                "translation session for {} returned an error: {}",
                self.provider_name, error
            )));
        }

        Ok(response)
    }
}
