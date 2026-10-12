//! Execution bornee du process local de traduction.

use tokio::{
    task,
    time::{Duration, timeout},
};

use crate::errors::AppError;

use super::super::{command::run_local_translation_command, dto::LocalCommandResponse};

/// Lance la commande one-shot sans bloquer le runtime Tokio.
pub(super) async fn execute_local_command(
    program: String,
    args: Vec<String>,
    payload_bytes: Vec<u8>,
    provider_name: String,
    timeout_ms: u64,
) -> Result<LocalCommandResponse, AppError> {
    // `spawn_blocking` évite de bloquer un worker Tokio avec un process standard synchrone.
    let response = timeout(
        Duration::from_millis(timeout_ms),
        task::spawn_blocking(move || {
            run_local_translation_command(program, args, payload_bytes, provider_name)
        }),
    )
    .await
    .map_err(|_| {
        AppError::downstream(format!(
            "translation command timed out after {timeout_ms}ms"
        ))
    })?
    .map_err(|error| {
        AppError::downstream(format!("translation command task join error: {error}"))
    })??;

    Ok(response)
}
