//! Feature `ingestion`, responsable des collecteurs et du pipeline chaud.

// Les collecteurs gardent les integrations amont isolees du reste du service.
pub(crate) mod collectors;
// Les handlers HTTP operateur exposent les controles runtime de la feature.
pub mod http;
// Le monitoring verifie l'etat effectif du fallback navigateur quand il devient necessaire.
pub(crate) mod monitor;
// Le pipeline orchestre ingestion, deduplication, persistence et broadcast brut.
pub(crate) mod pipeline;
// L'etat runtime garde les overrides admin sans modifier la configuration chargee depuis l'env.
pub mod runtime;
// Les parametres admin de source X sont persistés dans SQLite.
pub(crate) mod x_settings;
// Les tests ingestion communs restent separes des tests collecteurs.
#[cfg(test)]
mod tests;
