//! Rattrapage des traductions manquantes au demarrage du worker.

// OffsetDateTime sert ici aux calculs de jours UTC du rattrapage de traduction.
use time::OffsetDateTime;

use crate::{
    app_state::AppState, errors::AppError, features::translation::provider::TranslationProvider,
};

use super::processing::translate_and_store_item;

/// Retourne les deux jours UTC les plus recents utilises pour le rattrapage de traduction.
fn recent_utc_day_keys() -> Vec<String> {
    let today = OffsetDateTime::now_utc().date();
    let yesterday = today.previous_day().unwrap_or(today);

    vec![today.to_string(), yesterday.to_string()]
}

/// Traduit au demarrage les news deja persistees mais encore sans variante FR.
pub(super) async fn backfill_recent_missing_translations(
    state: &AppState,
    provider: &mut TranslationProvider,
) -> Result<usize, AppError> {
    let missing_items = state
        .news
        .list_news_missing_translation_for_days(
            Some("financial_juice"),
            &state.config.translation.target_language,
            &recent_utc_day_keys(),
            2_000,
        )
        .await?;

    let backfilled_count = missing_items.len();

    for item in missing_items {
        translate_and_store_item(state, provider, &item, false).await;
    }

    Ok(backfilled_count)
}
