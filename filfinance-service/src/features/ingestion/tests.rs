//! Tests d'architecture ingestion reserves aux prochaines extractions.

mod collectors_tests {
    // Le test a seulement besoin du type de header HTTP brut.
    use reqwest::header::HeaderValue;
    // `OffsetDateTime` sert a figer un "maintenant" deterministic dans les tests de fenetre J/J-1.
    use time::{OffsetDateTime, format_description::well_known::Rfc3339};
    use url::Url;

    // On cible ici la fonction utilitaire locale.
    use crate::config::{SourceConfig, XConfig};
    use crate::features::ingestion::collectors::{
        event_business_date, filter_startup_backfill_items, parse_rfc3339_business_date,
        retry_after_delay_ms,
        x::{test_build_recent_search_url, test_build_x_query, test_normalize_recent_search_json},
    };
    use crate::features::news::domain::NewsEvent;

    #[test]
    fn retry_after_delay_ms_parses_numeric_seconds() {
        // Cas nominal: un header numerique est converti en millisecondes.
        let header = HeaderValue::from_static("12");

        assert_eq!(retry_after_delay_ms(Some(&header)), Some(12_000));
    }

    #[test]
    fn retry_after_delay_ms_ignores_invalid_values() {
        // Cas de robustesse: une valeur non numerique doit etre ignoree proprement.
        let header = HeaderValue::from_static("soon");

        assert_eq!(retry_after_delay_ms(Some(&header)), None);
    }

    #[test]
    fn startup_backfill_keeps_only_today_and_yesterday_items() {
        // Ce "maintenant" fige la fenetre attendue sur 2026-04-23 UTC.
        let now = OffsetDateTime::parse("2026-04-23T12:00:00Z", &Rfc3339).unwrap();
        let batch = filter_startup_backfill_items(
            vec![
                test_event("today", Some("2026-04-23T08:30:00Z"), None),
                test_event("yesterday", Some("2026-04-22T22:15:00Z"), None),
                test_event("older", Some("2026-04-21T23:59:59Z"), None),
            ],
            now,
        );

        let retained_titles = batch
            .items
            .iter()
            .map(|item| item.title.as_str())
            .collect::<Vec<_>>();

        assert_eq!(retained_titles, vec!["today", "yesterday"]);
        assert_eq!(batch.skipped_outside_window, 1);
        assert_eq!(batch.skipped_without_business_date, 0);
    }

    #[test]
    fn startup_backfill_uses_event_time_when_published_at_is_missing() {
        let now = OffsetDateTime::parse("2026-04-23T12:00:00Z", &Rfc3339).unwrap();
        let batch = filter_startup_backfill_items(
            vec![
                test_event("event time fallback", None, Some("2026-04-22T07:45:00Z")),
                test_event("undated", None, None),
                test_event("invalid date", Some("not-a-date"), None),
            ],
            now,
        );

        let retained_titles = batch
            .items
            .iter()
            .map(|item| item.title.as_str())
            .collect::<Vec<_>>();

        assert_eq!(retained_titles, vec!["event time fallback"]);
        assert_eq!(batch.skipped_without_business_date, 2);
        assert_eq!(batch.skipped_outside_window, 0);
    }

    #[test]
    fn business_date_is_normalized_in_utc() {
        // `-04:00` pousse ici l'instant au 23 avril une fois converti en UTC.
        let business_date = parse_rfc3339_business_date("2026-04-22T23:30:00-04:00");

        assert_eq!(
            business_date.map(|date| date.to_string()),
            Some("2026-04-23".to_owned())
        );
    }

    #[test]
    fn business_date_prefers_published_at_over_event_time() {
        let event = test_event(
            "published at wins",
            Some("2026-04-23T08:30:00Z"),
            Some("2026-04-22T08:30:00Z"),
        );

        assert_eq!(
            event_business_date(&event).map(|date| date.to_string()),
            Some("2026-04-23".to_owned())
        );
    }

    #[test]
    fn x_query_builds_account_filter_with_language_and_repost_exclusion() {
        let config = x_config(vec!["federalreserve", "USTreasury"], None);

        let query = test_build_x_query(&config).expect("X query should build");

        assert_eq!(
            query,
            "(from:federalreserve OR from:USTreasury) lang:en -is:retweet"
        );
    }

