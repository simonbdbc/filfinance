//! Construction du client HTTP edge.

use std::time::Duration;

use reqwest::Client;

use crate::{config::EdgeConfig, errors::AppError};

use super::EdgePublisher;

impl EdgePublisher {
    /// Construit le client HTTP court utilise par le publisher edge.
    pub(in crate::features::live::edge) fn from_config(
        config: &EdgeConfig,
    ) -> Result<Self, AppError> {
        // Le timeout global borne connexion, ecriture et lecture de la reponse Worker.
        let client = Client::builder()
            .timeout(Duration::from_millis(config.publish_timeout_ms))
            .build()?;

        Ok(Self {
            config: config.clone(),
            client,
        })
    }
}
