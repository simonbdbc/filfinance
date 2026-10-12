//! DTO et mapping canonique FinancialJuice.

// `common` regroupe les helpers partagés par les formats API FinancialJuice.
mod common;
// `home` reserve le parsing /home et le bootstrap authentifie.
mod home;
// `news` reserve les DTO et conversions du flux news FinancialJuice.
mod news;

pub(super) use common::{
    event_business_date_for_offset, sort_and_dedup_events_chronologically,
    time_offset_to_utc_offset,
};
pub(super) use home::FinancialJuiceBootstrap;
pub(in crate::features::ingestion) use home::{FinancialJuiceHomePageState, parse_home_page_state};
pub(in crate::features::ingestion) use news::{
    FinancialJuiceApiNewsItem, FinancialJuiceBackfillTimestampCursor,
    normalize_financial_juice_history_items, normalize_financial_juice_news_items,
    parse_financial_juice_news_items_payload,
};

// Ces parsers restent exposés uniquement aux tests de régression FinancialJuice.
#[cfg(test)]
pub(in crate::features::ingestion) use common::{
    extract_asmx_string_payload, parse_financial_juice_time_of_day,
};
#[cfg(test)]
pub(in crate::features::ingestion) use home::extract_js_number_list;
#[cfg(test)]
pub(in crate::features::ingestion) use news::is_priority_like_unknown_value_active;
