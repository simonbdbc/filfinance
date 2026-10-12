//! Facade du pipeline chaud d'ingestion.

mod dedup;
mod duplicates;
mod runtime;

// La facade garde le point d'entree historique utilise par le wiring des workers.
pub(crate) use runtime::run;
// `cfg(test)` expose la synchronisation aux seuls scenarios Rust internes.
#[cfg(test)]
pub(crate) use runtime::run_with_startup_signal;
