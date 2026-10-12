//! Lecture et parsing de la page `/home` FinancialJuice.

use std::time::Duration;

use crate::errors::AppError;

use super::super::{
    FinancialJuiceSiteCollector, ensure_success_response,
    model::{FinancialJuiceHomePageState, parse_home_page_state},
};

impl FinancialJuiceSiteCollector {
    /// Relit `/home` et en extrait l'état JS utile au service.
    pub(super) async fn fetch_home_page(&self) -> Result<FinancialJuiceHomePageState, AppError> {
        // Le GET reste simple: toute la logique d'interprétation du HTML est déportée dans un helper pur.
        let body = self
            .client
            .get(&self.config.home_url)
            .timeout(Duration::from_millis(self.config.request_timeout_ms))
            .send()
            .await
            .map_err(|error| AppError::fetch(self.config.name.clone(), error.to_string()))
            .and_then(|response| ensure_success_response(&self.config.name, response))?
            .text()
            .await
            .map_err(|error| AppError::fetch(self.config.name.clone(), error.to_string()))?;

        // Le parseur renvoie une struct Rust dédiée plutôt qu'une map dynamique pour garder un contrat typé.
        parse_home_page_state(&self.config.name, &body)
    }
}
