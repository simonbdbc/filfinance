//! Retraitement controle des news deja stockees apres mise a jour blacklist.

use time::{Date, OffsetDateTime};

use crate::{
    app_state::AppState,
    errors::AppError,
    features::{
        blacklist::{
            domain::{
                BLACKLIST_ACTION_REJECT_NEWS, BLACKLIST_ACTION_STRIP_CONTENT,
                NewsBlacklistListParams,
            },
            matcher::find_news_blacklist_match,
        },
        glossary::domain::{GlossaryEntry, GlossaryListParams},
        news::domain::{EventKind, NewsEvent, PersistedNewsEvent},
        translation::{
            provider::TranslationProvider, service::translate_and_store_item_with_glossary,
        },
    },
    shared::time::europe_paris_day_for_utc,
};

/// Rapport retourne par la commande de retraitement blacklist.
#[derive(Debug, serde::Serialize)]
pub(crate) struct NewsBlacklistReapplyReport {
    pub local_day: String,
    pub local_days: Vec<String>,
    pub targeted_total: usize,
    pub rejected_total: usize,
    pub stripped_total: usize,
    pub reprocessed_total: usize,
    pub errors: Vec<String>,
}

/// Applique les regles blacklist actives aux news FinancialJuice de la table retenue J/J-1.
pub(crate) async fn reapply_current_day_blacklist(
    state: &AppState,
) -> Result<NewsBlacklistReapplyReport, AppError> {
    let now_utc = OffsetDateTime::now_utc();
    let local_days = target_local_days(now_utc);
    let local_day_strings = local_day_strings(&local_days);
    let entries = state
        .news
        .list_news_blacklist_entries(&NewsBlacklistListParams::active_for_pipeline())
        .await?;
    let glossary_entries = load_active_glossary_entries(state).await?;
    let candidates = state
        .news
        .list_news_matching_active_blacklist(Some("financial_juice"), 2_000)
        .await?;
    let mut provider = TranslationProvider::from_config(&state.config.translation);
    let mut report = NewsBlacklistReapplyReport {
        local_day: local_day_strings.first().cloned().unwrap_or_default(),
        local_days: local_day_strings,
        targeted_total: 0,
        rejected_total: 0,
        stripped_total: 0,
        reprocessed_total: 0,
        errors: Vec::new(),
    };

    for item in candidates {
        let event = news_event_from_persisted(&item);
        let Some(matched) = find_news_blacklist_match(&event, &entries) else {
            continue;
        };

        report.targeted_total += 1;

        match matched.hit.action.as_str() {
            BLACKLIST_ACTION_REJECT_NEWS => {
                match state.news.insert_news_blacklist_hit(&matched.hit).await {
                    Ok(_) => {
                        if let Err(error) = state
                            .news
                            .delete_news_event_for_blacklist_reapply(item.id)
                            .await
                        {
                            report.errors.push(format!(
                                "news_event_id {} suppression impossible: {error}",
                                item.id
                            ));
                        } else {
                            report.rejected_total += 1;
                        }
                    }
                    Err(error) => report.errors.push(format!(
                        "news_event_id {} audit blacklist impossible: {error}",
                        item.id
                    )),
                }
            }
            BLACKLIST_ACTION_STRIP_CONTENT => {
                match state.news.insert_news_blacklist_hit(&matched.hit).await {
                    Ok(_) => {
                        if let Err(error) = state
                            .news
                            .strip_news_event_content_for_blacklist_reapply(
                                item.id,
                                &state.config.translation.target_language,
                            )
                            .await
                        {
                            report.errors.push(format!(
                                "news_event_id {} retrait contenu impossible: {error}",
                                item.id
                            ));
                            continue;
                        }

                        report.stripped_total += 1;
                        match state.news.get_news_by_id(item.id).await? {
                            Some(stripped_item) => {
                                if translate_and_store_item_with_glossary(
                                    state,
                                    &mut provider,
                                    &stripped_item,
                                    false,
                                    &glossary_entries,
                                )
                                .await
                                {
                                    report.reprocessed_total += 1;
                                } else {
                                    report.errors.push(format!(
                                        "news_event_id {} traduction non retraitee",
                                        item.id
                                    ));
                                }
                            }
                            None => report.errors.push(format!(
                                "news_event_id {} introuvable apres retrait contenu",
                                item.id
                            )),
                        }
                    }
                    Err(error) => report.errors.push(format!(
                        "news_event_id {} audit blacklist impossible: {error}",
                        item.id
                    )),
                }
            }
            other => report.errors.push(format!(
                "news_event_id {} action blacklist inconnue: {other}",
                item.id
            )),
        }
    }

    Ok(report)
}

