//! Types internes du domaine traduction.

mod draft;
mod history;
mod persisted;
mod provider;

// La facade conserve les chemins historiques `translation::domain::*`.
pub use draft::NewsTranslationDraft;
pub use history::{TranslationHistoryPage, TranslationHistoryParams, TranslationHistoryScope};
pub use persisted::PersistedNewsTranslation;
pub(crate) use provider::{ProviderTranslation, TranslationDraftBuild, TranslationRequest};
