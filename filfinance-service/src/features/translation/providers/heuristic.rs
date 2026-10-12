//! Facade du provider heuristique embarque.

mod dictionary;
mod protection;
mod provider;

#[cfg(test)]
pub(crate) use dictionary::heuristic_translate_text;
pub(in crate::features::translation::providers) use protection::ProtectedText;
pub(crate) use protection::protect_finance_tokens;
pub(crate) use provider::HeuristicTranslationProvider;
