//! Traitement d'une page du backfill startup FinancialJuice.

use crate::features::{
    ingestion::collectors::financial_juice::{
        model::{
            FinancialJuiceApiNewsItem, FinancialJuiceBackfillTimestampCursor,
            normalize_financial_juice_history_items,
        },
        poll::backlog::startup::window::{StartupBackfillWindow, StartupWindowMatch},
    },
    news::domain::NewsEvent,
};

/// Resultat intermediaire d'une page `GetPreviousNews` pendant le bootstrap.
pub(super) struct StartupBackfillPageResult {
    /// Evenements conserves dans J/J-1.
    pub(super) retained_items: Vec<NewsEvent>,
    /// Plus petit `news_id` de la page, utilise comme curseur vers le passe.
    pub(super) current_oldest_news_id: Option<i64>,
    /// Indique que la page a franchi la borne plus vieille que J-1.
    pub(super) contains_older_than_window: bool,
    /// Nombre d'items reellement gardes dans cette page.
    pub(super) retained_in_current_page: usize,
}

/// Normalise et classe une page brute dans la fenetre de demarrage.
pub(super) fn process_startup_backfill_page(
    source_name: &str,
    page: Vec<FinancialJuiceApiNewsItem>,
    timestamp_cursor: &mut FinancialJuiceBackfillTimestampCursor,
    window: &StartupBackfillWindow,
) -> StartupBackfillPageResult {
    // On calcule le curseur avant de consommer la page brute dans la normalisation.
    let current_oldest_news_id = page.iter().map(|item| item.news_id).min();
    let normalized = normalize_financial_juice_history_items(source_name, page, timestamp_cursor);

    let mut retained_items = Vec::new();
    // Ce flag dit "on a deja franchi la borne J-1, inutile de demander la page suivante".
    let mut contains_older_than_window = false;
    let mut retained_in_current_page = 0_usize;

    for event in normalized {
        match window.classify(&event) {
            StartupWindowMatch::Retained => {
                retained_items.push(event);
                retained_in_current_page += 1;
            }
            StartupWindowMatch::OlderThanWindow => {
                // Des qu'une page contient un item plus vieux que J-1, on sait qu'on a atteint
                // la fin utile du bootstrap, meme si quelques items de la page etaient encore valides.
                contains_older_than_window = true;
            }
            StartupWindowMatch::Ignored => {}
        }
    }

    StartupBackfillPageResult {
        retained_items,
        current_oldest_news_id,
        contains_older_than_window,
        retained_in_current_page,
    }
}
