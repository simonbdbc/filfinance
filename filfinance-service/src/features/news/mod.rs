//! Feature `news`, responsable du flux persiste, de l'historique et de la retention.

// `domain` porte les contrats historiques pendant la premiere passe de refonte.
pub(crate) mod domain;
// `errors` reserve les erreurs propres au domaine news.
pub(crate) mod errors;
// `http` porte les handlers HTTP deja exposes par le backend.
pub(crate) mod http;
// `repository` expose le store news branche dans `AppState`.
pub(crate) mod repository;
// `retention` porte le worker de purge J/J-1.
pub(crate) mod retention;
// `service` reserve les cas d'usage news.
pub(crate) mod service;
// `sqlite` porte le repository SQLite du pipeline news/audio.
pub(crate) mod sqlite;
// `status` expose les projections admin/publiques des sources de collecte.
pub(crate) mod status;
// Les tests news communs restent separes du repository SQLite.
#[cfg(test)]
mod tests;
// La facade publique expose seulement le store necessaire au wiring.
pub(crate) use repository::NewsStore;
