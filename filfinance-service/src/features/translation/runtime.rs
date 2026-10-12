//! Facade de l'etat runtime et des snapshots d'observabilite de la traduction.

mod latest;
mod samples;
mod snapshot;
mod state;

pub(crate) use state::TranslationRuntimeState;
