//! Tests HTTP et runtime de la feature traduction.

// Les extracteurs Axum permettent d'appeler le handler runtime sans serveur HTTP.
use axum::{
    Json,
    extract::{Path, Query, State},
};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use crate::{
    config::TranslationConfig,
    features::{
        glossary::domain::{
            CreateGlossaryEntryRequest, GLOSSARY_SCOPE_FULL_PIPELINE, GlossaryEntry,
            GlossaryListParams, GlossaryListQuery, GlossaryPreviewRequest, MATCH_MODE_PATTERN,
            MATCH_MODE_WORD_BOUNDARY,
        },
        glossary::http::{preview_glossary_text, reapply_glossary_translations},
        news::domain::{NewsEvent, PersistedNewsEvent},
        test_support::{app_state, seed_headline, seed_test_run_headline},
        translation::{
            domain::{
                NewsTranslationDraft, TranslationHistoryParams, TranslationHistoryScope,
                TranslationRequest,
            },
            http::{
                TranslationHistoryQuery, get_translation_detail, list_translations_history,
                translation_runtime,
            },
            provider::{
                HEURISTIC_PROVIDER, LOCAL_COMMAND_SESSION_PROVIDER, OPUS_MT_LOCAL_COMMAND_PROVIDER,
                TranslationProvider,
            },
            providers::{
                heuristic::{
                    HeuristicTranslationProvider, heuristic_translate_text, protect_finance_tokens,
                },
                local_command::translation_command_args_for_profile,
            },
            service::build_translation_draft,
        },
    },
};

#[tokio::test]
async fn translation_runtime_returns_a_serializable_snapshot() {
    let state = app_state().await;
    state.translation_runtime.record_received();
    state
        .translation_runtime
        .record_success("opus_mt_tc_big_local_session", 123);

    let Json(payload) = translation_runtime(State(state)).await;

    assert_eq!(payload["translation"]["provider"], "heuristic_fr");
    assert_eq!(payload["translation"]["profile"], "quality");
    assert_eq!(payload["translation"]["received_total"], 1);
    assert_eq!(payload["translation"]["translated_total"], 1);
    assert_eq!(payload["translation"]["pending_total"], 0);
    assert_eq!(payload["translation"]["translation_duration_p50_ms"], 123);
    assert_eq!(payload["translation"]["translation_duration_p95_ms"], 123);
    assert_eq!(
        payload["translation"]["last_translation_mode"],
        "opus_mt_tc_big_local_session"
    );
}

#[tokio::test]
async fn list_translations_history_separates_real_feed_and_test_runs() {
    // Ce test couvre l'API HTTP consommee par le client pour l'historique journalier.
    let state = app_state().await;
    let real_event = seed_headline(&state, "financial_juice", "real headline").await;
    let test_event = seed_test_run_headline(&state, "financial_juice", "test headline").await;

    state
        .news
        .upsert_translation(
            &real_event,
            &NewsTranslationDraft {
                language: "fr".to_owned(),
                status: "ready".to_owned(),
                translated_title: "Titre reel".to_owned(),
                translated_body: None,
                narration_text: "10:00. Titre reel.".to_owned(),
                translation_mode: "opus_mt_tc_big_local_session".to_owned(),
                error_message: None,
            },
        )
        .await
        .expect("real translation upsert should succeed");
    state
        .news
        .upsert_translation(
            &test_event,
            &NewsTranslationDraft {
                language: "fr".to_owned(),
                status: "ready".to_owned(),
                translated_title: "Titre test".to_owned(),
                translated_body: None,
                narration_text: "10:01. Titre test.".to_owned(),
                translation_mode: "opus_mt_tc_big_local_session".to_owned(),
                error_message: None,
            },
        )
        .await
        .expect("test translation upsert should succeed");

    let Json(real_feed_response) = list_translations_history(
        State(state.clone()),
        Query(TranslationHistoryQuery {
            limit: Some(10),
            source: Some("financial_juice".to_owned()),
            day: None,
            history_scope: Some("real_feed".to_owned()),
            before_id: None,
        }),
    )
    .await
    .expect("real feed history should succeed");
    let Json(test_run_response) = list_translations_history(
        State(state),
        Query(TranslationHistoryQuery {
            limit: Some(10),
            source: Some("financial_juice".to_owned()),
            day: None,
            history_scope: Some("test_run".to_owned()),
            before_id: None,
        }),
    )
    .await
    .expect("test-run history should succeed");

    assert_eq!(real_feed_response.items.len(), 1);
    assert!(real_feed_response.items[0].test_run_id.is_none());
    assert_eq!(test_run_response.items.len(), 1);
    assert_eq!(
        test_run_response.items[0].test_run_id.as_deref(),
        Some("run-api-test")
    );
}

