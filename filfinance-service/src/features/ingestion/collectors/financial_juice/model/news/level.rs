//! Normalisation locale du champ Level FinancialJuice.

/// Normalise le champ `Level` FinancialJuice sans inventer de taxonomie locale prematuree.
pub(in crate::features::ingestion::collectors::financial_juice::model::news) fn normalize_news_level(
    value: Option<&str>,
) -> Option<String> {
    value
        // `trim()` retire les espaces parasites tout en conservant la valeur source significative.
        .map(str::trim)
        // Une chaine vide FinancialJuice signifie "niveau absent", pas un niveau exploitable.
        .filter(|level| !level.is_empty())
        .map(ToOwned::to_owned)
}