    #[test]
    fn x_query_adds_keywords_to_account_filter() {
        let mut config = x_config(vec!["USTreasury"], None);
        config.keywords = vec!["bonds".to_owned(), "Federal Reserve".to_owned()];

        let query = test_build_x_query(&config).expect("X query should build");

        assert_eq!(
            query,
            "((from:USTreasury) (bonds OR \"Federal Reserve\")) lang:en -is:retweet"
        );
    }

    #[test]
    fn x_recent_search_url_includes_cursor_and_response_fields() {
        let config = x_config(vec!["federalreserve"], None);

        let url =
            test_build_recent_search_url(&config, "1780000000000000000").expect("url should build");
        let parsed = Url::parse(&url).expect("url should parse");
        let query_pairs = parsed.query_pairs().collect::<Vec<_>>();

        assert_eq!(
            parsed.as_str().split('?').next(),
            Some(config.source.url.as_str())
        );
        assert!(
            query_pairs
                .iter()
                .any(|(key, value)| key == "since_id" && value == "1780000000000000000")
        );
        assert!(
            query_pairs
                .iter()
                .any(|(key, value)| key == "expansions" && value == "author_id")
        );
        assert!(!query_pairs.iter().any(|(key, _)| key == "start_time"));
    }

    #[test]
    fn x_recent_search_normalizes_posts_to_headlines() {
        let body = r#"{
            "data": [
                {
                    "id": "1780000000000000001",
                    "text": "Fed   signals rates stay high",
                    "author_id": "42",
                    "created_at": "2026-04-23T10:15:00Z"
                }
            ],
            "includes": {
                "users": [
                    {"id": "42", "username": "federalreserve"}
                ]
            },
            "meta": {"result_count": 1, "newest_id": "1780000000000000001"}
        }"#;

        let events =
            test_normalize_recent_search_json("x", body).expect("X payload should normalize");

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].source, "x");
        assert_eq!(
            events[0].title,
            "@federalreserve: Fed signals rates stay high"
        );
        assert_eq!(
            events[0].url,
            "https://x.com/federalreserve/status/1780000000000000001"
        );
        assert_eq!(
            events[0].published_at.as_deref(),
            Some("2026-04-23T10:15:00Z")
        );
        assert_eq!(
            events[0].raw_payload_ref.as_deref(),
            Some("x_post:1780000000000000001")
        );
    }

    fn test_event(title: &str, published_at: Option<&str>, event_time: Option<&str>) -> NewsEvent {
        let mut event = NewsEvent::new_headline(
            "financial_juice",
            title,
            format!("https://example.com/{title}"),
            published_at.map(ToOwned::to_owned),
            None,
        );
        // Les tests pilotent explicitement les deux champs de date selon le cas verifie.
        event.event_time = event_time.map(ToOwned::to_owned);
        event.recompute_signature();
        event
    }

    fn x_config(account_usernames: Vec<&str>, query: Option<&str>) -> XConfig {
        XConfig {
            source: SourceConfig {
                name: "x".to_owned(),
                enabled: true,
                url: "https://api.x.com/2/tweets/search/recent".to_owned(),
                poll_interval_ms: 15_000,
                request_timeout_ms: 5_000,
                rate_limit_backoff_ms: 60_000,
            },
            bearer_token: Some("test-token".to_owned()),
            query: query.map(ToOwned::to_owned),
            account_usernames: account_usernames
                .into_iter()
                .map(ToOwned::to_owned)
                .collect(),
            keywords: Vec::new(),
            language: Some("en".to_owned()),
            exclude_reposts: true,
            max_results: 20,
            startup_lookback_minutes: 120,
        }
    }
}

mod runtime_tests {
    use crate::features::ingestion::runtime::{
        BrowserCollectorStatus, FinancialJuiceBrowserRuntimeState, FinancialJuiceRuntimeState,
    };

    #[test]
    fn financial_juice_runtime_uses_config_as_initial_fallback() {
        let runtime = FinancialJuiceRuntimeState::new(false);

        let snapshot = runtime.snapshot();

        assert!(!snapshot.configured_enabled);
        assert!(!snapshot.enabled);
        assert_eq!(snapshot.disabled_reason, None);
    }