#[tokio::test]
async fn list_translations_history_returns_complete_texts() {
    // L'historique HTTP alimente les vues admin et ne doit pas heriter de la limite live WS.
    let state = app_state().await;
    let source_tail = "SOURCE_HISTORY_TAIL_MARKER";
    let translated_tail = "TRANSLATED_HISTORY_TAIL_MARKER";
    let narration_tail = "NARRATION_HISTORY_TAIL_MARKER";
    let source_body = format!("{} {source_tail}", "English source body. ".repeat(80));
    let translated_body = format!(
        "{} {translated_tail}",
        "Corps francais complet. ".repeat(80)
    );
    let narration_text = format!(
        "{} {narration_tail}",
        "10:00. Narration complete. ".repeat(80)
    );
    let event = NewsEvent::new_headline(
        "financial_juice",
        "Full history headline",
        "https://example.test/full-history",
        None,
        Some(source_body.clone()),
    );
    let persisted_event = state
        .news
        .insert_event(&event)
        .await
        .expect("event insert should succeed")
        .expect("event should be stored");

    state
        .news
        .upsert_translation(
            &persisted_event,
            &NewsTranslationDraft {
                language: "fr".to_owned(),
                status: "ready".to_owned(),
                translated_title: "Titre historique complet".to_owned(),
                translated_body: Some(translated_body.clone()),
                narration_text: narration_text.clone(),
                translation_mode: "opus_mt_tc_big_local_session".to_owned(),
                error_message: None,
            },
        )
        .await
        .expect("translation upsert should succeed");

    let Json(response) = list_translations_history(
        State(state),
        Query(TranslationHistoryQuery {
            limit: Some(10),
            source: Some("financial_juice".to_owned()),
            day: None,
            history_scope: Some("real_feed".to_owned()),
            before_id: None,
        }),
    )
    .await
    .expect("history should succeed");

    assert_eq!(response.items.len(), 1);
    let item = &response.items[0];
    assert_eq!(item.source_body.as_deref(), Some(source_body.as_str()));
    assert_eq!(
        item.translated_body.as_deref(),
        Some(translated_body.as_str())
    );
    assert!(item.source_body.as_deref().unwrap().contains(source_tail));
    assert!(
        item.translated_body
            .as_deref()
            .unwrap()
            .contains(translated_tail)
    );
    assert!(item.narration_text.contains(narration_tail));
}

#[tokio::test]
async fn reapply_glossary_translations_repairs_current_and_previous_day_news() {
    let state = app_state().await;
    let event = seed_headline(
        &state,
        "financial_juice",
        "ECB's Nagel says rates stay high",
    )
    .await;
    let previous_event = seed_headline_with_published_at(
        &state,
        "financial_juice",
        "ECB's Kazimir says policy remains data dependent",
        previous_local_day_noon_rfc3339(),
    )
    .await;
    let unmatched_event = seed_headline(
        &state,
        "financial_juice",
        "Fed speakers see rates unchanged",
    )
    .await;

    state
        .news
        .upsert_translation(
            &event,
            &NewsTranslationDraft {
                language: "fr".to_owned(),
                status: "ready".to_owned(),
                translated_title: "BCE's Nagel dit que les taux restent hauts".to_owned(),
                translated_body: None,
                narration_text: "10:00. BCE's Nagel dit que les taux restent hauts.".to_owned(),
                translation_mode: "legacy_before_glossary_update".to_owned(),
                error_message: None,
            },
        )
        .await
        .expect("legacy translation should be inserted");
    state
        .news
        .upsert_translation(
            &previous_event,
            &NewsTranslationDraft {
                language: "fr".to_owned(),
                status: "ready".to_owned(),
                translated_title: "Kazimir: de la BCE dit que la politique depend des donnees"
                    .to_owned(),
                translated_body: None,
                narration_text:
                    "10:00. Kazimir: de la BCE dit que la politique depend des donnees.".to_owned(),
                translation_mode: "legacy_before_glossary_update".to_owned(),
                error_message: None,
            },
        )
        .await
        .expect("previous-day legacy translation should be inserted");
    state
        .news
        .upsert_translation(
            &unmatched_event,
            &NewsTranslationDraft {
                language: "fr".to_owned(),
                status: "ready".to_owned(),
                translated_title: "Les intervenants de la Fed voient les taux inchanges".to_owned(),
                translated_body: None,
                narration_text: "10:00. Les intervenants de la Fed voient les taux inchanges."
                    .to_owned(),
                translation_mode: "legacy_before_glossary_update".to_owned(),
                error_message: None,
            },
        )
        .await
        .expect("unmatched legacy translation should be inserted");
    let Json(report) = reapply_glossary_translations(State(state.clone()))
        .await
        .expect("glossary reapply should succeed");
    let updated = state
        .news
        .get_translation_by_news_event_id_and_language(
            event.id,
            &state.config.translation.target_language,
        )
        .await
        .expect("updated translation should be readable")
        .expect("updated translation should exist");
    let previous_updated = state
        .news
        .get_translation_by_news_event_id_and_language(
            previous_event.id,
            &state.config.translation.target_language,
        )
        .await
        .expect("previous-day translation should be readable")
        .expect("previous-day translation should exist");
    let untouched = state
        .news
        .get_translation_by_news_event_id_and_language(
            unmatched_event.id,
            &state.config.translation.target_language,
        )
        .await
        .expect("unmatched translation should be readable")
        .expect("unmatched translation should exist");

    assert_eq!(report.local_days.len(), 2);
    assert_eq!(report.targeted_total, 2);
    assert_eq!(report.reprocessed_total, 0);
    assert_eq!(report.display_repaired_total, 2);
    assert!(report.errors.is_empty());
    assert!(updated.translated_title.contains("Nagel de la BCE"));
    assert!(!updated.translated_title.contains("BCE's Nagel"));
    assert!(
        previous_updated
            .translated_title
            .contains("Kazimir de la BCE")
    );
    assert!(
        !previous_updated
            .translated_title
            .contains("Kazimir: de la BCE")
    );
    assert_eq!(untouched.translation_mode, "legacy_before_glossary_update");

    let Json(second_report) = reapply_glossary_translations(State(state.clone()))
        .await
        .expect("second glossary reapply should succeed");

    assert_eq!(second_report.targeted_total, 0);
    assert_eq!(second_report.reprocessed_total, 0);
    assert_eq!(second_report.display_repaired_total, 0);
    assert!(second_report.errors.is_empty());
}

