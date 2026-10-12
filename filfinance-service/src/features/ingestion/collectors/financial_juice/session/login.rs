//! Login WebForms FinancialJuice et extraction du bootstrap authentifié.

use std::time::Duration;

use crate::errors::AppError;

use super::{
    super::{FinancialJuiceSiteCollector, ensure_success_response},
    bootstrap::login_form_body,
};
use crate::features::ingestion::collectors::financial_juice::model::FinancialJuiceBootstrap;

impl FinancialJuiceSiteCollector {
    /// Réalise le vrai login HTTP WebForms puis relit `/home` connecté pour en extraire les variables utiles.
    pub(super) async fn authenticate_and_bootstrap(
        &self,
    ) -> Result<FinancialJuiceBootstrap, AppError> {
        // Le premier GET `/home` sert à récupérer les champs WebForms dynamiques avant le POST login.
        let initial_home = self.fetch_home_page().await?;
        // `as_deref()` convertit `Option<String>` en `Option<&str>` pour éviter une allocation tant que
        // l'on ne sait pas si la valeur est présente.
        let email = self
            .config
            .email
            .as_deref()
            .ok_or_else(|| AppError::Config("missing FinancialJuice email".to_owned()))?;
        let password = self
            .config
            .password
            .as_deref()
            .ok_or_else(|| AppError::Config("missing FinancialJuice password".to_owned()))?;

        // Le login FJ reste du pur WebForms: on reconstruit donc le formulaire attendu champ par champ.
        let body = login_form_body(&initial_home, email, password);
        // On ne garde pas la réponse HTML du POST login: le contrat fiable est de relire `/home`
        // après coup pour vérifier l'état connecté et extraire les variables JS utiles.
        self.client
            .post(&self.config.home_url)
            .timeout(Duration::from_millis(self.config.request_timeout_ms))
            .header("content-type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .map_err(|error| AppError::fetch(self.config.name.clone(), error.to_string()))
            .and_then(|response| ensure_success_response(&self.config.name, response))?;

        // Cette seconde lecture `/home` est la vraie source de vérité du bootstrap authentifié.
        let authenticated_home = self.fetch_home_page().await?;
        authenticated_home.into_authenticated_bootstrap(&self.config.name)
    }
}
