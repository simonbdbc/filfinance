//! Observation des valeurs `Level` mappees dans les payloads FinancialJuice.

// Les logs structurés documentent les champs de priorité encore non contractualisés.
use tracing::{debug, info};

use super::super::{item::FinancialJuiceApiNewsItem, level::normalize_news_level};

/// Logue les valeurs `Level` observées maintenant que le champ est mappé explicitement.
pub(in crate::features::ingestion::collectors::financial_juice::model::news) fn log_observed_news_level(
    source_name: &str,
    transport_name: &str,
    item: &FinancialJuiceApiNewsItem,
) {
    let Some(raw_level) = item.level.as_deref() else {
        // L'absence du champ reste un cas valide pour les anciennes reponses FinancialJuice.
        return;
    };
    // On applique la meme normalisation que le modele canonique pour comparer les valeurs exploitables.
    let normalized_level = normalize_news_level(Some(raw_level));

    // Le debug conserve la valeur brute et sa normalisation pour constituer l'inventaire runtime.
    debug!(
        source = %source_name,
        transport = %transport_name,
        news_id = item.news_id,
        iid = ?item.iid,
        raw_news_level = %raw_level,
        news_level = ?normalized_level,
        "financial juice news level observed"
    );

    // `if let ... &&` combine le depliage de l'Option et le filtre de valeurs connues en une garde lisible.
    if let Some(level) = normalized_level.as_deref()
        && !matches!(level, "active active-critical" | "news-general")
    {
        // Le niveau `info` rend visible toute nouvelle valeur exploitable sans activer tous les logs debug.
        info!(
            source = %source_name,
            transport = %transport_name,
            news_id = item.news_id,
            iid = ?item.iid,
            news_level = %level,
            "financial juice news level is not documented yet"
        );
    }
}
