//! Backfill de demarrage FinancialJuice limite a J et J-1.

mod page;
mod window;

// OffsetDateTime sert a figer le moment de reference du backfill au demarrage.
use time::OffsetDateTime;

use crate::{errors::AppError, features::news::domain::NewsEvent};

use self::{page::process_startup_backfill_page, window::StartupBackfillWindow};
use super::super::super::{
    FINANCIAL_JUICE_OUTSIDE_WINDOW_PAGE_STREAK_LIMIT, FinancialJuiceSiteCollector,
    model::{FinancialJuiceBackfillTimestampCursor, sort_and_dedup_events_chronologically},
};

impl FinancialJuiceSiteCollector {
    /// Recharge les pages `J` et `J-1` au demarrage, puis s'arrete des que la fenetre est depassee.
    pub async fn fetch_startup_backfill(&self) -> Result<Vec<NewsEvent>, AppError> {
        // L'API FinancialJuice raisonne avec un offset type navigateur; on le recompose d'abord
        // en `UtcOffset` pour pouvoir comparer des dates metier coherentes.
        let window =
            StartupBackfillWindow::from_time_offset_minutes(self.config.time_offset_minutes);
        // `GetPreviousNews` renvoie actuellement des heures UTC correctes mais une date de jour
        // parfois stale. On recalcule donc la composante jour a partir du "maintenant" UTC.
        let mut timestamp_cursor =
            FinancialJuiceBackfillTimestampCursor::new(OffsetDateTime::now_utc());
        // `oldest_news_id` est le curseur applicatif de `GetPreviousNews`: plus il descend, plus
        // on remonte loin dans l'historique.
        let mut oldest_news_id = 0_i64;
        // On memorise le dernier curseur pour detecter une pagination bloquee qui renverrait en
        // boucle la meme page, ce qui eviterait une boucle infinie au demarrage.
        let mut previous_oldest_news_id = None;
        // Le lot retenu reste mutable car on l'alimente page apres page avant tri final.
        let mut retained_items = Vec::new();
        // On exige au moins deux pages totalement hors fenetre avant de couper, pour tolerer
        // un ordre imparfait ou un leger melange d'items en bord de pagination.
        let mut consecutive_outside_window_pages = 0_usize;

        // La borne explicite de pages garde le bootstrap fini et controle meme si l'amont repond mal.
        for _ in 0..self.config.startup_backfill_page_limit {
            let Some(page) = self
                .fetch_backlog_page_or_stop(oldest_news_id, "startup_backfill")
                .await?
            else {
                break;
            };

            // Une page vide signifie qu'il n'y a plus rien a remonter cote amont.
            if page.is_empty() {
                break;
            }

            let page_result = process_startup_backfill_page(
                &self.config.name,
                page,
                &mut timestamp_cursor,
                &window,
            );
            retained_items.extend(page_result.retained_items);

            // On recupere le plus petit `news_id` de la page pour paginer vers le passe au prochain tour.
            // `let Some(...) else` evite un `unwrap()` si l'amont renvoyait une structure incoherente.
            let Some(current_oldest_news_id) = page_result.current_oldest_news_id else {
                break;
            };

            // Si le curseur n'avance plus, on coupe pour eviter de retelecharger indefiniment la meme page.
            if Some(current_oldest_news_id) == previous_oldest_news_id {
                break;
            }

            previous_oldest_news_id = Some(current_oldest_news_id);
            oldest_news_id = current_oldest_news_id;

            if page_result.contains_older_than_window && page_result.retained_in_current_page == 0 {
                consecutive_outside_window_pages += 1;
            } else {
                consecutive_outside_window_pages = 0;
            }

            // On ne coupe qu'apres avoir vu plusieurs pages integralement hors fenetre, pour
            // eviter de perdre des items valides si l'amont melange legerement les bordures.
            if consecutive_outside_window_pages >= FINANCIAL_JUICE_OUTSIDE_WINDOW_PAGE_STREAK_LIMIT
            {
                break;
            }
        }

        Ok(sort_and_dedup_events_chronologically(retained_items))
    }
}
