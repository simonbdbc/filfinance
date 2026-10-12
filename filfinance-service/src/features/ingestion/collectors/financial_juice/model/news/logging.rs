//! Facade d'observabilite des champs news FinancialJuice.

mod level;
mod priority;

pub(in crate::features::ingestion::collectors::financial_juice::model::news) use level::log_observed_news_level;
#[cfg(test)]
pub(in crate::features::ingestion) use priority::is_priority_like_unknown_value_active;
pub(in crate::features::ingestion::collectors::financial_juice::model::news) use priority::log_priority_like_unknown_fields;