#[tokio::test]
async fn reapply_glossary_repairs_display_only_pattern_without_retranslation() {
    let state = app_state().await;
    let event = seed_headline(
        &state,
        "financial_juice",
        "NASDAQ: SpaceX remains in pre-launch status. $SPCX",
    )
    .await;
    let normalized = CreateGlossaryEntryRequest {
        source_term: "<string>:".to_owned(),
        translated_term: "<string> :".to_owned(),
        spoken_term: Some("<string> :".to_owned()),
        source_language: "en".to_owned(),
        target_language: "fr".to_owned(),
        match_mode: MATCH_MODE_PATTERN.to_owned(),
        case_sensitive: false,
        priority: 100,
        enabled: true,
        scope: None,
        notes: Some("formatage deux-points".to_owned()),
    }
    .normalize()
    .expect("display-only pattern should be valid");
    state
        .news
        .create_glossary_entry(&normalized)
        .await
        .expect("display-only glossary entry should be inserted");
    state
        .news
        .upsert_translation(
            &event,
            &NewsTranslationDraft {
                language: "fr".to_owned(),
                status: "ready".to_owned(),
                translated_title: "NASDAQ: SpaceX reste en pre-lancement.".to_owned(),
                translated_body: None,
                narration_text: "16:11. NASDAQ: SpaceX reste en pre-lancement.".to_owned(),
                translation_mode: "legacy_before_display_glossary".to_owned(),
                error_message: None,
            },
        )
        .await
        .expect("legacy translation should be inserted");

    let Json(report) = reapply_glossary_translations(State(state.clone()))
        .await
        .expect("glossary reapply should succeed");
    let updated = state
        .news
        .get_translation_by_news_event_id_and_language(
            event.id,
            &state.config.translation.target_language,
        )
        .await
        .expect("updated translation should be readable")
        .expect("updated translation should exist");

    assert_eq!(report.targeted_total, 1);
    assert_eq!(report.reprocessed_total, 0);
    assert_eq!(report.display_repaired_total, 1);
    assert!(report.errors.is_empty());
    assert!(updated.translated_title.contains("NASDAQ :"));
    assert!(updated.narration_text.contains("NASDAQ :"));
    assert_eq!(updated.translation_mode, "legacy_before_display_glossary");
}

#[tokio::test]
async fn get_translation_detail_returns_complete_texts() {
    // Le feed peut completer un item live compact via cette lecture detaillee.
    let state = app_state().await;
    let translated_tail = "TRANSLATED_DETAIL_TAIL_MARKER";
    let translated_body = format!(
        "{} {translated_tail}",
        "Detail francais complet. ".repeat(80)
    );
    let event = NewsEvent::new_headline(
        "financial_juice",
        "Detail headline",
        "https://example.test/detail",
        None,
        Some("English detail source body.".repeat(40)),
    );
    let persisted_event = state
        .news
        .insert_event(&event)
        .await
        .expect("event insert should succeed")
        .expect("event should be stored");
    let translation = state
        .news
        .upsert_translation(
            &persisted_event,
            &NewsTranslationDraft {
                language: "fr".to_owned(),
                status: "ready".to_owned(),
                translated_title: "Titre detail complet".to_owned(),
                translated_body: Some(translated_body.clone()),
                narration_text: "10:00. Titre detail complet.".to_owned(),
                translation_mode: "opus_mt_tc_big_local_session".to_owned(),
                error_message: None,
            },
        )
        .await
        .expect("translation upsert should succeed");

    let Json(item) = get_translation_detail(State(state.clone()), Path(translation.id))
        .await
        .expect("detail should succeed");

    assert_eq!(item.id, translation.id);
    assert_eq!(
        item.translated_body.as_deref(),
        Some(translated_body.as_str())
    );
    assert!(
        item.translated_body
            .as_deref()
            .unwrap()
            .contains(translated_tail)
    );

    let missing = get_translation_detail(State(state), Path(-1)).await;
    assert!(missing.is_err());
}