async fn load_active_glossary_entries(state: &AppState) -> Result<Vec<GlossaryEntry>, AppError> {
    let entries = state
        .news
        .list_glossary_entries(&GlossaryListParams::active_for_pipeline())
        .await?;
    state
        .translation_runtime
        .record_glossary_loaded(entries.len() as u64);
    state.translation_runtime.clear_glossary_error();
    Ok(entries)
}

fn news_event_from_persisted(item: &PersistedNewsEvent) -> NewsEvent {
    NewsEvent {
        source: item.source.clone(),
        event_kind: event_kind_from_persisted(&item.event_kind),
        title: item.title.clone(),
        url: item.url.clone(),
        published_at: item.published_at.clone(),
        event_time: item.event_time.clone(),
        ingested_at: item.ingested_at.clone(),
        body: item.body.clone(),
        news_level: item.news_level.clone(),
        raw_payload_ref: item.raw_payload_ref.clone(),
        macro_metadata: None,
        signature: item.signature.clone(),
    }
}

fn event_kind_from_persisted(value: &str) -> EventKind {
    match value {
        "macro_calendar" => EventKind::MacroCalendar,
        _ => EventKind::Headline,
    }
}

fn target_local_days(now_utc: OffsetDateTime) -> Vec<Date> {
    let current_day = europe_paris_day_for_utc(now_utc).unwrap_or_else(|| now_utc.date());
    let mut days = vec![current_day];
    if let Some(previous_day) = current_day.previous_day() {
        days.push(previous_day);
    }
    days
}

