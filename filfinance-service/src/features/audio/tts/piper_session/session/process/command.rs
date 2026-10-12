//! Echange JSONL avec le helper Piper long-vivant.

use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt};

use crate::errors::AppError;

use super::super::{
    super::protocol::{PiperCommandPayload, PiperCommandResponse},
    PiperLocalSessionProvider,
};

impl PiperLocalSessionProvider {
    /// Echange une commande JSONL complete avec le helper Piper.
    pub(in crate::features::audio::tts::piper_session::session) async fn send_session_command(
        &mut self,
        payload: PiperCommandPayload<'_>,
    ) -> Result<PiperCommandResponse, AppError> {
        self.ensure_session_started().await?;
        self.write_payload(payload).await?;
        let response = self.read_response().await?;
        self.validate_response(&response).await?;
        Ok(response)
    }

    /// Ecrit une ligne JSON complete sur `stdin`.
    async fn write_payload(&mut self, payload: PiperCommandPayload<'_>) -> Result<(), AppError> {
        crate::features::calendar_logs::operation("write_payload", serde_json::json!({}), async {
            let payload_line = format!(
                "{}\n",
                serde_json::to_string(&payload).map_err(AppError::from)?
            );

            crate::features::calendar_logs::payload("piper.stdin", &payload_line);
            let stdin = self.stdin.as_mut().ok_or_else(|| {
                AppError::downstream(format!(
                    "piper session stdin is not available for {}",
                    self.provider_name
                ))
            })?;
            stdin.write_all(payload_line.as_bytes()).await?;
            stdin.flush().await?;
            Ok(())
        })
        .await
    }

    /// Lit une ligne JSON complete depuis `stdout`.
    async fn read_response(&mut self) -> Result<PiperCommandResponse, AppError> {
        let stdout = self.stdout.as_mut().ok_or_else(|| {
            AppError::downstream(format!(
                "piper session stdout is not available for {}",
                self.provider_name
            ))
        })?;

        let mut line = String::new();
        let bytes_read = {
            let response = stdout.read_line(&mut line);
            tokio::pin!(response);
            let mut diagnostics = [0u8; 2048];
            let mut stderr = self.child.as_mut().and_then(|child| child.stderr.as_mut());
            loop {
                if let Some(stream) = stderr.as_mut() {
                    tokio::select! {
                        result = &mut response => break result?,
                        result = stream.read(&mut diagnostics) => match result {
                            Ok(0) | Err(_) => stderr = None,
                            Ok(size) => crate::features::calendar_logs::bytes("piper.stderr", &diagnostics[..size]),
                        }
                    }
                } else {
                    break response.await?;
                }
            }
        };
        if bytes_read == 0 {
            self.shutdown().await;
            return Err(AppError::downstream(format!(
                "piper session for {} closed stdout unexpectedly",
                self.provider_name
            )));
        }

        crate::features::calendar_logs::piper_response(&line);
        serde_json::from_str::<PiperCommandResponse>(line.trim_end()).map_err(|error| {
            AppError::downstream(format!(
                "invalid JSON from piper session for {}: {}",
                self.provider_name, error
            ))
        })
    }

    /// Valide les invariants minimaux du payload de reponse.
    async fn validate_response(&mut self, response: &PiperCommandResponse) -> Result<(), AppError> {
        crate::features::calendar_logs::operation(
            "validate_response",
            serde_json::json!({}),
            async {
                if response.sample_rate_hz == Some(0) {
                    self.shutdown().await;
                    return Err(AppError::downstream(
                        "piper session returned an invalid sample rate".to_owned(),
                    ));
                }

                Ok(())
            },
        )
        .await
    }
}