#[tokio::test]
async fn glossary_storage_creates_sorts_and_disables_entries() {
    // Ce test couvre le backend du CRUD glossaire sans passer par un serveur HTTP.
    let state = app_state().await;
    let low_priority = CreateGlossaryEntryRequest {
        source_term: "Fed".to_owned(),
        translated_term: "Fed".to_owned(),
        spoken_term: Some("Reserve federale americaine".to_owned()),
        source_language: "en".to_owned(),
        target_language: "fr".to_owned(),
        match_mode: MATCH_MODE_WORD_BOUNDARY.to_owned(),
        case_sensitive: false,
        priority: 1,
        enabled: true,
        scope: None,
        notes: None,
    }
    .normalize()
    .expect("low priority glossary payload should be valid");
    let high_priority = CreateGlossaryEntryRequest {
        source_term: "ECB".to_owned(),
        translated_term: "BCE".to_owned(),
        spoken_term: Some("Banque Centrale Europeenne".to_owned()),
        source_language: "en".to_owned(),
        target_language: "fr".to_owned(),
        match_mode: MATCH_MODE_WORD_BOUNDARY.to_owned(),
        case_sensitive: false,
        priority: 200,
        enabled: true,
        scope: None,
        notes: Some("banque centrale".to_owned()),
    }
    .normalize()
    .expect("high priority glossary payload should be valid");

    let fed = state
        .news
        .create_glossary_entry(&low_priority)
        .await
        .expect("low priority glossary entry should be inserted");
    let ecb = state
        .news
        .create_glossary_entry(&high_priority)
        .await
        .expect("high priority glossary entry should be inserted");
    let entries = state
        .news
        .list_glossary_entries(&GlossaryListParams::from_query(GlossaryListQuery {
            enabled: Some(true),
            q: None,
            limit: Some(10),
        }))
        .await
        .expect("glossary entries should be listed");

    let ecb_position = entries
        .iter()
        .position(|entry| entry.id == ecb.id)
        .expect("ECB entry should be listed");
    let fed_position = entries
        .iter()
        .position(|entry| entry.id == fed.id)
        .expect("Fed entry should be listed");

    assert!(ecb_position < fed_position);

    state
        .news
        .disable_glossary_entry(ecb.id)
        .await
        .expect("glossary entry should be disabled");
    let active_entries = state
        .news
        .list_glossary_entries(&GlossaryListParams::from_query(GlossaryListQuery {
            enabled: Some(true),
            q: None,
            limit: Some(10),
        }))
        .await
        .expect("active glossary entries should be listed");

    assert!(active_entries.iter().any(|entry| entry.id == fed.id));
    assert!(!active_entries.iter().any(|entry| entry.id == ecb.id));
}

#[tokio::test]
async fn glossary_storage_accepts_string_pattern_capture() {
    let state = app_state().await;
    let normalized = CreateGlossaryEntryRequest {
        source_term: "ECB's <string>".to_owned(),
        translated_term: "<string> de la BCE".to_owned(),
        spoken_term: None,
        source_language: "en".to_owned(),
        target_language: "fr".to_owned(),
        match_mode: MATCH_MODE_PATTERN.to_owned(),
        case_sensitive: false,
        priority: 100,
        enabled: true,
        scope: None,
        notes: Some("directionnel".to_owned()),
    }
    .normalize()
    .expect("string capture pattern should be valid");

    let entry = state
        .news
        .create_glossary_entry(&normalized)
        .await
        .expect("pattern glossary entry should be inserted");

    assert_eq!(entry.source_term, "ECB's <string>");
    assert_eq!(entry.translated_term, "<string> de la BCE");
    assert_eq!(entry.match_mode, MATCH_MODE_PATTERN);
}

