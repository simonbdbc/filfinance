//! Facade des types d'historique des traductions.

mod filters;
mod page;
mod params;
mod scope;

pub use page::TranslationHistoryPage;
pub use params::TranslationHistoryParams;
pub use scope::TranslationHistoryScope;