    #[test]
    fn financial_juice_runtime_can_be_overridden_in_memory() {
        let runtime = FinancialJuiceRuntimeState::new(false);

        runtime.set_enabled(true);

        let snapshot = runtime.snapshot();
        assert!(!snapshot.configured_enabled);
        assert!(snapshot.enabled);
        assert_eq!(snapshot.disabled_reason, None);
    }

    #[test]
    fn financial_juice_runtime_records_cloudflare_auto_disable() {
        let runtime = FinancialJuiceRuntimeState::new(true);

        let was_enabled = runtime.disable_after_cloudflare_managed_challenge();

        let snapshot = runtime.snapshot();
        assert!(was_enabled);
        assert!(snapshot.configured_enabled);
        assert!(!snapshot.enabled);
        assert_eq!(
            snapshot.disabled_reason.as_deref(),
            Some("cloudflare_managed_challenge")
        );
    }

    #[test]
    fn financial_juice_runtime_disables_after_ten_consecutive_rate_limits() {
        let runtime = FinancialJuiceRuntimeState::new(true);

        for index in 1..10 {
            let status = runtime.record_rate_limit_failure(10);

            assert_eq!(status.consecutive_failures, index);
            assert_eq!(status.disable_threshold, 10);
            assert!(!status.disabled);
            assert!(!status.was_enabled);
            assert!(runtime.snapshot().enabled);
        }

        let status = runtime.record_rate_limit_failure(10);

        assert_eq!(status.consecutive_failures, 10);
        assert!(status.disabled);
        assert!(status.was_enabled);

        let snapshot = runtime.snapshot();
        assert!(!snapshot.enabled);
        assert_eq!(snapshot.disabled_reason.as_deref(), Some("rate_limited"));
    }

    #[test]
    fn financial_juice_runtime_resets_rate_limit_streak() {
        let runtime = FinancialJuiceRuntimeState::new(true);

        for _ in 0..9 {
            runtime.record_rate_limit_failure(10);
        }
        runtime.reset_rate_limit_failures();

        let status = runtime.record_rate_limit_failure(10);

        assert_eq!(status.consecutive_failures, 1);
        assert!(!status.disabled);
        assert!(runtime.snapshot().enabled);
    }

    #[test]
    fn financial_juice_runtime_admin_override_clears_disabled_reason() {
        let runtime = FinancialJuiceRuntimeState::new(true);

        runtime.record_rate_limit_failure(1);
        runtime.set_enabled(true);

        let snapshot = runtime.snapshot();
        assert!(snapshot.enabled);
        assert_eq!(snapshot.disabled_reason, None);

        let status = runtime.record_rate_limit_failure(10);
        assert_eq!(status.consecutive_failures, 1);
        assert!(runtime.snapshot().enabled);
    }

    #[test]
    fn financial_juice_browser_runtime_records_external_status() {
        let runtime = FinancialJuiceBrowserRuntimeState::new(true);

        runtime.update_status(
            BrowserCollectorStatus::Blocked,
            Some("cloudflare managed challenge".to_owned()),
        );

        let snapshot = runtime.snapshot(true, false, 30_000);
        assert!(snapshot.configured_enabled);
        assert!(!snapshot.primary_enabled);
        assert!(snapshot.fallback_active);
        assert!(snapshot.ingest_token_configured);
        assert_eq!(snapshot.status, BrowserCollectorStatus::Blocked);
        assert_eq!(
            snapshot.message.as_deref(),
            Some("cloudflare managed challenge")
        );
        assert!(snapshot.updated_at.is_some());
        assert!(!snapshot.effective_enabled);
        assert!(snapshot.service_accessible);
        assert!(!snapshot.main_feed_available);
    }

    #[test]
    fn financial_juice_browser_runtime_snapshot_excludes_process_control() {
        let runtime = FinancialJuiceBrowserRuntimeState::new(true);

        let payload =
            serde_json::to_value(runtime.snapshot(true, false, 30_000)).expect("serializable");

        assert!(payload.get("fallback_active").is_some());
        assert!(payload.get("process_control").is_none());
    }