#[tokio::test]
async fn glossary_preview_display_fr_applies_spoken_and_system_tts_rules() {
    let state = app_state().await;
    let normalized = CreateGlossaryEntryRequest {
        source_term: "ECB".to_owned(),
        translated_term: "BCE".to_owned(),
        spoken_term: Some("Banque Centrale Europeenne".to_owned()),
        source_language: "en".to_owned(),
        target_language: "fr".to_owned(),
        match_mode: MATCH_MODE_WORD_BOUNDARY.to_owned(),
        case_sensitive: false,
        priority: 100,
        enabled: true,
        scope: Some(GLOSSARY_SCOPE_FULL_PIPELINE.to_owned()),
        notes: Some("preview diction TTS".to_owned()),
    }
    .normalize()
    .expect("preview glossary payload should be valid");
    state
        .news
        .create_glossary_entry(&normalized)
        .await
        .expect("preview glossary entry should be inserted");

    let Json(preview) = preview_glossary_text(
        State(state),
        Json(GlossaryPreviewRequest {
            input_text: "14:30. BCE attend EUR/USD.".to_owned(),
            input_stage: "display_fr".to_owned(),
            draft_entry: None,
        }),
    )
    .await
    .expect("preview should be generated");

    assert_eq!(preview.display_text, "14:30. BCE attend EUR/USD.");
    assert!(preview.spoken_text.contains("Banque Centrale Europeenne"));
    assert!(
        preview
            .normalized_tts_text
            .contains("quatorze heures trente")
    );
    assert!(preview.normalized_tts_text.contains("euro dollar"));
    assert_eq!(preview.glossary_hits, 1);
    assert!(preview.warnings.is_empty());
}

#[tokio::test]
async fn glossary_preview_display_fr_applies_seeded_spoken_time_patterns() {
    let state = app_state().await;
    let Json(preview) = preview_glossary_text(
        State(state),
        Json(GlossaryPreviewRequest {
            input_text: "12h01 puis 14h00 et 09h30. 29h30 reste invalide.".to_owned(),
            input_stage: "display_fr".to_owned(),
            draft_entry: None,
        }),
    )
    .await
    .expect("preview should be generated");

    assert_eq!(
        preview.display_text,
        "12h01 puis 14h00 et 09h30. 29h30 reste invalide."
    );
    assert_eq!(
        preview.spoken_text,
        "douze heures une puis quatorze heures et neuf heures trente. 29h30 reste invalide."
    );
    assert_eq!(preview.normalized_tts_text, preview.spoken_text);
    assert_eq!(preview.glossary_hits, 3);
    assert!(preview.warnings.is_empty());
}

#[test]
fn glossary_pattern_validation_accepts_spoken_time_number_modifiers() {
    let normalized = CreateGlossaryEntryRequest {
        source_term: "<number>h<number>".to_owned(),
        translated_term: "<number>h<number>".to_owned(),
        spoken_term: Some("<number:hour> <number:minute>".to_owned()),
        source_language: "en".to_owned(),
        target_language: "fr".to_owned(),
        match_mode: MATCH_MODE_PATTERN.to_owned(),
        case_sensitive: false,
        priority: 100,
        enabled: true,
        scope: None,
        notes: None,
    }
    .normalize()
    .expect("spoken time modifiers should be valid pattern captures");

    assert_eq!(
        normalized.spoken_term.as_deref(),
        Some("<number:hour> <number:minute>")
    );
}

#[test]
fn glossary_pattern_validation_rejects_mismatched_captures() {
    let result = CreateGlossaryEntryRequest {
        source_term: "ECB's <string>".to_owned(),
        translated_term: "<number> de la BCE".to_owned(),
        spoken_term: None,
        source_language: "en".to_owned(),
        target_language: "fr".to_owned(),
        match_mode: MATCH_MODE_PATTERN.to_owned(),
        case_sensitive: false,
        priority: 100,
        enabled: true,
        scope: None,
        notes: None,
    }
    .normalize();

    assert!(result.is_err());
}

#[test]
fn translation_history_params_normalize_scope_day_and_limit() {
    // Ce test verrouille le parsing de la query d'historique journalier du client.
    let params = TranslationHistoryParams::new(
        Some(900),
        Some(" Financial_Juice ".to_owned()),
        Some("2026-04-23".to_owned()),
        Some(" TEST_RUN ".to_owned()),
        Some(12),
    );

    assert_eq!(params.limit, 500);
    assert_eq!(params.source.as_deref(), Some("financial_juice"));
    assert_eq!(params.day.as_deref(), Some("2026-04-23"));
    assert_eq!(params.history_scope, TranslationHistoryScope::TestRun);
    assert_eq!(params.before_id, Some(12));
}

fn test_translation_config() -> TranslationConfig {
    // Cette config minimale evite de dupliquer un gros montage dans chaque test.
    TranslationConfig {
        enabled: true,
        source_language: "en".to_owned(),
        target_language: "fr".to_owned(),
        provider: HEURISTIC_PROVIDER.to_owned(),
        profile: "quality".to_owned(),
        worker_count: 1,
        local_command_program: None,
        local_command_args: Vec::new(),
        local_command_timeout_ms: 5_000,
        warmup_enabled: false,
        warmup_source_text: "Federal Reserve signals rates stay higher for longer".to_owned(),
        max_body_chars: 280,
    }
}

