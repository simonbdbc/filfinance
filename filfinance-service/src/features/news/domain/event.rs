//! Facade des evenements news canoniques et projections persistées.

mod canonical;
mod kind;
mod macro_metadata;
mod persisted;

pub use canonical::NewsEvent;
pub use kind::EventKind;
pub use macro_metadata::MacroMetadata;
pub use persisted::PersistedNewsEvent;
