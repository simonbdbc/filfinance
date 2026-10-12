//! DTO news FinancialJuice et mapping canonique.

mod item;
mod level;
mod logging;
mod normalize;
mod parser;
mod timestamp;

// La facade garde les imports historiques du collecteur FinancialJuice.
pub(in crate::features::ingestion) use item::FinancialJuiceApiNewsItem;
#[cfg(test)]
pub(in crate::features::ingestion) use logging::is_priority_like_unknown_value_active;
pub(in crate::features::ingestion) use normalize::{
    normalize_financial_juice_history_items, normalize_financial_juice_news_items,
};
pub(in crate::features::ingestion) use parser::parse_financial_juice_news_items_payload;
pub(in crate::features::ingestion) use timestamp::FinancialJuiceBackfillTimestampCursor;