fn session_mock_python() -> String {
    // Le faux helper parle le meme protocole JSONL que le vrai wrapper OPUS-MT.
    r#"
import json
import sys

count = 0

for line in sys.stdin:
    if not line.strip():
        continue
    count += 1
    payload = json.loads(line)
    body = payload.get("body")
    response = {
        "translated_title": f"{count}: session fr {payload['title']}",
        "translated_body": (f"{count}: session fr {body}" if body else None),
        "translation_mode": "mock_session",
    }
    sys.stdout.write(json.dumps(response) + "\n")
    sys.stdout.flush()
"#
    .to_owned()
}

fn persisted_news_event(id: i64, title: &str) -> PersistedNewsEvent {
    // Les tests de service n'ont pas besoin de SQLite; cette fixture explicite le contrat d'entree.
    PersistedNewsEvent {
        id,
        source: "financial_juice".to_owned(),
        event_kind: "headline".to_owned(),
        title: title.to_owned(),
        url: "https://example.test/item".to_owned(),
        published_at: Some("2026-04-21T10:00:00Z".to_owned()),
        event_time: None,
        ingested_at: "2026-04-21T10:00:01Z".to_owned(),
        body: None,
        news_level: None,
        importance: None,
        country_code: None,
        currency: None,
        actual_value: None,
        forecast_value: None,
        previous_value: None,
        raw_payload_ref: None,
        signature: format!("test-signature-{id}"),
    }
}

async fn seed_headline_with_published_at(
    state: &crate::app_state::AppState,
    source: &str,
    title: &str,
    published_at: String,
) -> PersistedNewsEvent {
    let event = NewsEvent::new_headline(
        source,
        title,
        format!("https://example.test/{source}/{title}"),
        Some(published_at),
        None,
    );

    state
        .news
        .insert_event(&event)
        .await
        .expect("insert should succeed")
        .expect("event should be stored")
}

