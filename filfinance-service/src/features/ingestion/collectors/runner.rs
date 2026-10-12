//! Runner commun des collecteurs périodiques.

mod emit;
mod lifecycle;

// La façade expose seulement les helpers encore utilisés par les collecteurs actifs.
pub(crate) use emit::emit_items_to_pipeline;
pub(crate) use lifecycle::{CollectorWorkerHandles, spawn};
