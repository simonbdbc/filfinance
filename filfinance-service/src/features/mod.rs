//! Facade feature-first du backend Fil Finance.

// La feature audio contient le data plane TTS/HLS.
pub(crate) mod audio;
// La feature auth contient les middlewares et la verification JWT Better Auth.
pub(crate) mod auth;
// La feature blacklist isole les rejets operateur tres tot dans le pipeline news.
pub(crate) mod blacklist;
// La feature calendar_day orchestre le calendrier economique journalier Rust/Piper.
pub(crate) mod calendar_day;
pub(crate) mod calendar_logs;
// La feature health isole le healthcheck public.
pub(crate) mod health;
// La feature glossary maitrise les corrections EN -> FR -> vocal.
pub(crate) mod glossary;
// La feature ingestion contient collecteurs et pipeline.
pub(crate) mod ingestion;
// La feature live contient WebSockets, edge et runs E2E.
pub(crate) mod live;
// La feature logs expose le viewer admin des logs backend.
pub(crate) mod logs;
// La feature news contient le flux persiste et l'historique.
pub(crate) mod news;
// La feature notifications contient les downstreams externes.
pub(crate) mod notifications;
// La feature translation contient la derivation EN vers FR.
pub(crate) mod translation;
// Les fixtures de test partagent un bootstrap local entre features sans exposer de code en prod.
#[cfg(test)]
pub(crate) mod test_support;

mod router;
mod workers;

// Le bootstrap ne voit que les deux points d'entree publics de la facade feature-first.
pub use router::router;
#[allow(unused_imports)]
pub use workers::{RuntimeWorkerHandles, spawn_workers};
