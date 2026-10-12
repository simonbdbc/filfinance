//! Providers de traduction concrets.

// `heuristic` garde le fallback embarque et les protections de tokens finance.
pub(crate) mod heuristic;
// `local_command` garde le mode process one-shot compatible OPUS-MT.
pub(crate) mod local_command;
// `local_command_session` garde le mode JSONL long-vivant.
pub(crate) mod local_command_session;