fn local_day_strings(local_days: &[Date]) -> Vec<String> {
    local_days.iter().map(ToString::to_string).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::format_description::well_known::Rfc3339;

    use crate::features::{
        blacklist::domain::{
            BLACKLIST_MATCH_MODE_PARTIAL, BLACKLIST_TARGET_SCOPE_CONTENT,
            BLACKLIST_TARGET_SCOPE_TITLE, BLACKLIST_TARGET_SCOPE_TITLE_CONTENT,
            CreateNewsBlacklistEntryRequest, NewsAuditListParams, NewsBlacklistEntry,
        },
        blacklist::matcher::find_news_blacklist_match,
        news::domain::{DailyNewsQualificationDraft, NewsEvent},
        test_support::{app_state, seed_headline},
        translation::domain::NewsTranslationDraft,
    };

    #[tokio::test]
    async fn reapply_blacklist_title_rule_rejects_stored_news_and_linked_rows() {
        let state = app_state().await;
        let event = seed_headline(&state, "financial_juice", "Blocked headline title").await;
        let translation = state
            .news
            .upsert_translation(
                &event,
                &NewsTranslationDraft {
                    language: "fr".to_owned(),
                    status: "ready".to_owned(),
                    translated_title: "Titre bloque".to_owned(),
                    translated_body: None,
                    narration_text: "10:00. Titre bloque.".to_owned(),
                    translation_mode: "legacy".to_owned(),
                    error_message: None,
                },
            )
            .await
            .expect("translation should be inserted");
        let local_day = current_local_day();
        state
            .news
            .upsert_daily_news_qualification(&DailyNewsQualificationDraft {
                local_day: local_day.clone(),
                news_event_id: event.id,
                translation_id: Some(translation.id),
                category: "macro".to_owned(),
                importance: "high".to_owned(),
                summary: "summary".to_owned(),
                reasoning: "reasoning".to_owned(),
                confidence: Some(0.9),
                is_important: true,
                provider: Some("test".to_owned()),
                model: Some("test".to_owned()),
            })
            .await
            .expect("qualification should be inserted");
        create_blacklist_entry(&state, "Blocked headline", BLACKLIST_TARGET_SCOPE_TITLE).await;

        let report = reapply_current_day_blacklist(&state)
            .await
            .expect("blacklist reapply should succeed");

        assert_eq!(report.targeted_total, 1);
        assert_eq!(report.rejected_total, 1);
        assert_eq!(report.stripped_total, 0);
        assert!(report.errors.is_empty());
        assert!(
            state
                .news
                .get_news_by_id(event.id)
                .await
                .expect("news read should succeed")
                .is_none()
        );
        assert!(
            state
                .news
                .get_translation_by_news_event_id_and_language(event.id, "fr")
                .await
                .expect("translation read should succeed")
                .is_none()
        );
        assert!(
            state
                .news
                .list_daily_news_qualifications(&local_day)
                .await
                .expect("qualification list should succeed")
                .is_empty()
        );
    }

    #[tokio::test]
    async fn reapply_blacklist_title_content_body_match_rejects_stored_news() {
        let state = app_state().await;
        let event = insert_body_news(
            &state,
            "Neutral title",
            "This content includes the blocked body phrase.",
        )
        .await;
        create_blacklist_entry(
            &state,
            "blocked body phrase",
            BLACKLIST_TARGET_SCOPE_TITLE_CONTENT,
        )
        .await;

        let report = reapply_current_day_blacklist(&state)
            .await
            .expect("blacklist reapply should succeed");

        assert_eq!(report.targeted_total, 1);
        assert_eq!(report.rejected_total, 1);
        assert!(
            state
                .news
                .get_news_by_id(event.id)
                .await
                .expect("news read should succeed")
                .is_none()
        );
    }

    #[tokio::test]
    async fn reapply_blacklist_title_rule_rejects_previous_day_news() {
        let state = app_state().await;
        let event = insert_news_with_published_at(
            &state,
            "Previous day blocked headline",
            None,
            previous_local_day_noon_rfc3339(),
        )
        .await;
        create_blacklist_entry(&state, "Previous day blocked", BLACKLIST_TARGET_SCOPE_TITLE).await;

        let report = reapply_current_day_blacklist(&state)
            .await
            .expect("blacklist reapply should succeed");

        assert_eq!(report.local_days.len(), 2);
        assert_eq!(report.targeted_total, 1);
        assert_eq!(report.rejected_total, 1);
        assert!(report.errors.is_empty());
        assert!(
            state
                .news
                .get_news_by_id(event.id)
                .await
                .expect("news read should succeed")
                .is_none()
        );
    }

    #[tokio::test]
    async fn reapply_blacklist_content_rule_strips_body_and_retranslates_once() {
        let state = app_state().await;
        let event = insert_body_news(
            &state,
            "Market remains open",
            "This content includes an embargoed paragraph.",
        )
        .await;
        state
            .news
            .upsert_translation(
                &event,
                &NewsTranslationDraft {
                    language: "fr".to_owned(),
                    status: "ready".to_owned(),
                    translated_title: "Le marche reste ouvert".to_owned(),
                    translated_body: Some("Ce contenu contient un paragraphe interdit.".to_owned()),
                    narration_text: "10:00. Le marche reste ouvert.".to_owned(),
                    translation_mode: "legacy".to_owned(),
                    error_message: None,
                },
            )
            .await
            .expect("translation should be inserted");
        create_blacklist_entry(
            &state,
            "embargoed paragraph",
            BLACKLIST_TARGET_SCOPE_CONTENT,
        )
        .await;

        let first_report = reapply_current_day_blacklist(&state)
            .await
            .expect("blacklist reapply should succeed");
        let stored = state
            .news
            .get_news_by_id(event.id)
            .await
            .expect("news read should succeed")
            .expect("news should remain stored");
        let translation = state
            .news
            .get_translation_by_news_event_id_and_language(event.id, "fr")
            .await
            .expect("translation read should succeed")
            .expect("translation should remain stored");
        let hits = state
            .news
            .list_news_blacklist_hits(&NewsAuditListParams { limit: 10 })
            .await
            .expect("hits should be readable");

        assert_eq!(first_report.targeted_total, 1);
        assert_eq!(first_report.rejected_total, 0);
        assert_eq!(first_report.stripped_total, 1);
        assert_eq!(first_report.reprocessed_total, 1);
        assert!(first_report.errors.is_empty());
        assert!(stored.body.is_none());
        assert!(translation.source_body.is_none());
        assert!(translation.translated_body.is_none());
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].action, BLACKLIST_ACTION_STRIP_CONTENT);

        let second_report = reapply_current_day_blacklist(&state)
            .await
            .expect("second blacklist reapply should succeed");
        let second_hits = state
            .news
            .list_news_blacklist_hits(&NewsAuditListParams { limit: 10 })
            .await
            .expect("hits should be readable");

        assert_eq!(second_report.targeted_total, 0);
        assert_eq!(second_report.stripped_total, 0);
        assert_eq!(second_hits.len(), 1);
    }

    #[tokio::test]
    async fn reapply_blacklist_content_rule_does_not_skip_stale_strip_hit() {
        let state = app_state().await;
        let trading_view_body = r#"new TradingView.chart({width:"100%",height:"400",chart:"YBv5p8Am",container_id:"desc_{{NewsID}}"});"#;
        let event = insert_body_news(
            &state,
            "Iran's Foreign Minister Araghchi: The Islamabad Memorandum of Understanding has never been closer - Post on X.",
            trading_view_body,
        )
        .await;
        state
            .news
            .upsert_translation(
                &event,
                &NewsTranslationDraft {
                    language: "fr".to_owned(),
                    status: "ready".to_owned(),
                    translated_title: "Le ministre iranien des Affaires etrangeres Araghchi"
                        .to_owned(),
                    translated_body: Some(
                        r#"nouveau TradingView.chart({width:"100%",height:"400"});"#.to_owned(),
                    ),
                    narration_text: "16:52. Le ministre iranien des Affaires etrangeres Araghchi."
                        .to_owned(),
                    translation_mode: "heuristic_fr".to_owned(),
                    error_message: None,
                },
            )
            .await
            .expect("translation should be inserted");
        let entry =
            create_blacklist_entry(&state, "TradingView.chart", BLACKLIST_TARGET_SCOPE_CONTENT)
                .await;
        let matched = find_news_blacklist_match(&news_event_from_persisted(&event), &[entry])
            .expect("stale content should match blacklist");
        state
            .news
            .insert_news_blacklist_hit(&matched.hit)
            .await
            .expect("stale strip_content audit should be inserted");

        let report = reapply_current_day_blacklist(&state)
            .await
            .expect("blacklist reapply should succeed");
        let stored = state
            .news
            .get_news_by_id(event.id)
            .await
            .expect("news read should succeed")
            .expect("news should remain stored");
        let translation = state
            .news
            .get_translation_by_news_event_id_and_language(event.id, "fr")
            .await
            .expect("translation read should succeed")
            .expect("translation should remain stored");

        assert_eq!(report.targeted_total, 1);
        assert_eq!(report.stripped_total, 1);
        assert_eq!(report.reprocessed_total, 1);
        assert!(report.errors.is_empty());
        assert!(stored.body.is_none());
        assert!(translation.source_body.is_none());
        assert!(translation.translated_body.is_none());
    }

    async fn create_blacklist_entry(
        state: &AppState,
        term: &str,
        target_scope: &str,
    ) -> NewsBlacklistEntry {
        let normalized = CreateNewsBlacklistEntryRequest {
            term: term.to_owned(),
            match_mode: BLACKLIST_MATCH_MODE_PARTIAL.to_owned(),
            target_scope: Some(target_scope.to_owned()),
            enabled: true,
            notes: None,
        }
        .normalize()
        .expect("blacklist entry should be valid");

        state
            .news
            .create_news_blacklist_entry(&normalized)
            .await
            .expect("blacklist entry should be inserted")
    }

    async fn insert_body_news(state: &AppState, title: &str, body: &str) -> PersistedNewsEvent {
        let event = NewsEvent::new_headline(
            "financial_juice",
            title,
            format!("https://example.test/{title}"),
            None,
            Some(body.to_owned()),
        );

        state
            .news
            .insert_event(&event)
            .await
            .expect("event insert should succeed")
            .expect("event should be stored")
    }

    async fn insert_news_with_published_at(
        state: &AppState,
        title: &str,
        body: Option<String>,
        published_at: String,
    ) -> PersistedNewsEvent {
        let event = NewsEvent::new_headline(
            "financial_juice",
            title,
            format!("https://example.test/{title}"),
            Some(published_at),
            body,
        );

        state
            .news
            .insert_event(&event)
            .await
            .expect("event insert should succeed")
            .expect("event should be stored")
    }

    fn previous_local_day_noon_rfc3339() -> String {
        let now_utc = OffsetDateTime::now_utc();
        let current_day = europe_paris_day_for_utc(now_utc).unwrap_or_else(|| now_utc.date());
        let previous_day = current_day
            .previous_day()
            .expect("current day should have a previous day");

        previous_day
            .with_hms(12, 0, 0)
            .expect("noon should be valid")
            .assume_utc()
            .format(&Rfc3339)
            .expect("timestamp should format")
    }

    fn current_local_day() -> String {
        let now_utc = OffsetDateTime::now_utc();
        europe_paris_day_for_utc(now_utc)
            .unwrap_or_else(|| now_utc.date())
            .to_string()
    }
}