    #[test]
    fn financial_juice_browser_effective_enabled_requires_ok_fresh_fallback() {
        let runtime = FinancialJuiceBrowserRuntimeState::new(true);

        assert!(!runtime.effective_enabled(true, 30_000));
        assert!(!runtime.effective_enabled(false, 30_000));

        runtime.update_status(
            BrowserCollectorStatus::Ok,
            Some("FinancialJuice #mainFeed is available".to_owned()),
        );

        assert!(runtime.effective_enabled(false, 30_000));

        runtime.update_status(
            BrowserCollectorStatus::Blocked,
            Some("cloudflare managed challenge".to_owned()),
        );

        assert!(!runtime.effective_enabled(false, 30_000));
    }
}

mod browser_ingest_tests {
    use crate::features::{
        ingestion::http::{BrowserNewsEventRequest, test_browser_news_event_from_request},
        test_support,
    };

    #[tokio::test]
    async fn browser_ingest_maps_payload_to_canonical_news_event() {
        let state = test_support::app_state().await;

        let event = test_browser_news_event_from_request(
            &state,
            BrowserNewsEventRequest {
                external_id: Some("9566991".to_owned()),
                title: "  Fed   signals rates stay high  ".to_owned(),
                description: Some(" Market detail ".to_owned()),
                url: Some("https://www.financialjuice.com/news/9566991".to_owned()),
                published_at: Some("2026-05-06T12:00:00Z".to_owned()),
                news_level: Some("active active-critical".to_owned()),
            },
        )
        .expect("browser event should parse")
        .expect("browser event should be retained");

        assert_eq!(event.source, "financial_juice_browser");
        assert_eq!(event.title, "Fed signals rates stay high");
        assert_eq!(event.body.as_deref(), Some("Market detail"));
        assert_eq!(event.published_at.as_deref(), Some("2026-05-06T12:00:00Z"));
        assert_eq!(
            event.raw_payload_ref.as_deref(),
            Some("financial_juice_browser:9566991")
        );
        assert_eq!(event.news_level.as_deref(), Some("active active-critical"));
        assert!(!event.signature.is_empty());
    }

    #[tokio::test]
    async fn browser_ingest_drops_empty_titles() {
        let state = test_support::app_state().await;

        let event = test_browser_news_event_from_request(
            &state,
            BrowserNewsEventRequest {
                external_id: None,
                title: "   ".to_owned(),
                description: None,
                url: None,
                published_at: None,
                news_level: None,
            },
        )
        .expect("empty title should not be fatal");

        assert!(event.is_none());
    }
}

mod financial_juice_tests {
    // `HashMap` sert aux fixtures du champ `extra` capturé par `serde(flatten)`.
    // Les tests peuvent ainsi expliciter "aucun champ inconnu" sans dépendre du parsing JSON.
    use std::collections::HashMap;

    use crate::features::ingestion::collectors::financial_juice::model::{
        FinancialJuiceApiNewsItem, FinancialJuiceBackfillTimestampCursor,
        extract_asmx_string_payload, extract_js_number_list, is_priority_like_unknown_value_active,
        normalize_financial_juice_history_items, normalize_financial_juice_news_items,
        parse_financial_juice_news_items_payload, parse_financial_juice_time_of_day,
        parse_home_page_state,
    };
    use serde_json::json;
    use time::OffsetDateTime;

    #[test]
    fn parse_home_page_state_extracts_authenticated_bootstrap_fields() {
        let body = r#"
        <input type="hidden" id="__VIEWSTATE" value="STATE123" />
        <input type="hidden" id="__VIEWSTATEGENERATOR" value="GEN456" />
        <script>
        var UserLoggedIn=true;
        var LoggedUser=true;
        var MainURLData='https://live.financialjuice.com/';
        var info = 'opaque-info-token';
        var feedtoken = 'jwt-feed-token';
        var FeedIds= [0,7,8,1334];
        </script>
        "#;

        let parsed =
            parse_home_page_state("financial_juice", body).expect("page state should parse");

        assert!(parsed.user_logged_in);
        assert_eq!(parsed.view_state, "STATE123");
        assert_eq!(parsed.view_state_generator, "GEN456");
        assert_eq!(
            parsed.main_url.as_deref(),
            Some("https://live.financialjuice.com/")
        );
        assert_eq!(parsed.info.as_deref(), Some("opaque-info-token"));
        assert_eq!(parsed.feedtoken.as_deref(), Some("jwt-feed-token"));
        assert_eq!(parsed.feed_ids, vec![0, 7, 8, 1334]);
    }

