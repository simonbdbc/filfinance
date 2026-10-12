//! Facade du client HTTP de publication vers le Worker edge.

use reqwest::Client;

use crate::config::EdgeConfig;

mod error;
mod factory;
mod request;
mod retry;
mod status;

/// Client de publication HTTP vers le Worker edge.
#[derive(Clone)]
pub(super) struct EdgePublisher {
    // La config est possedee pour que la tache vive independamment du `Arc<AppConfig>`.
    config: EdgeConfig,
    // `reqwest::Client` est clonable et mutualise le pool HTTP interne.
    client: Client,
}
