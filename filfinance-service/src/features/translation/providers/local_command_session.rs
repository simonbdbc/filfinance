//! Facade du provider de traduction locale garde en session JSONL.

mod args;
mod process;
mod provider;
mod request;
mod response;

pub(crate) use provider::LocalCommandSessionTranslationProvider;
