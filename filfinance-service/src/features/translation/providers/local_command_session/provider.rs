//! Provider local session pilote par une commande externe long-vivante.

mod lifecycle;

use tokio::{
    io::BufReader,
    process::{Child, ChildStdin, ChildStdout},
};

use super::args::ensure_session_flag;

/// Provider local garde en vie dans le worker pour eviter un reload du modele a chaque item.
#[derive(Debug)]
pub(crate) struct LocalCommandSessionTranslationProvider {
    pub(super) provider_name: String,
    pub(super) program: String,
    pub(super) args: Vec<String>,
    pub(super) timeout_ms: u64,
    pub(super) child: Option<Child>,
    pub(super) stdin: Option<ChildStdin>,
    pub(super) stdout: Option<BufReader<ChildStdout>>,
}

impl LocalCommandSessionTranslationProvider {
    /// Construit une session locale sans demarrer le process immediatement.
    pub(crate) fn new(
        provider_name: String,
        program: String,
        args: Vec<String>,
        timeout_ms: u64,
    ) -> Self {
        Self {
            provider_name,
            program,
            args: ensure_session_flag(&args),
            timeout_ms,
            child: None,
            stdin: None,
            stdout: None,
        }
    }

    /// Retourne le nom provider pour les logs sans exposer les handles process.
    pub(crate) fn provider_name(&self) -> &str {
        &self.provider_name
    }
}
