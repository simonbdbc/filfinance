//! Appel HTTP GetPreviousNews et isolation des pages backlog corrompues.

// Duration borne les appels HTTP vers l'API interne FinancialJuice.
use std::time::Duration;

// Les logs structurés rendent visibles les pages backlog ignorees.
use tracing::debug;

use crate::errors::AppError;

use super::super::{
    FinancialJuiceSiteCollector, encode_form_body, ensure_success_response,
    model::{FinancialJuiceApiNewsItem, parse_financial_juice_news_items_payload},
};

impl FinancialJuiceSiteCollector {
    /// Récupère une page `GetPreviousNews` normalisée côté DTO.
    async fn fetch_previous_news_page(
        &self,
        old_news_id: i64,
    ) -> Result<Vec<FinancialJuiceApiNewsItem>, AppError> {
        // Même pour le backlog, l'API interne exige une session authentifiée valide.
        let bootstrap = self.ensure_bootstrap(false).await?;
        // Les clés ici reflètent exactement le formulaire `.asmx` observé côté site.
        let body = encode_form_body(&[
            ("info", bootstrap.info.clone()),
            ("TimeOffset", self.config.time_offset_minutes.to_string()),
            ("tabID", "0".to_owned()),
            ("oldID", old_news_id.to_string()),
            ("TickerID", "0".to_owned()),
            ("FeedCompanyID", "0".to_owned()),
            ("strSearch", "\"\"".to_owned()),
            ("extraNID", "0".to_owned()),
        ]);
        // On poste le formulaire tel quel plutôt qu'un JSON car c'est le vrai contrat legacy de FJ.
        let response = self
            .client
            .post(format!(
                "{}/FJService.asmx/GetPreviousNews",
                bootstrap.main_url
            ))
            .timeout(Duration::from_millis(self.config.request_timeout_ms))
            .header("content-type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .map_err(|error| AppError::fetch(self.config.name.clone(), error.to_string()))?;

        let body = ensure_success_response(&self.config.name, response)?
            .text()
            .await
            .map_err(|error| AppError::fetch(self.config.name.clone(), error.to_string()))?;

        // Le backlog FJ peut contenir un objet JSON malforme dans une page; on recupere alors
        // les autres objets valides au lieu de perdre tout le rattrapage.
        parse_financial_juice_news_items_payload(&self.config.name, &body)
    }

    /// Récupère une page backlog en isolant les corruptions JSON amont du cycle collecteur global.
    pub(in crate::features::ingestion::collectors::financial_juice::poll) async fn fetch_backlog_page_or_stop(
        &self,
        old_news_id: i64,
        phase: &str,
    ) -> Result<Option<Vec<FinancialJuiceApiNewsItem>>, AppError> {
        // Les erreurs réseau/auth doivent rester bloquantes: elles indiquent que la session ou l'amont
        // ne sont pas exploitables. Seules les pages déjà reçues mais malformées arrêtent le backlog.
        match self.fetch_previous_news_page(old_news_id).await {
            Ok(page) => Ok(Some(page)),
            Err(error @ AppError::Parse { .. }) => {
                debug!(
                    %error,
                    source = %self.config.name,
                    old_news_id,
                    phase,
                    "financial juice backlog page ignored because upstream JSON is not recoverable"
                );
                Ok(None)
            }
            Err(error) => Err(error),
        }
    }
}
