//! Provider Piper long-vivant et cycle de vie du process helper.

mod process;
mod synthesis;

use std::sync::Arc;

use tokio::{
    io::BufReader,
    process::{Child, ChildStdin, ChildStdout},
};

use crate::features::audio::tts::runtime::TtsRuntimeState;

use super::args::ensure_piper_session_args;

/// Provider `Piper` pilote via un helper Python garde charge en memoire.
#[derive(Debug)]
pub(in crate::features::audio) struct PiperLocalSessionProvider {
    runtime_state: Arc<TtsRuntimeState>,
    provider_name: String,
    program: String,
    args: Vec<String>,
    models_dir: String,
    rate_words_per_minute: u32,
    timeout_ms: u64,
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    stdout: Option<BufReader<ChildStdout>>,
}

impl PiperLocalSessionProvider {
    /// Construit l'adapter sans demarrer le helper, pour garder un demarrage backend leger.
    pub(in crate::features::audio::tts) fn new(
        runtime_state: Arc<TtsRuntimeState>,
        provider_name: String,
        program: String,
        args: Vec<String>,
        models_dir: String,
        rate_words_per_minute: u32,
        timeout_ms: u64,
    ) -> Self {
        Self {
            runtime_state,
            provider_name,
            program,
            args: ensure_piper_session_args(&args),
            models_dir,
            rate_words_per_minute,
            timeout_ms,
            child: None,
            stdin: None,
            stdout: None,
        }
    }

    /// Retourne le nom provider sans exposer les champs internes.
    pub(in crate::features::audio::tts) fn provider_name(&self) -> &str {
        &self.provider_name
    }

    /// Retourne la voix runtime active.
    pub(in crate::features::audio::tts) fn voice(&self) -> String {
        self.runtime_state.current_voice()
    }
}
