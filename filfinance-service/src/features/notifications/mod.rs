//! Feature `notifications`, responsable des downstreams de notification.

// Telegram reste un downstream optionnel et isole par queue bornee.
pub(crate) mod telegram;
// Les tests notifications communs restent separes des tests Telegram.
#[cfg(test)]
mod tests;
