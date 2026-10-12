//! Parsing strict puis tolerant des payloads GetPreviousNews FinancialJuice.

mod lenient;

use tracing::debug;

use crate::errors::AppError;

use lenient::parse_financial_juice_news_items_lenient;

use super::{super::common::extract_asmx_string_payload, item::FinancialJuiceApiNewsItem};

/// Parse une page `GetPreviousNews` en tolerant une ligne JSON amont corrompue.
pub(in crate::features::ingestion) fn parse_financial_juice_news_items_payload(
    source_name: &str,
    body: &str,
) -> Result<Vec<FinancialJuiceApiNewsItem>, AppError> {
    // On garde l'extraction XML commune: seul le parsing du tableau metier devient tolerant.
    let payload = extract_asmx_string_payload(source_name, body)?;

    // Le chemin nominal reste strict pour ne pas masquer une derive globale du contrat FJ.
    match serde_json::from_str::<Vec<FinancialJuiceApiNewsItem>>(&payload) {
        Ok(items) => Ok(items),
        Err(error) => {
            // Le fallback scanne les objets top-level et parse chaque objet separement.
            let (items, skipped_count) = parse_financial_juice_news_items_lenient(&payload);

            if items.is_empty() {
                return Err(AppError::parse(
                    source_name.to_owned(),
                    format!("failed to parse FinancialJuice JSON payload: {error}"),
                ));
            }

            // Une page partiellement corrompue est un fait d'observation amont, pas un incident service.
            debug!(
                source = %source_name,
                strict_error = %error,
                recovered_count = items.len(),
                skipped_count,
                "financial juice backlog payload recovered from malformed JSON"
            );

            Ok(items)
        }
    }
}