    #[test]
    fn parse_home_page_state_accepts_spaces_around_js_assignments() {
        let body = r#"
        <input type="hidden" id="__VIEWSTATE" value="STATE123" />
        <input type="hidden" id="__VIEWSTATEGENERATOR" value="GEN456" />
        <script>
        var UserLoggedIn = true;
        var LoggedUser = true;
        var MainURLData = 'https://live.financialjuice.com/';
        var info = 'opaque-info-token';
        var feedtoken = 'jwt-feed-token';
        var FeedIds = [0, 7, 8, 1334];
        </script>
        "#;

        let parsed =
            parse_home_page_state("financial_juice", body).expect("page state should parse");
        assert!(parsed.user_logged_in);
        assert_eq!(parsed.feedtoken.as_deref(), Some("jwt-feed-token"));
        assert_eq!(extract_js_number_list(body, "FeedIds"), vec![0, 7, 8, 1334]);
    }

    #[test]
    fn extract_asmx_string_payload_reads_xml_wrapped_json() {
        let body = r#"<?xml version="1.0" encoding="utf-8"?>
        <string xmlns="http://tempuri.org/">[{"NewsID":1,"Title":"Headline"}]</string>"#;

        let payload =
            extract_asmx_string_payload("financial_juice", body).expect("payload should parse");

        assert_eq!(payload, r#"[{"NewsID":1,"Title":"Headline"}]"#);
    }

    #[test]
    fn parse_financial_juice_news_items_payload_recovers_valid_items_around_bad_object() {
        // Cette fixture reproduit une page `GetPreviousNews` ou un seul objet casse la syntaxe JSON.
        let body = r#"<?xml version="1.0" encoding="utf-8"?>
        <string xmlns="http://tempuri.org/">[
          {"NewsID":9566991,"Title":"Valid before","Description":"","DatePublished":"2026-05-04T08:45:00","EURL":"/News/9566991/valid-before.aspx"},
          {"NewsID":9566990,"Title":"Broken object" "Description":"","DatePublished":"2026-05-04T08:44:00","EURL":"/News/9566990/broken.aspx"},
          {"NewsID":9566989,"Title":"Valid after","Description":"","DatePublished":"2026-05-04T08:43:00","EURL":"/News/9566989/valid-after.aspx"}
        ]</string>"#;

        let items = parse_financial_juice_news_items_payload("financial_juice", body)
            .expect("valid surrounding items should be recovered");

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].news_id, 9566991);
        assert_eq!(items[1].news_id, 9566989);
    }

    #[test]
    fn normalize_financial_juice_news_items_maps_api_items_to_headlines() {
        let items = vec![FinancialJuiceApiNewsItem {
            news_id: 9551289,
            title: "Ukrainian drones have struck an oil pumping station".to_owned(),
            description: "".to_owned(),
            date_published: Some("2026-04-23T10:07:00.35".to_owned()),
            url: "https://www.financialjuice.com/News/9551289/example.aspx".to_owned(),
            iid: Some("dee07c0e-4b3d-4dd0-b26c-07511e6e225f".to_owned()),
            // `Level` est conserve brut pour que les clients sachent distinguer les alertes critiques.
            level: Some("active active-critical".to_owned()),
            // Le cas nominal ne porte pas de champ inconnu: le refactor `flatten` ne doit pas changer la conversion headline.
            extra: HashMap::new(),
        }];

        let events = normalize_financial_juice_news_items("financial_juice", items);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].source, "financial_juice");
        assert_eq!(
            events[0].title,
            "Ukrainian drones have struck an oil pumping station"
        );
        assert_eq!(
            events[0].published_at.as_deref(),
            Some("2026-04-23T10:07:00.35Z")
        );
        assert_eq!(
            events[0].news_level.as_deref(),
            Some("active active-critical")
        );
    }

    #[test]
    fn normalize_financial_juice_news_items_maps_empty_level_to_none() {
        // FinancialJuice emet parfois `Level: ""`; ce test evite de propager un faux niveau client.
        let item = FinancialJuiceApiNewsItem {
            news_id: 9551290,
            title: "General headline".to_owned(),
            description: "".to_owned(),
            date_published: Some("2026-04-23T10:08:00.00".to_owned()),
            url: "https://www.financialjuice.com/News/9551290/general.aspx".to_owned(),
            iid: None,
            // La chaine vide doit etre normalisee en absence de niveau exploitable.
            level: Some("".to_owned()),
            // Aucun champ extra n'est necessaire: le scenario cible uniquement la normalisation de `Level`.
            extra: HashMap::new(),
        };

        let event = item
            .into_news_event("financial_juice")
            .expect("item should convert to a headline");

        assert_eq!(event.news_level, None);
    }

    #[test]
    fn priority_like_unknown_value_ignores_false_breaking_flags() {
        // Le backlog FinancialJuice expose `Breaking: false` sur des lignes ordinaires: cela ne doit pas bruiter les logs info.
        assert!(!is_priority_like_unknown_value_active(&json!(false)));
        assert!(!is_priority_like_unknown_value_active(&json!("false")));
        assert!(!is_priority_like_unknown_value_active(&json!(0)));
        assert!(is_priority_like_unknown_value_active(&json!(true)));
        assert!(is_priority_like_unknown_value_active(&json!("critical")));
    }

    #[test]
    fn parse_financial_juice_time_of_day_reads_fractional_seconds() {
        let parsed =
            parse_financial_juice_time_of_day("2026-04-19T06:45:00.21").expect("time should parse");

        assert_eq!(parsed.hour(), 6);
        assert_eq!(parsed.minute(), 45);
        assert_eq!(parsed.second(), 0);
        assert_eq!(parsed.nanosecond(), 210_000_000);
    }

    #[test]
    fn normalize_financial_juice_history_items_rebuilds_recent_utc_days() {
        let mut cursor = FinancialJuiceBackfillTimestampCursor::new(
            OffsetDateTime::parse(
                "2026-04-24T07:03:00Z",
                &time::format_description::well_known::Rfc3339,
            )
            .expect("reference timestamp should parse"),
        );
        let items = vec![
            FinancialJuiceApiNewsItem {
                news_id: 9553148,
                title: "Currency Strength Chart".to_owned(),
                description: "".to_owned(),
                date_published: Some("2026-04-19T07:02:08.7".to_owned()),
                url: "https://www.financialjuice.com/News/9553148/example.aspx".to_owned(),
                iid: None,
                // Le test historique n'a pas besoin de niveau: l'absence explicite preserve l'ancien contrat.
                level: None,
                // Le backfill testé ici se concentre sur la réparation de date, pas sur les champs `extra`.
                extra: HashMap::new(),
            },
            FinancialJuiceApiNewsItem {
                news_id: 9552800,
                title: "Asia session late headline".to_owned(),
                description: "".to_owned(),
                date_published: Some("2026-04-19T00:07:36".to_owned()),
                url: "https://www.financialjuice.com/News/9552800/example.aspx".to_owned(),
                iid: None,
                // Pas de niveau source dans cette fixture: on isole le test de reconstruction des jours UTC.
                level: None,
                // `HashMap::new()` documente volontairement l'absence de signal inconnu pour ce scénario.
                extra: HashMap::new(),
            },
            FinancialJuiceApiNewsItem {
                news_id: 9552799,
                title: "Previous day late headline".to_owned(),
                description: "".to_owned(),
                date_published: Some("2026-04-19T23:51:34".to_owned()),
                url: "https://www.financialjuice.com/News/9552799/example.aspx".to_owned(),
                iid: None,
                // L'absence de `Level` couvre le comportement historique des anciennes reponses FJ.
                level: None,
                // Le dernier item protège le même contrat: aucun champ inconnu ne doit perturber la chronologie reconstruite.
                extra: HashMap::new(),
            },
        ];

        let events = normalize_financial_juice_history_items("financial_juice", items, &mut cursor);

        assert_eq!(events.len(), 3);
        assert_eq!(
            events[0].published_at.as_deref(),
            Some("2026-04-24T07:02:08.7Z")
        );
        assert_eq!(
            events[1].published_at.as_deref(),
            Some("2026-04-24T00:07:36Z")
        );
        assert_eq!(
            events[2].published_at.as_deref(),
            Some("2026-04-23T23:51:34Z")
        );
    }
}
