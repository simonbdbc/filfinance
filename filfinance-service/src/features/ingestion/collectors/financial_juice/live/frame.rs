//! Décodage des frames live SignalR FinancialJuice.

use serde_json::Value;

use crate::{errors::AppError, features::news::domain::NewsEvent};

use super::super::{
    FINANCIAL_JUICE_SIGNALR_HUB_NAME, FinancialJuiceSiteCollector,
    model::{FinancialJuiceApiNewsItem, normalize_financial_juice_news_items},
    signalr::FinancialJuiceSignalrEnvelope,
};

impl FinancialJuiceSiteCollector {
    /// Décode une frame live FinancialJuice et en extrait des `NewsEvent` si elle porte des news.
    pub(in crate::features::ingestion::collectors::financial_juice::live) fn extract_news_events_from_live_frame(
        &self,
        frame: &str,
    ) -> Result<Vec<NewsEvent>, AppError> {
        let trimmed = frame.trim();

        // SignalR envoie parfois des frames vides ou de heartbeat JSON minimal, non métier.
        if trimmed.is_empty() || trimmed == "{}" {
            return Ok(Vec::new());
        }

        let envelope: FinancialJuiceSignalrEnvelope = serde_json::from_str(trimmed)?;

        if let Some(error) = envelope.error {
            return Err(AppError::downstream(format!(
                "financial juice signalr server error: {error}"
            )));
        }

        let mut items = Vec::new();

        for message in envelope.messages {
            // `as_deref()` transforme `Option<String>` en `Option<&str>` pour comparer sans allouer.
            let Some(hub) = message.hub.as_deref() else {
                continue;
            };
            let Some(method) = message.method.as_deref() else {
                continue;
            };

            // On ignore tout ce qui n'est pas l'invocation news attendue afin de garder le parser robuste
            // si SignalR transporte un jour d'autres méthodes applicatives.
            if !hub.eq_ignore_ascii_case(FINANCIAL_JUICE_SIGNALR_HUB_NAME)
                || !method.eq_ignore_ascii_case("sendupdates")
            {
                continue;
            }

            // Le site sérialise encore la liste de news comme une chaîne JSON dans `args[0]`;
            // on valide donc explicitement `Value::String` avant le `from_str`.
            let Some(Value::String(payload)) = message.args.first() else {
                continue;
            };
            let live_items: Vec<FinancialJuiceApiNewsItem> =
                serde_json::from_str(payload.as_str())?;
            items.extend(normalize_financial_juice_news_items(
                &self.config.name,
                live_items,
            ));
        }

        Ok(items)
    }
}