fn previous_local_day_noon_rfc3339() -> String {
    let now_utc = OffsetDateTime::now_utc();
    let current_day =
        crate::shared::time::europe_paris_day_for_utc(now_utc).unwrap_or_else(|| now_utc.date());
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

fn test_glossary_entry(
    source_term: &str,
    translated_term: &str,
    spoken_term: Option<&str>,
    priority: i64,
) -> GlossaryEntry {
    // Les tests de traduction n'ont besoin que des champs fonctionnels du glossaire.
    GlossaryEntry {
        id: priority,
        source_term: source_term.to_owned(),
        translated_term: translated_term.to_owned(),
        spoken_term: spoken_term.map(str::to_owned),
        source_language: "en".to_owned(),
        target_language: "fr".to_owned(),
        match_mode: MATCH_MODE_WORD_BOUNDARY.to_owned(),
        case_sensitive: false,
        priority,
        enabled: true,
        scope: GLOSSARY_SCOPE_FULL_PIPELINE.to_owned(),
        notes: None,
        created_at: "2026-06-02T00:00:00Z".to_owned(),
        updated_at: "2026-06-02T00:00:00Z".to_owned(),
    }
}

#[test]
fn heuristic_translate_text_translates_common_market_terms() {
    // On verifie quelques termes de marche frequents pour capter une regression grossiere du dictionnaire.
    let translated = heuristic_translate_text("Powell says rate cuts lift stocks after CPI");

    assert!(translated.contains("Powell"));
    assert!(translated.contains("indique"));
    assert!(translated.contains("baisses de taux"));
    assert!(translated.contains("actions"));
    assert!(translated.contains("apres"));
}

#[test]
fn protect_finance_tokens_keeps_tickers_pairs_and_percentages() {
    // Les placeholders doivent proteger a la fois paire FX, ticker et pourcentage.
    let protected = protect_finance_tokens("EUR/USD holds above 1.08 after CPI and NVDA +3%");

    assert!(protected.masked_text.contains("ZXQTOKEN0QXZ"));
    assert!(protected.masked_text.contains("ZXQTOKEN1QXZ"));
    assert!(protected.masked_text.contains("ZXQTOKEN2QXZ"));
    assert!(protected.masked_text.contains("ZXQTOKEN3QXZ"));

    let restored = protected.restore(&protected.masked_text);
    assert!(restored.contains("EUR/USD"));
    assert!(restored.contains("1.08"));
    assert!(restored.contains("CPI"));
    assert!(restored.contains("NVDA"));
}

#[test]
fn protected_provider_output_rejects_mangled_placeholders() {
    // OPUS-MT peut parfois detruire `ZXQTOKEN0QXZ` en une longue sequence
    // sous-mot. On doit refuser cette sortie pour activer le fallback.
    let protected = protect_finance_tokens("Thursday FX Options Expiries");

    let error = protected
        .restore_provider_output("ZXQTOKEN0QZZKENZZZZKENZZZZZZZZZKENZZZZ")
        .unwrap_err();

    assert!(error.contains("mangled protected finance placeholder"));
}

#[test]
fn protected_provider_output_restores_exact_placeholders() {
    // Une sortie provider qui conserve le placeholder exact reste acceptee.
    let protected = protect_finance_tokens("EUR/USD holds above 1.08 after CPI");

    let restored = protected
        .restore_provider_output(&protected.masked_text)
        .expect("exact placeholders should restore");

    assert!(restored.contains("EUR/USD"));
    assert!(restored.contains("1.08"));
    assert!(restored.contains("CPI"));
}

#[test]
fn opus_mt_profile_appends_missing_beam_size_argument() {
    // Les helpers OPUS-MT restent configurables sans repeter le beam-size dans COMMAND_ARGS.
    let args = vec![
        "/app/scripts/opus_mt_local_command.py".to_owned(),
        "--device".to_owned(),
        "cpu".to_owned(),
    ];

    let resolved = translation_command_args_for_profile(&args, "fast");

    assert_eq!(
        resolved,
        vec![
            "/app/scripts/opus_mt_local_command.py",
            "--device",
            "cpu",
            "--beam-size",
            "1",
        ]
    );
}

#[test]
fn opus_mt_provider_uses_profile_resolved_args() {
    // Ce test couvre le point d'integration: le provider OPUS-MT recoit les args effectifs.
    let mut config = test_translation_config();
    config.provider = OPUS_MT_LOCAL_COMMAND_PROVIDER.to_owned();
    config.profile = "quality".to_owned();
    config.local_command_program = Some("python3".to_owned());
    config.local_command_args = vec!["/app/scripts/opus_mt_local_command.py".to_owned()];

    let provider = TranslationProvider::from_config(&config);

    match provider {
        TranslationProvider::LocalCommand(local) => {
            assert_eq!(
                local.args(),
                vec!["/app/scripts/opus_mt_local_command.py", "--beam-size", "4"]
            );
        }
        _ => panic!("expected local command provider"),
    }
}

#[test]
fn heuristic_provider_preserves_finance_tokens() {
    // Le provider heuristique complet doit conserver les memes protections que l'helper pur.
    let request = TranslationRequest {
        source_title: "EUR/USD holds above 1.08 after CPI".to_owned(),
        source_body: None,
        source_language: "en".to_owned(),
        target_language: "fr".to_owned(),
    };

    let translated = HeuristicTranslationProvider.translate(&request);

    assert!(translated.translated_title.contains("EUR/USD"));
    assert!(translated.translated_title.contains("1.08"));
    assert!(translated.translated_title.contains("CPI"));
}

#[tokio::test]
async fn build_translation_draft_falls_back_to_french_narration_when_title_is_unchanged() {
    // Quand le titre ne peut pas etre ameliore, on doit au moins produire une narration FR stable.
    let item = persisted_news_event(42, "NVDA");
    let config = test_translation_config();
    let mut provider = TranslationProvider::from_config(&config);

    let draft = build_translation_draft(&item, &config, &mut provider, &[])
        .await
        .expect("news item should be eligible");

    assert_eq!(draft.draft.language, "fr");
    assert_eq!(draft.draft.translation_mode, "source_with_french_narration");
    assert!(draft.draft.narration_text.contains(':'));
    assert!(
        draft
            .draft
            .narration_text
            .contains("Titre original en anglais")
    );
}

#[tokio::test]
async fn build_translation_draft_prefixes_active_news_with_important() {
    // Ce test verrouille le contrat produit: `active` dans `Level` produit le libelle important.
    let mut item = persisted_news_event(43, "Powell says rate cuts stay data-dependent");
    item.url = "https://example.test/important".to_owned();
    item.news_level = Some("active active-critical".to_owned());
    item.signature = "important-deadbeef".to_owned();
    let config = test_translation_config();
    let mut provider = TranslationProvider::from_config(&config);

    let draft = build_translation_draft(&item, &config, &mut provider, &[])
        .await
        .expect("news item should be eligible");

    assert!(draft.draft.narration_text.contains(':'));
    assert!(
        draft
            .draft
            .narration_text
            .contains(". Information importante.")
    );
    assert!(!draft.draft.narration_text.starts_with("Important "));
}

#[tokio::test]
async fn build_translation_draft_preserves_long_body_without_backend_ellipsis() {
    // Le corps complet doit atteindre le provider et la persistence; les limites compactes
    // appartiennent seulement aux DTO live.
    let tail_marker = "TAIL_MARKER_FULL_BODY";
    let mut item = persisted_news_event(44, "Long body fixture");
    item.body = Some(format!(
        "{} {tail_marker}",
        "Global equities advance after AI trade strengthens.".repeat(12)
    ));
    let config = test_translation_config();
    assert!(item.body.as_deref().unwrap_or_default().chars().count() > config.max_body_chars);
    let mut provider = TranslationProvider::from_config(&config);

    let draft = build_translation_draft(&item, &config, &mut provider, &[])
        .await
        .expect("news item should be eligible");

    let translated_body = draft
        .draft
        .translated_body
        .as_deref()
        .expect("long body should be translated");
    assert!(translated_body.contains(tail_marker));
    assert!(!translated_body.ends_with('…'));
}

#[tokio::test]
async fn local_command_session_provider_reuses_the_same_process_after_warmup() {
    // Ce test verrouille la promesse principale du mode session: le warmup consomme le premier
    // echange, puis la vraie traduction reutilise le meme helper deja charge.
    let mut item = persisted_news_event(7, "Powell says EUR/USD holds above 1.08");
    item.published_at = None;
    item.body = Some("NVDA rises +3%".to_owned());
    item.signature = "beadfeed".to_owned();
    let config = TranslationConfig {
        enabled: true,
        source_language: "en".to_owned(),
        target_language: "fr".to_owned(),
        provider: LOCAL_COMMAND_SESSION_PROVIDER.to_owned(),
        profile: "quality".to_owned(),
        worker_count: 1,
        local_command_program: Some("python3".to_owned()),
        local_command_args: vec!["-c".to_owned(), session_mock_python()],
        // Le runner CI auto-heberge peut etre fortement contendu pendant les premiers runs a froid.
        // Le test garde donc une marge superieure a la config runtime par defaut.
        local_command_timeout_ms: 30_000,
        warmup_enabled: true,
        warmup_source_text: "warmup headline".to_owned(),
        max_body_chars: 280,
    };
    let mut provider = TranslationProvider::from_config(&config);

    provider
        .warmup(&config)
        .await
        .expect("session warmup should succeed");

    let draft = build_translation_draft(&item, &config, &mut provider, &[])
        .await
        .expect("news item should be eligible");

    assert_eq!(draft.draft.translation_mode, "mock_session");
    assert!(draft.draft.translated_title.starts_with("2: session fr "));
    assert!(draft.draft.translated_title.contains("EUR/USD"));
    assert!(
        draft
            .draft
            .translated_body
            .as_deref()
            .unwrap_or_default()
            .contains("NVDA")
    );
}

#[tokio::test]
async fn build_translation_draft_applies_glossary_without_spoken_pollution() {
    // Cette fixture verrouille le contrat `EN source -> FR affiche -> FR vocal`.
    let item = persisted_news_event(88, "ECB signals no rush to cut rates");
    let config = test_translation_config();
    let mut provider = TranslationProvider::from_config(&config);
    let glossary = vec![test_glossary_entry(
        "ECB",
        "BCE",
        Some("Banque Centrale Europeenne"),
        10,
    )];

    let draft = build_translation_draft(&item, &config, &mut provider, &glossary)
        .await
        .expect("news item should be eligible");

    assert_eq!(draft.glossary_hits, 1);
    assert!(draft.draft.translated_title.contains("BCE"));
    assert!(
        !draft
            .draft
            .translated_title
            .contains("Banque Centrale Europeenne")
    );
    assert!(draft.draft.narration_text.contains("BCE"));
    assert!(
        !draft
            .draft
            .narration_text
            .contains("Banque Centrale Europeenne")
    );
}

#[tokio::test]
async fn build_translation_draft_applies_ecb_possessive_string_pattern() {
    let item = persisted_news_event(89, "ECB's Nagel says inflation risks remain");
    let config = test_translation_config();
    let mut provider = TranslationProvider::from_config(&config);
    let glossary = vec![GlossaryEntry {
        match_mode: MATCH_MODE_PATTERN.to_owned(),
        priority: 100,
        ..test_glossary_entry("ECB's <string>", "<string> de la BCE", None, 100)
    }];

    let draft = build_translation_draft(&item, &config, &mut provider, &glossary)
        .await
        .expect("news item should be eligible");

    assert_eq!(draft.glossary_hits, 1);
    assert!(draft.draft.translated_title.contains("Nagel de la BCE"));
    assert!(!draft.draft.translated_title.contains("ECB's Nagel"));
}

#[tokio::test]
async fn build_translation_draft_formats_glossary_string_colons() {
    let item = persisted_news_event(90, "ECB's Kazimir: Markets have a good sense of the ECB");
    let config = test_translation_config();
    let mut provider = TranslationProvider::from_config(&config);
    let glossary = vec![
        GlossaryEntry {
            match_mode: MATCH_MODE_PATTERN.to_owned(),
            priority: 10,
            ..test_glossary_entry("ECB's <string>", "<string> de la BCE", None, 10)
        },
        GlossaryEntry {
            match_mode: MATCH_MODE_PATTERN.to_owned(),
            priority: 100,
            ..test_glossary_entry("<string>:", "<string> :", Some("<string> :"), 100)
        },
    ];

    let draft = build_translation_draft(&item, &config, &mut provider, &glossary)
        .await
        .expect("news item should be eligible");

    assert!(draft.draft.translated_title.contains("Kazimir de la BCE :"));
    assert!(!draft.draft.translated_title.contains("Kazimir: de la BCE"));
    assert!(!draft.draft.translated_title.contains("Kazimir de la BCE:"));
}
