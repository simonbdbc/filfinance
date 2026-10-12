//! Facade des helpers communs aux formats FinancialJuice.

mod chronology;
mod text;
mod time;
mod url;
mod xml;

// Les helpers URL restent visibles dans toute la feature FinancialJuice.
pub(in crate::features::ingestion::collectors::financial_juice) use url::absolutize_url_from_site;
// Les helpers texte restent proches des DTO news.
pub(in crate::features::ingestion::collectors::financial_juice) use text::{
    non_empty, normalize_description, normalize_title,
};
// Les helpers temps portent les timestamps source et les fenêtres métier FinancialJuice.
pub(in crate::features::ingestion::collectors::financial_juice) use time::{
    event_business_date_for_offset, normalize_timestamp, time_offset_to_utc_offset,
};
// Le tri/déduplication du backlog reste un détail du collecteur site.
pub(in crate::features::ingestion::collectors::financial_juice) use chronology::sort_and_dedup_events_chronologically;

// Ces parsers sont aussi exposés aux tests d'intégration de la feature ingestion.
pub(in crate::features::ingestion) use time::parse_financial_juice_time_of_day;
pub(in crate::features::ingestion) use xml::extract_asmx_string_payload;
