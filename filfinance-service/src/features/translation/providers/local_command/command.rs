//! Execution one-shot d'une commande locale de traduction.

use std::{
    io::Write,
    process::{Command as StdCommand, Stdio},
};

use crate::errors::AppError;

use super::dto::LocalCommandResponse;

/// Execute la commande locale de traduction et parse sa sortie JSON.
pub(super) fn run_local_translation_command(
    program: String,
    args: Vec<String>,
    payload_bytes: Vec<u8>,
    provider_name: String,
) -> Result<LocalCommandResponse, AppError> {
    // Cette voie one-shot privilegie la simplicite de branchement sur la performance.
    let mut child = StdCommand::new(&program)
        .args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            AppError::downstream(format!(
                "failed to spawn translation command for {provider_name}: {error}"
            ))
        })?;

    if let Some(mut stdin) = child.stdin.take() {
        // `take()` deplace le handle hors du `Child` pour pouvoir ecrire dedans explicitement.
        stdin.write_all(&payload_bytes)?;
    }

    let output = child.wait_with_output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        return Err(AppError::downstream(format!(
            "translation command for {provider_name} failed with status {}: {}",
            output.status,
            if stderr.is_empty() {
                "no stderr returned"
            } else {
                &stderr
            }
        )));
    }

    serde_json::from_slice(&output.stdout).map_err(AppError::from)
}
