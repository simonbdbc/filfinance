//! Facade du provider de traduction locale execute en process one-shot.

mod args;
mod command;
mod dto;
mod provider;

pub(crate) use args::resolved_local_command_args;
#[cfg(test)]
pub(crate) use args::translation_command_args_for_profile;
pub(super) use dto::{LocalCommandPayload, LocalCommandResponse};
pub(crate) use provider::LocalCommandTranslationProvider;
