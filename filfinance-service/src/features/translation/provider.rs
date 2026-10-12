//! Port provider traduction utilise par le worker applicatif.

mod constants;
mod factory;
mod lifecycle;
mod operations;

use crate::features::translation::providers::{
    heuristic::HeuristicTranslationProvider, local_command::LocalCommandTranslationProvider,
    local_command_session::LocalCommandSessionTranslationProvider,
};

pub(crate) use constants::{
    HEURISTIC_PROVIDER, LOCAL_COMMAND_PROVIDER, LOCAL_COMMAND_SESSION_PROVIDER,
    OPUS_MT_LOCAL_COMMAND_PROVIDER, OPUS_MT_LOCAL_SESSION_PROVIDER,
};

/// Provider de traduction encapsule pour garder le pipeline stable.
#[derive(Debug)]
pub(crate) enum TranslationProvider {
    // L'enum permet de cacher plusieurs implementations derriere un seul point d'appel,
    // sans exposer de generics complexes au reste du worker.
    Heuristic(HeuristicTranslationProvider),
    LocalCommand(LocalCommandTranslationProvider),
    LocalCommandSession(Box<LocalCommandSessionTranslationProvider>),
}
