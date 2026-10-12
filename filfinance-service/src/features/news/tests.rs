//! Tests d'architecture news reserves aux prochaines extractions.

mod sqlite_tests {
    // On importe la config de stockage et les modeles minimaux necessaires pour construire des fixtures realistes.
    use crate::{
        config::StorageConfig,
        features::{
            news::domain::DailyNewsQualificationDraft,
            news::domain::{DailySummaryDraft, EventKind, NewsEvent, NewsListParams},
            translation::domain::{
                NewsTranslationDraft, TranslationHistoryParams, TranslationHistoryScope,
            },
        },
    };

    // Les tests ciblent directement `SqliteNewsStore`, donc on importe le type depuis le module parent.
    use crate::features::news::sqlite::SqliteNewsStore;
    // `Rfc3339` parse une date fixe pour rendre le test de retention independant de l'horloge courante.
    use time::{OffsetDateTime, format_description::well_known::Rfc3339};

    #[tokio::test]
    async fn storage_lists_history_with_filters_and_cursor() {
        // Ce test verrouille le contrat de tri/pagination par date visible puis `id`.
        let storage = test_storage().await;

        let oldest = insert_headline_at(
            &storage,
            "financial_juice",
            "older headline",
            Some("2026-04-21T09:00:00Z"),
        )
        .await
        .expect("insert should succeed")
        .expect("first insert should persist");
        let latest = insert_headline_at(
            &storage,
            "financial_juice",
            "latest by published_at",
            Some("2026-04-21T11:00:00Z"),
        )
        .await
        .expect("insert should succeed")
        .expect("second insert should persist");
        let middle = insert_headline_at(
            &storage,
            "financial_juice_browser",
            "middle by published_at",
            Some("2026-04-21T10:00:00Z"),
        )
        .await
        .expect("insert should succeed")
        .expect("third insert should persist");

        // La premiere page doit ramener les deux plus recentes selon `published_at`.
        let first_page = storage
            .list_news(&NewsListParams {
                limit: 2,
                source: None,
                event_kind: None,
                before_id: None,
            })
            .await
            .expect("listing should succeed");

        assert_eq!(first_page.items.len(), 2);
        assert!(first_page.has_more);
        assert_eq!(first_page.items[0].id, latest.id);
        assert_eq!(first_page.items[1].id, middle.id);
        assert_eq!(first_page.next_before_id, Some(first_page.items[1].id));

        // La seconde page repart du curseur `before_id` et doit retrouver le plus ancien restant.
        let second_page = storage
            .list_news(&NewsListParams {
                limit: 2,
                source: None,
                event_kind: None,
                before_id: first_page.next_before_id,
            })
            .await
            .expect("cursor listing should succeed");

        assert_eq!(second_page.items.len(), 1);
        assert!(!second_page.has_more);
        assert_eq!(second_page.items[0].id, oldest.id);
    }

    #[tokio::test]
    async fn storage_filters_by_source_and_reads_single_event() {
        // Ce test couvre a la fois le filtre `source` et la lecture par identifiant.
        let storage = test_storage().await;

        let expected = insert_headline(&storage, "financial_juice", "headline one")
            .await
            .expect("insert should succeed")
            .expect("event should be stored");
        let _ = insert_headline(&storage, "financial_juice_browser", "headline two")
            .await
            .expect("insert should succeed")
            .expect("event should be stored");

        let filtered = storage
            .list_news(&NewsListParams {
                limit: 10,
                source: Some("financial_juice".to_owned()),
                event_kind: Some(EventKind::Headline.as_str().to_owned()),
                before_id: None,
            })
            .await
            .expect("filtered listing should succeed");

        assert_eq!(filtered.items.len(), 1);
        assert_eq!(filtered.items[0].id, expected.id);

        let fetched = storage
            .get_news_by_id(expected.id)
            .await
            .expect("single fetch should succeed");

        // On verifie ici qu'une lecture unitaire retrouve bien la ligne persistee.
        assert_eq!(fetched.expect("event should exist").title, "headline one");
    }

    #[tokio::test]
    async fn storage_persists_news_level_on_news_events() {
        // Ce test verrouille la nouvelle colonne source avant de l'exposer aux payloads client.
        let storage = test_storage().await;
        let mut event = NewsEvent::new_headline(
            "financial_juice",
            "Critical headline",
            "https://example.test/critical",
            Some("2026-04-23T10:07:00Z".to_owned()),
            None,
        );
        // `news_level` n'entre pas dans la signature, mais doit survivre a l'insertion durable.
        event.news_level = Some("active active-critical".to_owned());
        event.recompute_signature();

        let stored = storage
            .insert_event(&event)
            .await
            .expect("insert should succeed")
            .expect("event should be stored");
        let fetched = storage
            .get_news_by_id(stored.id)
            .await
            .expect("single fetch should succeed")
            .expect("event should exist");

        assert_eq!(
            fetched.news_level.as_deref(),
            Some("active active-critical")
        );
        assert_eq!(fetched.importance.as_deref(), Some("high"));
    }

    #[tokio::test]
    async fn storage_derives_financial_juice_importance_from_news_level() {
        let storage = test_storage().await;

        for (suffix, news_level, expected_importance) in [
            ("active-critical", Some("active active-critical"), "high"),
            ("active-alone", Some("active"), "high"),
            ("inactive-word", Some("inactive"), "medium"),
            ("general", Some("news-general"), "medium"),
            ("missing", None, "low"),
        ] {
            let mut event = NewsEvent::new_headline(
                "financial_juice",
                format!("Importance fixture {suffix}"),
                format!("https://example.test/importance/{suffix}"),
                Some("2026-04-23T10:07:00Z".to_owned()),
                None,
            );
            event.news_level = news_level.map(ToOwned::to_owned);
            event.recompute_signature();

            let stored = storage
                .insert_event(&event)
                .await
                .expect("insert should succeed")
                .expect("event should be stored");
            let fetched = storage
                .get_news_by_id(stored.id)
                .await
                .expect("single fetch should succeed")
                .expect("event should exist");

            assert_eq!(
                fetched.importance.as_deref(),
                Some(expected_importance),
                "unexpected importance for {suffix}"
            );
        }
    }

    #[tokio::test]
    async fn storage_merges_news_level_when_duplicate_arrives_later() {
        // Ce test protege le cas reel ou une premiere occurrence arrive sans `Level`.
        let storage = test_storage().await;
        let first = NewsEvent::new_headline(
            "financial_juice",
            "Same headline",
            "https://example.test/same",
            Some("2026-04-23T10:07:00Z".to_owned()),
            None,
        );
        let stored = storage
            .insert_event(&first)
            .await
            .expect("first insert should succeed")
            .expect("first event should be stored");

        let mut duplicate = first.clone();
        // La signature reste identique: seul le niveau source s'enrichit sur le doublon.
        duplicate.news_level = Some("news-general".to_owned());
        let ignored_duplicate = storage
            .insert_event(&duplicate)
            .await
            .expect("duplicate insert should not fail");
        let fetched = storage
            .get_news_by_id(stored.id)
            .await
            .expect("single fetch should succeed")
            .expect("event should exist");

        assert!(ignored_duplicate.is_none());
        assert_eq!(fetched.news_level.as_deref(), Some("news-general"));
        assert_eq!(fetched.importance.as_deref(), Some("medium"));
    }

    #[tokio::test]
    async fn storage_deduplicates_financial_juice_browser_after_primary() {
        // Le collecteur navigateur est un fallback: il ne doit pas polluer une ligne officielle.
        let storage = test_storage().await;
        let primary = NewsEvent::new_headline(
            "financial_juice",
            "Fed's Hammack concludes remarks",
            "https://www.financialjuice.com/News/9574057/Feds-Hammack-concludes-remarks.aspx",
            Some("2026-05-07T14:40:27.123Z".to_owned()),
            None,
        );
        let mut browser_duplicate = NewsEvent::new_headline(
            "financial_juice_browser",
            "Fed's Hammack concludes remarks",
            "https://www.financialjuice.com/News/9574057/Feds-Hammack-concludes-remarks.aspx",
            Some("2026-05-07T14:40:00Z".to_owned()),
            None,
        );
        browser_duplicate.news_level = Some("news-general US Bonds USD".to_owned());
        browser_duplicate.recompute_signature();

        assert_eq!(primary.signature, browser_duplicate.signature);

        let stored = storage
            .insert_event(&primary)
            .await
            .expect("primary insert should succeed")
            .expect("primary should be stored");
        let ignored_duplicate = storage
            .insert_event(&browser_duplicate)
            .await
            .expect("browser duplicate insert should not fail");
        let fetched = storage
            .get_news_by_id(stored.id)
            .await
            .expect("single fetch should succeed")
            .expect("event should exist");

        assert!(ignored_duplicate.is_none());
        assert_eq!(fetched.source, "financial_juice");
        assert_eq!(fetched.news_level, None);
    }

    #[tokio::test]
    async fn storage_promotes_browser_duplicate_when_primary_arrives_later() {
        // Si le fallback a gagne temporairement, la source officielle reprend la priorite sans doublon.
        let storage = test_storage().await;
        let browser = NewsEvent::new_headline(
            "financial_juice_browser",
            "US Energy Secretary Wright concludes remarks on Fox News",
            "https://www.financialjuice.com/News/9574053/browser.aspx",
            Some("2026-05-07T14:37:00Z".to_owned()),
            None,
        );
        let mut primary = NewsEvent::new_headline(
            "financial_juice",
            "US Energy Secretary Wright concludes remarks on Fox News",
            "https://www.financialjuice.com/News/9574053/US-Energy-Secretary-Wright-concludes-remarks-on-Fox-News.aspx",
            Some("2026-05-07T14:37:22.456Z".to_owned()),
            Some("official body".to_owned()),
        );
        primary.news_level = Some("news-general".to_owned());
        primary.raw_payload_ref = Some("financial_juice_news_id:9574053".to_owned());
        primary.recompute_signature();

        assert_eq!(browser.signature, primary.signature);

        let stored_browser = storage
            .insert_event(&browser)
            .await
            .expect("browser insert should succeed")
            .expect("browser event should be stored");
        storage
            .upsert_translation(
                &stored_browser,
                &NewsTranslationDraft {
                    language: "fr".to_owned(),
                    status: "ready".to_owned(),
                    translated_title: "Le secretaire americain conclut".to_owned(),
                    translated_body: None,
                    narration_text: "16:37. Le secretaire americain conclut.".to_owned(),
                    translation_mode: "heuristic_fr".to_owned(),
                    error_message: None,
                },
            )
            .await
            .expect("translation upsert should succeed");

        let ignored_primary_duplicate = storage
            .insert_event(&primary)
            .await
            .expect("primary duplicate insert should not fail");
        let fetched = storage
            .get_news_by_id(stored_browser.id)
            .await
            .expect("single fetch should succeed")
            .expect("event should exist");
        let translation = storage
            .get_translation_by_news_event_id_and_language(stored_browser.id, "fr")
            .await
            .expect("translation read should succeed")
            .expect("translation should still exist");

        assert!(ignored_primary_duplicate.is_none());
        assert_eq!(fetched.source, "financial_juice");
        assert_eq!(fetched.url, primary.url);
        assert_eq!(fetched.body.as_deref(), Some("official body"));
        assert_eq!(fetched.news_level.as_deref(), Some("news-general"));
        assert_eq!(fetched.importance.as_deref(), Some("medium"));
        assert_eq!(
            fetched.raw_payload_ref.as_deref(),
            Some("financial_juice_news_id:9574053")
        );
        assert_eq!(translation.source, "financial_juice");
        assert_eq!(translation.source_body.as_deref(), Some("official body"));
    }

    #[tokio::test]
    async fn storage_backfills_empty_financial_juice_importance_without_overwriting() {
        let storage = test_storage().await;

        let mut active = NewsEvent::new_headline(
            "financial_juice",
            "Backfill active",
            "https://example.test/backfill/active",
            Some("2026-04-23T10:07:00Z".to_owned()),
            None,
        );
        active.news_level = Some("active active-critical".to_owned());
        active.recompute_signature();
        let active = storage
            .insert_event(&active)
            .await
            .expect("active insert should succeed")
            .expect("active should be stored");

        let missing_level = storage
            .insert_event(&NewsEvent::new_headline(
                "financial_juice",
                "Backfill missing level",
                "https://example.test/backfill/missing",
                Some("2026-04-23T10:08:00Z".to_owned()),
                None,
            ))
            .await
            .expect("missing insert should succeed")
            .expect("missing should be stored");

        let preserve = storage
            .insert_event(&NewsEvent::new_headline(
                "financial_juice",
                "Backfill preserve",
                "https://example.test/backfill/preserve",
                Some("2026-04-23T10:09:00Z".to_owned()),
                None,
            ))
            .await
            .expect("preserve insert should succeed")
            .expect("preserve should be stored");

        let pool = storage.clone_pool_for_feature_store();
        sqlx::query("UPDATE news_events SET importance = NULL WHERE id IN (?1, ?2)")
            .bind(active.id)
            .bind(missing_level.id)
            .execute(&pool)
            .await
            .expect("fixture reset should succeed");
        sqlx::query("UPDATE news_events SET importance = 'manual' WHERE id = ?1")
            .bind(preserve.id)
            .execute(&pool)
            .await
            .expect("manual fixture should update");

        let updated = storage
            .backfill_financial_juice_importance_from_news_level()
            .await
            .expect("backfill should succeed");

        assert_eq!(updated, 2);
        assert_eq!(
            storage
                .get_news_by_id(active.id)
                .await
                .unwrap()
                .unwrap()
                .importance
                .as_deref(),
            Some("high")
        );
        assert_eq!(
            storage
                .get_news_by_id(missing_level.id)
                .await
                .unwrap()
                .unwrap()
                .importance
                .as_deref(),
            Some("low")
        );
        assert_eq!(
            storage
                .get_news_by_id(preserve.id)
                .await
                .unwrap()
                .unwrap()
                .importance
                .as_deref(),
            Some("manual")
        );
    }

    #[tokio::test]
    async fn storage_deduplicates_financial_juice_text_punctuation_variants_across_minutes() {
        // FinancialJuice peut republier le meme texte avec une ponctuation source differente une minute apres.
        let storage = test_storage().await;
        let first = insert_headline_at(
            &storage,
            "financial_juice",
            "OpenAI's AI chip agreement with Broadcom encounters $18 billion funding hurdle: The Information",
            Some("2026-05-07T18:12:00Z"),
        )
        .await
        .expect("first insert should succeed")
        .expect("first event should be stored");
        let duplicate = insert_headline_at(
            &storage,
            "financial_juice",
            "OpenAI's AI chip agreement with Broadcom encounters $18 billion funding hurdle - The Information",
            Some("2026-05-07T18:13:00Z"),
        )
        .await
        .expect("near duplicate insert should not fail");
        let listed = storage
            .list_news(&NewsListParams {
                limit: 10,
                source: Some("financial_juice".to_owned()),
                event_kind: Some(EventKind::Headline.as_str().to_owned()),
                before_id: None,
            })
            .await
            .expect("listing should succeed");

        assert!(duplicate.is_none());
        assert_eq!(listed.items.len(), 1);
        assert_eq!(listed.items[0].id, first.id);
    }

    #[tokio::test]
    async fn storage_deduplicates_financial_juice_text_when_eighty_percent_words_match() {
        // La regle projet bypasse les messages dont au moins 80% des mots existent deja.
        let storage = test_storage().await;
        let first = insert_headline_at(
            &storage,
            "financial_juice",
            "AirPods cameras to supply visual data to Siri $AAPL",
            Some("2026-05-07T18:12:00Z"),
        )
        .await
        .expect("first insert should succeed")
        .expect("first event should be stored");
        let duplicate = insert_headline_at(
            &storage,
            "financial_juice",
            "AirPods cameras to supply visual data to Siri",
            Some("2026-05-07T18:13:00Z"),
        )
        .await
        .expect("near duplicate insert should not fail");

        assert!(duplicate.is_none());
        let fetched = storage
            .get_news_by_id(first.id)
            .await
            .expect("single fetch should succeed")
            .expect("first event should remain");
        assert_eq!(fetched.title, first.title);
    }

    #[tokio::test]
    async fn storage_only_compares_financial_juice_text_against_last_five_messages() {
        // La regle reste bornee aux cinq derniers messages persistants.
        let storage = test_storage().await;
        let first = insert_headline_at(
            &storage,
            "financial_juice",
            "ECB's Schnabel: Waiting for wage increases to appear will be too late for ECB.",
            Some("2026-05-07T18:12:00Z"),
        )
        .await
        .expect("first insert should succeed")
        .expect("first event should be stored");

        let filler_titles = [
            "Oil markets react to shipping update",
            "Dollar index holds gains into close",
            "Treasury yields edge lower after auction",
            "European equities finish mixed Thursday",
            "Crypto volumes slow ahead of weekend",
        ];

        for filler_title in filler_titles {
            insert_headline_at(
                &storage,
                "financial_juice",
                filler_title,
                Some("2026-05-07T18:13:00Z"),
            )
            .await
            .expect("filler insert should succeed")
            .expect("filler event should be stored");
        }

        let later = insert_headline_at(
            &storage,
            "financial_juice",
            "ECB's Schnabel - Waiting for wage increases to appear will be too late for ECB.",
            Some("2026-05-07T18:14:00Z"),
        )
        .await
        .expect("later insert should succeed")
        .expect("later event should be stored");

        assert_ne!(first.id, later.id);
    }

    #[tokio::test]
    async fn storage_cleans_invalid_financial_juice_browser_rows() {
        // Les pollutions du fallback navigateur doivent sortir de la base, pas seulement du rendu.
        let storage = test_storage().await;
        let now = OffsetDateTime::parse("2026-05-07T15:32:00Z", &Rfc3339)
            .expect("fixed timestamp should parse");

        let browser_future = insert_headline_at(
            &storage,
            "financial_juice_browser",
            "Fed's Williams Speaks",
            Some("2026-05-07T20:10:00Z"),
        )
        .await
        .expect("future browser insert should succeed")
        .expect("future browser event should be stored");
        storage
            .upsert_translation(
                &browser_future,
                &NewsTranslationDraft {
                    language: "fr".to_owned(),
                    status: "ready".to_owned(),
                    translated_title: "Williams de la Fed parle".to_owned(),
                    translated_body: None,
                    narration_text: "22:10. Williams de la Fed parle.".to_owned(),
                    translation_mode: "opus_mt_tc_big_local_session".to_owned(),
                    error_message: None,
                },
            )
            .await
            .expect("future browser translation should persist");

        let browser_date = insert_headline_at(
            &storage,
            "financial_juice_browser",
            "December 18",
            Some("2026-05-07T15:00:00Z"),
        )
        .await
        .expect("date-only browser insert should succeed")
        .expect("date-only browser event should be stored");
        let browser_label = insert_headline_at(
            &storage,
            "financial_juice_browser",
            "US Indexes",
            Some("2026-05-07T15:26:00Z"),
        )
        .await
        .expect("market label browser insert should succeed")
        .expect("market label browser event should be stored");
        let browser_text_fallback = insert_headline_with_raw_ref(
            &storage,
            "financial_juice_browser",
            "A rally that drove stocks to all-time highs wavered as traders awaited updates",
            Some("2026-05-07T15:30:00Z"),
            Some("financial_juice_browser:text:2026-05-07T15:30:00Z:A rally"),
        )
        .await
        .expect("text fallback browser insert should succeed")
        .expect("text fallback browser event should be stored");
        let browser_embedded_timestamp = insert_headline_at(
            &storage,
            "financial_juice_browser",
            "[11:24 ET] Iran imposes new rules for Strait of Hormuz passage - CNN",
            Some("2026-05-07T15:26:00Z"),
        )
        .await
        .expect("embedded timestamp browser insert should succeed")
        .expect("embedded timestamp browser event should be stored");
        let mut event_header = NewsEvent::new_headline(
            "financial_juice_browser",
            "US Consumer Credit - 13.72B 9.48B",
            "https://example.test/calendar",
            Some("2026-05-07T15:00:00Z".to_owned()),
            None,
        );
        event_header.news_level = Some("event-header".to_owned());
        let browser_event_header = storage
            .insert_event(&event_header)
            .await
            .expect("event-header browser insert should succeed")
            .expect("event-header browser event should be stored");
        let browser_valid = insert_headline_at(
            &storage,
            "financial_juice_browser",
            "Fed's Hammack concludes remarks",
            Some("2026-05-07T15:31:00Z"),
        )
        .await
        .expect("valid browser insert should succeed")
        .expect("valid browser event should be stored");
        let primary_future = insert_headline_at(
            &storage,
            "financial_juice",
            "Official future primary headline",
            Some("2026-05-07T20:10:00Z"),
        )
        .await
        .expect("future primary insert should succeed")
        .expect("future primary event should be stored");

        let report = storage
            .cleanup_invalid_financial_juice_browser_events(now)
            .await
            .expect("browser cleanup should succeed");

        assert_eq!(report.deleted_news_events, 6);
        assert_eq!(report.deleted_translations, 1);
        for deleted_id in [
            browser_future.id,
            browser_date.id,
            browser_label.id,
            browser_text_fallback.id,
            browser_embedded_timestamp.id,
            browser_event_header.id,
        ] {
            assert!(
                storage
                    .get_news_by_id(deleted_id)
                    .await
                    .expect("deleted event lookup should succeed")
                    .is_none()
            );
        }
        assert!(
            storage
                .get_translation_by_news_event_id_and_language(browser_future.id, "fr")
                .await
                .expect("deleted translation lookup should succeed")
                .is_none()
        );
        assert!(
            storage
                .get_news_by_id(browser_valid.id)
                .await
                .expect("valid browser lookup should succeed")
                .is_some()
        );
        assert!(
            storage
                .get_news_by_id(primary_future.id)
                .await
                .expect("future primary lookup should succeed")
                .is_some()
        );
    }

    #[tokio::test]
    async fn storage_upserts_translations_by_news_event_and_language() {
        // Ce test valide la cle fonctionnelle `(news_event_id, language)` cote traduction.
        let storage = test_storage().await;

        let stored = insert_headline_at(
            &storage,
            "financial_juice",
            "Powell says rates stay high",
            Some("2026-04-21T10:15:00Z"),
        )
        .await
        .expect("insert should succeed")
        .expect("event should be stored");

        let first = storage
            .upsert_translation(
                &stored,
                &NewsTranslationDraft {
                    language: "fr".to_owned(),
                    status: "ready".to_owned(),
                    translated_title: "Powell indique que les taux restent eleves".to_owned(),
                    translated_body: None,
                    narration_text: "10:15. Powell indique que les taux restent eleves.".to_owned(),
                    translation_mode: "heuristic_fr".to_owned(),
                    error_message: None,
                },
            )
            .await
            .expect("translation upsert should succeed");

        let second = storage
            .upsert_translation(
                &stored,
                &NewsTranslationDraft {
                    language: "fr".to_owned(),
                    status: "ready".to_owned(),
                    translated_title: "Powell signale que les taux restent durablement eleves"
                        .to_owned(),
                    translated_body: None,
                    narration_text:
                        "10:15. Powell signale que les taux restent durablement eleves.".to_owned(),
                    translation_mode: "heuristic_fr".to_owned(),
                    error_message: None,
                },
            )
            .await
            .expect("translation update should succeed");

        // Le second upsert doit conserver le meme `id` SQLite tout en remplaçant le contenu.
        assert_eq!(first.news_event_id, stored.id);
        assert_eq!(first.language, "fr");
        assert_eq!(first.published_at, stored.published_at);
        assert_eq!(first.id, second.id);
        assert_eq!(second.published_at, stored.published_at);
        assert_eq!(
            second.translated_title,
            "Powell signale que les taux restent durablement eleves"
        );
    }

    #[tokio::test]
    async fn storage_reuses_daily_summary_by_day_and_hash() {
        let storage = test_storage().await;
        let draft = DailySummaryDraft {
            local_day: "2026-06-01".to_owned(),
            content_hash: "hash-1".to_owned(),
            summary_kind: "ai".to_owned(),
            news_count: 3,
            important_news_count: 2,
            source_urls_json: "[\"https://example.test\"]".to_owned(),
        };

        let first = storage
            .ensure_daily_summary_pending(&draft)
            .await
            .expect("first daily summary should be stored");
        let second = storage
            .ensure_daily_summary_pending(&draft)
            .await
            .expect("second daily summary should be reused");

        assert_eq!(first.id, second.id);
        assert_eq!(second.status, "pending");

        storage
            .mark_daily_summary_summarized(
                first.id,
                "Résumé du jour",
                Some("Contexte du jour."),
                Some("Résumé des nouvelles importantes."),
                "Résumé court.",
                "Résumé narré.",
                Some("openrouter"),
                Some("nvidia/nemotron-3-super-120b-a12b:free"),
                Some(1200),
            )
            .await
            .expect("summary should be marked summarized");
        storage
            .mark_daily_summary_audio_ready(
                first.id,
                "audio-summary-1",
                "/audio/clips/audio-summary-1/playlist.m3u8",
            )
            .await
            .expect("summary should be marked ready");

        let ready = storage
            .get_daily_summary_by_hash("2026-06-01", "hash-1")
            .await
            .expect("lookup should succeed")
            .expect("summary should exist");
        assert_eq!(ready.status, "audio_ready");
        assert_eq!(ready.stream_id.as_deref(), Some("audio-summary-1"));
        assert_eq!(
            ready.daily_context_text.as_deref(),
            Some("Contexte du jour.")
        );
        assert_eq!(
            ready.important_news_summary_text.as_deref(),
            Some("Résumé des nouvelles importantes.")
        );
    }

    #[tokio::test]
    async fn storage_lists_important_daily_qualifications_by_news_time_ascending() {
        let storage = test_storage().await;
        let later = insert_headline_at(
            &storage,
            "financial_juice",
            "Later important headline",
            Some("2026-04-21T12:00:00Z"),
        )
        .await
        .expect("insert should succeed")
        .expect("later event should be stored");
        let earlier = insert_headline_at(
            &storage,
            "financial_juice",
            "Earlier important headline",
            Some("2026-04-21T09:00:00Z"),
        )
        .await
        .expect("insert should succeed")
        .expect("earlier event should be stored");

        for event in [&later, &earlier] {
            storage
                .upsert_daily_news_qualification(&DailyNewsQualificationDraft {
                    local_day: "2026-04-21".to_owned(),
                    news_event_id: event.id,
                    translation_id: None,
                    category: "rates".to_owned(),
                    importance: "high".to_owned(),
                    summary: event.title.clone(),
                    reasoning: "Signal important.".to_owned(),
                    confidence: Some(0.8),
                    is_important: true,
                    provider: Some("openrouter".to_owned()),
                    model: Some("nvidia/nemotron-3-super-120b-a12b:free".to_owned()),
                })
                .await
                .expect("qualification should be stored");
        }

        let important = storage
            .list_important_daily_news_qualifications("2026-04-21")
            .await
            .expect("important qualifications should be listed");

        assert_eq!(important.len(), 2);
        assert_eq!(important[0].news_event_id, earlier.id);
        assert_eq!(important[1].news_event_id, later.id);
    }

    #[tokio::test]
    async fn storage_lists_recent_ready_translations_by_published_at_in_chronological_order() {
        // Ce test verrouille le replay WS: les lignes "ready" sont triees du plus ancien au plus recent
        // dans la fenetre retenue, meme si la lecture SQL se fait d'abord en ordre decroissant.
        let storage = test_storage().await;

        let first = insert_headline_at(
            &storage,
            "financial_juice",
            "First headline",
            Some("2026-04-21T09:00:00Z"),
        )
        .await
        .expect("insert should succeed")
        .expect("event should be stored");
        let second = insert_headline_at(
            &storage,
            "financial_juice",
            "Second headline",
            Some("2026-04-21T12:00:00Z"),
        )
        .await
        .expect("insert should succeed")
        .expect("event should be stored");
        let error_item = insert_headline_at(
            &storage,
            "financial_juice",
            "Error headline",
            Some("2026-04-21T13:00:00Z"),
        )
        .await
        .expect("insert should succeed")
        .expect("event should be stored");
        let third = insert_headline_at(
            &storage,
            "financial_juice",
            "Third headline",
            Some("2026-04-21T10:00:00Z"),
        )
        .await
        .expect("insert should succeed")
        .expect("last insert should be stored");

        storage
            .upsert_translation(
                &first,
                &NewsTranslationDraft {
                    language: "fr".to_owned(),
                    status: "ready".to_owned(),
                    translated_title: "Premier titre".to_owned(),
                    translated_body: None,
                    narration_text: "09:00. Premier titre.".to_owned(),
                    translation_mode: "heuristic_fr".to_owned(),
                    error_message: None,
                },
            )
            .await
            .expect("translation upsert should succeed");
        storage
            .upsert_translation(
                &second,
                &NewsTranslationDraft {
                    language: "fr".to_owned(),
                    status: "ready".to_owned(),
                    translated_title: "Deuxieme titre".to_owned(),
                    translated_body: None,
                    narration_text: "12:00. Deuxieme titre.".to_owned(),
                    translation_mode: "heuristic_fr".to_owned(),
                    error_message: None,
                },
            )
            .await
            .expect("translation upsert should succeed");
        storage
            .upsert_translation(
                &error_item,
                &NewsTranslationDraft {
                    language: "fr".to_owned(),
                    status: "error".to_owned(),
                    translated_title: "Titre en erreur".to_owned(),
                    translated_body: None,
                    narration_text: "13:00. Titre en erreur.".to_owned(),
                    translation_mode: "heuristic_fr".to_owned(),
                    error_message: Some("simulated".to_owned()),
                },
            )
            .await
            .expect("translation upsert should succeed");
        storage
            .upsert_translation(
                &third,
                &NewsTranslationDraft {
                    language: "fr".to_owned(),
                    status: "ready".to_owned(),
                    translated_title: "Troisieme titre".to_owned(),
                    translated_body: None,
                    narration_text: "10:00. Troisieme titre.".to_owned(),
                    translation_mode: "heuristic_fr".to_owned(),
                    error_message: None,
                },
            )
            .await
            .expect("translation upsert should succeed");

        let items = storage
            .list_recent_translations("fr", 2)
            .await
            .expect("listing recent translations should succeed");

        // On attend ici les deux plus recentes "ready", rejouees en ordre chronologique.
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].news_event_id, third.id);
        assert_eq!(items[1].news_event_id, second.id);
        assert!(items.iter().all(|item| item.status == "ready"));
    }

    #[tokio::test]
    async fn storage_gets_a_ready_translation_by_id_and_language() {
        // Ce test verrouille la lecture ciblee utilisee par la resynchronisation audio.
        let storage = test_storage().await;

        let ready_event = insert_headline_at(
            &storage,
            "financial_juice",
            "Ready headline",
            Some("2026-04-21T14:00:00Z"),
        )
        .await
        .expect("insert should succeed")
        .expect("event should be stored");
        let error_event = insert_headline_at(
            &storage,
            "financial_juice",
            "Errored headline",
            Some("2026-04-21T15:00:00Z"),
        )
        .await
        .expect("insert should succeed")
        .expect("event should be stored");

        let ready_translation = storage
            .upsert_translation(
                &ready_event,
                &NewsTranslationDraft {
                    language: "fr".to_owned(),
                    status: "ready".to_owned(),
                    translated_title: "Titre pret".to_owned(),
                    translated_body: None,
                    narration_text: "11:00. Titre pret.".to_owned(),
                    translation_mode: "opus_mt_tc_big_local_session".to_owned(),
                    error_message: None,
                },
            )
            .await
            .expect("ready translation upsert should succeed");
        let error_translation = storage
            .upsert_translation(
                &error_event,
                &NewsTranslationDraft {
                    language: "fr".to_owned(),
                    status: "error".to_owned(),
                    translated_title: "Titre en erreur".to_owned(),
                    translated_body: None,
                    narration_text: "12:00. Titre en erreur.".to_owned(),
                    translation_mode: "opus_mt_tc_big_local_session".to_owned(),
                    error_message: Some("simulated".to_owned()),
                },
            )
            .await
            .expect("error translation upsert should succeed");

        let found = storage
            .get_ready_translation_by_id_and_language(ready_translation.id, "fr")
            .await
            .expect("ready translation fetch should succeed");
        let missing_for_error_status = storage
            .get_ready_translation_by_id_and_language(error_translation.id, "fr")
            .await
            .expect("error translation fetch should succeed");
        let missing_for_wrong_language = storage
            .get_ready_translation_by_id_and_language(ready_translation.id, "en")
            .await
            .expect("wrong-language translation fetch should succeed");

        assert_eq!(
            found.expect("ready translation should exist").id,
            ready_translation.id
        );
        assert!(missing_for_error_status.is_none());
        assert!(missing_for_wrong_language.is_none());
    }

    #[tokio::test]
    async fn storage_lists_translation_history_with_real_feed_and_test_run_filters() {
        // Ce test verrouille la separation durable entre historique reel et historique de test.
        let storage = test_storage().await;

        let real_event = insert_headline_at(
            &storage,
            "financial_juice",
            "Real headline",
            Some("2026-04-23T08:00:00Z"),
        )
        .await
        .expect("insert should succeed")
        .expect("real event should be stored");
        let test_event = insert_headline_with_raw_ref(
            &storage,
            "financial_juice",
            "Test headline",
            Some("2026-04-23T09:00:00Z"),
            Some("test_run:run-123"),
        )
        .await
        .expect("insert should succeed")
        .expect("test event should be stored");

        storage
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
        storage
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

        let real_feed_page = storage
            .list_translation_history(
                &TranslationHistoryParams {
                    limit: 10,
                    source: Some("financial_juice".to_owned()),
                    day: Some("2026-04-23".to_owned()),
                    history_scope: TranslationHistoryScope::RealFeed,
                    before_id: None,
                },
                "fr",
            )
            .await
            .expect("real feed history should load");
        let test_run_page = storage
            .list_translation_history(
                &TranslationHistoryParams {
                    limit: 10,
                    source: Some("financial_juice".to_owned()),
                    day: Some("2026-04-23".to_owned()),
                    history_scope: TranslationHistoryScope::TestRun,
                    before_id: None,
                },
                "fr",
            )
            .await
            .expect("test-run history should load");

        assert_eq!(real_feed_page.items.len(), 1);
        assert_eq!(real_feed_page.items[0].news_event_id, real_event.id);
        assert_eq!(test_run_page.items.len(), 1);
        assert_eq!(test_run_page.items[0].news_event_id, test_event.id);
    }

    #[tokio::test]
    async fn storage_repairs_financial_juice_events_and_linked_translations() {
        // Ce test verrouille la reparation des timestamps backlog FJ sans dupliquer les lignes.
        let storage = test_storage().await;

        let stored_event = insert_headline_with_raw_ref(
            &storage,
            "financial_juice",
            "Morning Juice - Europe Session Prep",
            Some("2026-04-19T05:21:17.27Z"),
            Some("financial_juice_news_id:9552980"),
        )
        .await
        .expect("insert should succeed")
        .expect("event should be stored");

        storage
            .upsert_translation(
                &stored_event,
                &NewsTranslationDraft {
                    language: "fr".to_owned(),
                    status: "ready".to_owned(),
                    translated_title: "Morning Juice Europe".to_owned(),
                    translated_body: None,
                    narration_text: "07:21. Morning Juice Europe.".to_owned(),
                    translation_mode: "opus_mt_tc_big_local_session".to_owned(),
                    error_message: None,
                },
            )
            .await
            .expect("translation upsert should succeed");

        let mut repaired_event = NewsEvent::new_headline(
            "financial_juice".to_owned(),
            "Morning Juice - Europe Session Prep".to_owned(),
            "https://www.financialjuice.com/News/9552980/Morning-Juice---Europe-Session-Prep.aspx"
                .to_owned(),
            Some("2026-04-24T05:21:17.27Z".to_owned()),
            None,
        );
        repaired_event.raw_payload_ref = Some("financial_juice_news_id:9552980".to_owned());
        // La reparation backfill doit aussi enrichir le niveau source retrouve dans la reponse FJ.
        repaired_event.news_level = Some("active active-critical".to_owned());
        repaired_event.recompute_signature();

        storage
            .repair_financial_juice_events(&[repaired_event.clone()])
            .await
            .expect("repair should succeed");

        let repaired_row = storage
            .get_news_by_id(stored_event.id)
            .await
            .expect("read should succeed")
            .expect("row should still exist");
        let repaired_translation = storage
            .get_translation_by_news_event_id_and_language(stored_event.id, "fr")
            .await
            .expect("translation read should succeed")
            .expect("translation should still exist");

        assert_eq!(repaired_row.published_at, repaired_event.published_at);
        assert_eq!(repaired_row.news_level, repaired_event.news_level);
        assert_eq!(repaired_row.signature, repaired_event.signature);
        assert_eq!(
            repaired_translation.published_at,
            repaired_event.published_at
        );
        assert_eq!(
            repaired_translation.source_title,
            "Morning Juice - Europe Session Prep"
        );
    }

    #[tokio::test]
    async fn storage_prunes_news_older_than_current_and_previous_utc_day() {
        // La retention doit garder seulement J et J-1, et nettoyer les traductions avant la ligne source.
        let storage = test_storage().await;

        let today = insert_headline_at(
            &storage,
            "financial_juice",
            "today headline",
            Some("2026-05-05T08:00:00Z"),
        )
        .await
        .expect("today insert should succeed")
        .expect("today event should persist");
        let yesterday = insert_headline_at(
            &storage,
            "financial_juice",
            "yesterday headline",
            Some("2026-05-04T08:00:00Z"),
        )
        .await
        .expect("yesterday insert should succeed")
        .expect("yesterday event should persist");
        let older = insert_headline_at(
            &storage,
            "financial_juice",
            "older headline",
            Some("2026-05-03T08:00:00Z"),
        )
        .await
        .expect("older insert should succeed")
        .expect("older event should persist");

        storage
            .upsert_translation(
                &older,
                &NewsTranslationDraft {
                    language: "fr".to_owned(),
                    status: "ready".to_owned(),
                    translated_title: "Ancienne information".to_owned(),
                    translated_body: None,
                    narration_text: "10:00. Ancienne information.".to_owned(),
                    translation_mode: "opus_mt_tc_big_local_session".to_owned(),
                    error_message: None,
                },
            )
            .await
            .expect("translation upsert should succeed");

        storage
            .upsert_daily_news_qualification(&DailyNewsQualificationDraft {
                local_day: "2026-05-05".to_owned(),
                news_event_id: today.id,
                translation_id: None,
                category: "macro".to_owned(),
                importance: "medium".to_owned(),
                summary: "Qualification conservee.".to_owned(),
                reasoning: "News du jour.".to_owned(),
                confidence: Some(0.7),
                is_important: false,
                provider: Some("openrouter".to_owned()),
                model: Some("nvidia/nemotron-3-super-120b-a12b:free".to_owned()),
            })
            .await
            .expect("today qualification should be stored");
        storage
            .upsert_daily_news_qualification(&DailyNewsQualificationDraft {
                local_day: "2026-05-03".to_owned(),
                news_event_id: older.id,
                translation_id: None,
                category: "macro".to_owned(),
                importance: "high".to_owned(),
                summary: "Qualification ancienne.".to_owned(),
                reasoning: "News J-2.".to_owned(),
                confidence: Some(0.9),
                is_important: true,
                provider: Some("openrouter".to_owned()),
                model: Some("nvidia/nemotron-3-super-120b-a12b:free".to_owned()),
            })
            .await
            .expect("old qualification should be stored");
        storage
            .upsert_daily_news_qualification(&DailyNewsQualificationDraft {
                local_day: "2026-05-05".to_owned(),
                news_event_id: 999_999,
                translation_id: None,
                category: "other".to_owned(),
                importance: "low".to_owned(),
                summary: "Qualification orpheline.".to_owned(),
                reasoning: "News absente.".to_owned(),
                confidence: Some(0.1),
                is_important: false,
                provider: Some("openrouter".to_owned()),
                model: Some("nvidia/nemotron-3-super-120b-a12b:free".to_owned()),
            })
            .await
            .expect("orphan qualification should be stored");

        storage
            .ensure_daily_summary_pending(&DailySummaryDraft {
                local_day: "2026-05-05".to_owned(),
                content_hash: "today-summary".to_owned(),
                summary_kind: "ai".to_owned(),
                news_count: 1,
                important_news_count: 0,
                source_urls_json: "[]".to_owned(),
            })
            .await
            .expect("today summary should be stored");
        storage
            .ensure_daily_summary_pending(&DailySummaryDraft {
                local_day: "2026-05-03".to_owned(),
                content_hash: "old-summary".to_owned(),
                summary_kind: "ai".to_owned(),
                news_count: 1,
                important_news_count: 1,
                source_urls_json: "[]".to_owned(),
            })
            .await
            .expect("old summary should be stored");

        let now = OffsetDateTime::parse("2026-05-05T12:00:00Z", &Rfc3339)
            .expect("fixed timestamp should parse");
        let report = storage
            .prune_news_retention(now, 7)
            .await
            .expect("retention prune should succeed");

        assert_eq!(report.oldest_retained_day, "2026-05-04");
        assert_eq!(report.deleted_news_events, 1);
        assert_eq!(report.deleted_translations, 1);
        assert_eq!(report.deleted_daily_news_qualifications, 2);
        assert_eq!(report.deleted_daily_summaries, 1);
        assert!(storage.get_news_by_id(today.id).await.unwrap().is_some());
        assert!(
            storage
                .get_news_by_id(yesterday.id)
                .await
                .unwrap()
                .is_some()
        );
        assert!(storage.get_news_by_id(older.id).await.unwrap().is_none());
        assert!(
            storage
                .get_translation_by_news_event_id_and_language(older.id, "fr")
                .await
                .unwrap()
                .is_none()
        );
        assert_eq!(
            count_table_rows(&storage, "daily_news_qualifications").await,
            1
        );
        assert_eq!(count_table_rows(&storage, "daily_summaries").await, 1);
    }

    #[tokio::test]
    async fn storage_repair_merges_financial_juice_duplicate_signature_conflicts() {
        // Le live et le backfill peuvent avoir deja persiste deux variantes temporelles d'un meme NewsID.
        // La reparation doit converger vers la signature correcte sans violer l'unicite SQLite.
        let storage = test_storage().await;
        let raw_payload_ref = Some("financial_juice_news_id:9566970");
        let target_event = insert_headline_with_raw_ref(
            &storage,
            "financial_juice",
            "Italian Manufacturing PMI Actual 52.1",
            Some("2026-05-04T08:45:00Z"),
            raw_payload_ref,
        )
        .await
        .expect("target insert should succeed")
        .expect("target event should be stored");

        for filler_title in [
            "German services PMI holds near expansion",
            "France budget deficit update crosses wires",
            "Eurozone investor confidence improves",
            "Oil inventories draw less than expected",
            "US futures steady before cash open",
        ] {
            insert_headline_with_raw_ref(
                &storage,
                "financial_juice",
                filler_title,
                Some("2026-05-04T08:46:00Z"),
                Some("financial_juice_news_id:filler"),
            )
            .await
            .expect("filler insert should succeed")
            .expect("filler event should be stored");
        }

        let duplicate_event = insert_headline_with_raw_ref(
            &storage,
            "financial_juice",
            "Italian Manufacturing PMI Actual 52.1",
            Some("2026-05-03T08:45:00Z"),
            raw_payload_ref,
        )
        .await
        .expect("duplicate insert should succeed")
        .expect("duplicate event should be stored");

        storage
            .upsert_translation(
                &duplicate_event,
                &NewsTranslationDraft {
                    language: "fr".to_owned(),
                    status: "ready".to_owned(),
                    translated_title: "PMI manufacturier italien".to_owned(),
                    translated_body: None,
                    narration_text: "10:45. PMI manufacturier italien.".to_owned(),
                    translation_mode: "opus_mt_tc_big_local_session".to_owned(),
                    error_message: None,
                },
            )
            .await
            .expect("duplicate translation should be stored");

        let mut repaired_event = NewsEvent::new_headline(
            "financial_juice".to_owned(),
            "Italian Manufacturing PMI Actual 52.1".to_owned(),
            "https://example.test/financial_juice/Italian Manufacturing PMI Actual 52.1".to_owned(),
            Some("2026-05-04T08:45:00Z".to_owned()),
            None,
        );
        repaired_event.raw_payload_ref = raw_payload_ref.map(ToOwned::to_owned);
        repaired_event.news_level = Some("news-general".to_owned());
        repaired_event.recompute_signature();

        storage
            .repair_financial_juice_events(&[repaired_event.clone()])
            .await
            .expect("repair should merge duplicate instead of failing");

        let merged_target = storage
            .get_news_by_id(target_event.id)
            .await
            .expect("target read should succeed")
            .expect("target should remain");
        let removed_duplicate = storage
            .get_news_by_id(duplicate_event.id)
            .await
            .expect("duplicate read should succeed");
        let moved_translation = storage
            .get_translation_by_news_event_id_and_language(target_event.id, "fr")
            .await
            .expect("translation read should succeed")
            .expect("translation should move to target");

        assert!(removed_duplicate.is_none());
        assert_eq!(merged_target.signature, repaired_event.signature);
        assert_eq!(merged_target.news_level, repaired_event.news_level);
        assert_eq!(moved_translation.published_at, repaired_event.published_at);
    }

    #[tokio::test]
    async fn storage_lists_news_missing_translation_for_days() {
        // Ce test verrouille le helper de rattrapage qui relit les news brutes encore sans FR.
        let storage = test_storage().await;

        let missing_event = insert_headline_at(
            &storage,
            "financial_juice",
            "Missing translation",
            Some("2026-04-22T00:21:15Z"),
        )
        .await
        .expect("insert should succeed")
        .expect("missing event should be stored");
        let translated_event = insert_headline_at(
            &storage,
            "financial_juice",
            "Already translated",
            Some("2026-04-22T07:41:38Z"),
        )
        .await
        .expect("insert should succeed")
        .expect("translated event should be stored");
        let other_source_event = insert_headline_at(
            &storage,
            "financial_juice_browser",
            "Other source",
            Some("2026-04-22T08:00:00Z"),
        )
        .await
        .expect("insert should succeed")
        .expect("other source event should be stored");

        storage
            .upsert_translation(
                &translated_event,
                &NewsTranslationDraft {
                    language: "fr".to_owned(),
                    status: "ready".to_owned(),
                    translated_title: "Deja traduit".to_owned(),
                    translated_body: None,
                    narration_text: "09:41. Deja traduit.".to_owned(),
                    translation_mode: "opus_mt_tc_big_local_session".to_owned(),
                    error_message: None,
                },
            )
            .await
            .expect("translation upsert should succeed");

        let missing_items = storage
            .list_news_missing_translation_for_days(
                Some("financial_juice"),
                "fr",
                &["2026-04-22".to_owned(), "2026-04-23".to_owned()],
                100,
            )
            .await
            .expect("missing translation query should succeed");

        assert_eq!(missing_items.len(), 1);
        assert_eq!(missing_items[0].id, missing_event.id);
        assert_ne!(missing_items[0].id, translated_event.id);
        assert_ne!(missing_items[0].id, other_source_event.id);
    }

    #[tokio::test]
    async fn storage_migrates_legacy_news_events_with_news_level() {
        // Ce test simule une ancienne base `news_events` creee avant le champ FinancialJuice `Level`.
        let config = StorageConfig {
            database_url: "sqlite::memory:".to_owned(),
            max_connections: 1,
            data_audit_retention_days: 7,
        };
        let storage = SqliteNewsStore::connect(&config)
            .await
            .expect("in-memory sqlite should open");
        let pool = storage.clone_pool_for_feature_store();

        sqlx::query(
            r#"
            CREATE TABLE schema_migrations (
                name TEXT PRIMARY KEY,
                applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            "#,
        )
        .execute(&pool)
        .await
        .expect("legacy migrations table should be created");
        sqlx::query(
            r#"
            CREATE TABLE news_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                source TEXT NOT NULL,
                event_kind TEXT NOT NULL,
                title TEXT NOT NULL,
                url TEXT NOT NULL,
                published_at TEXT,
                event_time TEXT,
                ingested_at TEXT NOT NULL,
                body TEXT,
                importance TEXT,
                country_code TEXT,
                currency TEXT,
                actual_value TEXT,
                forecast_value TEXT,
                previous_value TEXT,
                raw_payload_ref TEXT,
                signature TEXT NOT NULL UNIQUE
            );
            "#,
        )
        .execute(&pool)
        .await
        .expect("legacy news_events table should be created");

        storage
            .migrate()
            .await
            .expect("migration should upgrade legacy schema");

        assert!(
            storage
                .column_exists("news_events", "news_level")
                .await
                .expect("column lookup should succeed")
        );
    }

    #[tokio::test]
    async fn storage_migrates_legacy_news_translations_with_published_at() {
        // Ce test simule une ancienne base sans `published_at` sur `news_translations`.
        let config = StorageConfig {
            database_url: "sqlite::memory:".to_owned(),
            max_connections: 1,
            data_audit_retention_days: 7,
        };
        let storage = SqliteNewsStore::connect(&config)
            .await
            .expect("in-memory sqlite should open");
        let pool = storage.clone_pool_for_feature_store();

        sqlx::query(
            r#"
            CREATE TABLE schema_migrations (
                name TEXT PRIMARY KEY,
                applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            "#,
        )
        .execute(&pool)
        .await
        .expect("legacy migrations table should be created");
        sqlx::query(
            r#"
            CREATE TABLE news_translations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                news_event_id INTEGER NOT NULL,
                language TEXT NOT NULL,
                status TEXT NOT NULL,
                source TEXT NOT NULL,
                event_kind TEXT NOT NULL,
                source_title TEXT NOT NULL,
                source_body TEXT,
                translated_title TEXT NOT NULL,
                translated_body TEXT,
                narration_text TEXT NOT NULL,
                translation_mode TEXT NOT NULL,
                error_message TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                UNIQUE(news_event_id, language)
            );
            "#,
        )
        .execute(&pool)
        .await
        .expect("legacy translations table should be created");

        storage
            .migrate()
            .await
            .expect("migration should upgrade legacy schema");

        assert!(
            storage
                .column_exists("news_translations", "published_at")
                .await
                .expect("column lookup should succeed")
        );
    }

    #[tokio::test]
    async fn storage_migration_drops_obsolete_audio_pipeline_candidates_table() {
        let config = StorageConfig {
            database_url: "sqlite::memory:".to_owned(),
            max_connections: 1,
            data_audit_retention_days: 7,
        };
        let storage = SqliteNewsStore::connect(&config)
            .await
            .expect("in-memory sqlite should open");
        let pool = storage.clone_pool_for_feature_store();

        sqlx::query(
            r#"
            CREATE TABLE news_audio_pipeline_candidates (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                news_event_id INTEGER NOT NULL,
                candidate TEXT NOT NULL,
                status TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            "#,
        )
        .execute(&pool)
        .await
        .expect("obsolete table fixture should be created");

        assert!(table_exists(&storage, "news_audio_pipeline_candidates").await);

        storage
            .migrate()
            .await
            .expect("migration should drop obsolete table");
        storage
            .migrate()
            .await
            .expect("migration should remain idempotent");

        assert!(!table_exists(&storage, "news_audio_pipeline_candidates").await);
        assert!(table_exists(&storage, "news_events").await);
        assert!(table_exists(&storage, "daily_summaries").await);
    }

    async fn test_storage() -> SqliteNewsStore {
        // Helper commun: base SQLite en memoire, migree comme en prod.
        let config = StorageConfig {
            database_url: "sqlite::memory:".to_owned(),
            max_connections: 1,
            data_audit_retention_days: 7,
        };
        let storage = SqliteNewsStore::connect(&config)
            .await
            .expect("in-memory sqlite should open");
        storage
            .migrate()
            .await
            .expect("schema migration should succeed");
        storage
    }

    async fn insert_headline(
        storage: &SqliteNewsStore,
        source: &str,
        title: &str,
    ) -> Result<Option<crate::features::news::domain::PersistedNewsEvent>, crate::errors::AppError>
    {
        // Ce helper specialise simplement le helper plus general sans `published_at`.
        insert_headline_at(storage, source, title, None).await
    }

    async fn insert_headline_at(
        storage: &SqliteNewsStore,
        source: &str,
        title: &str,
        published_at: Option<&str>,
    ) -> Result<Option<crate::features::news::domain::PersistedNewsEvent>, crate::errors::AppError>
    {
        // Les fixtures passent toujours par `NewsEvent::new_headline` pour rester alignees sur le modele canonique.
        let event = NewsEvent::new_headline(
            source.to_owned(),
            title.to_owned(),
            format!("https://example.test/{source}/{title}"),
            published_at.map(ToOwned::to_owned),
            None,
        );

        // L'insertion repository reste le seul point d'ecriture teste.
        storage.insert_event(&event).await
    }

    async fn insert_headline_with_raw_ref(
        storage: &SqliteNewsStore,
        source: &str,
        title: &str,
        published_at: Option<&str>,
        raw_payload_ref: Option<&str>,
    ) -> Result<Option<crate::features::news::domain::PersistedNewsEvent>, crate::errors::AppError>
    {
        // Ce helper permet de simuler explicitement un run de test synthetique persiste.
        let mut event = NewsEvent::new_headline(
            source.to_owned(),
            title.to_owned(),
            format!("https://example.test/{source}/{title}"),
            published_at.map(ToOwned::to_owned),
            None,
        );
        event.raw_payload_ref = raw_payload_ref.map(ToOwned::to_owned);
        event.recompute_signature();

        storage.insert_event(&event).await
    }

    async fn count_table_rows(storage: &SqliteNewsStore, table_name: &str) -> i64 {
        let sql = format!("SELECT COUNT(*) FROM {table_name}");
        let pool = storage.clone_pool_for_feature_store();
        sqlx::query_scalar::<_, i64>(&sql)
            .fetch_one(&pool)
            .await
            .expect("row count should succeed")
    }

    async fn table_exists(storage: &SqliteNewsStore, table_name: &str) -> bool {
        let pool = storage.clone_pool_for_feature_store();
        sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*)
            FROM sqlite_master
            WHERE type = 'table'
              AND name = ?1;
            "#,
        )
        .bind(table_name)
        .fetch_one(&pool)
        .await
        .expect("table lookup should succeed")
            > 0
    }
}

mod domain_tests {
    // Les tests de marqueur horaire utilisent un offset fixe pour rester independants du fuseau local.
    use time::{OffsetDateTime, UtcOffset, format_description::well_known::Rfc3339};

    // On importe seulement les types touches par les tests de ce module.
    use crate::features::news::domain::{
        NewsListParams, PersistedNewsEvent, build_news_narration_prefix,
        build_news_narration_prefix_from_timestamp, format_clock_label_for_offset,
        news_level_contains_active_word, normalize_news_narration_prefix,
    };

    fn test_source_event_with_level(news_level: Option<&str>) -> PersistedNewsEvent {
        // Ce helper concentre les champs obligatoires d'un event source pour tester les prefixes live.
        PersistedNewsEvent {
            id: 7,
            source: "financial_juice".to_owned(),
            event_kind: "headline".to_owned(),
            title: "Critical source".to_owned(),
            url: "https://example.test/item".to_owned(),
            published_at: Some("2026-04-21T10:00:00Z".to_owned()),
            event_time: None,
            ingested_at: "2026-04-21T10:00:01Z".to_owned(),
            body: None,
            // Le niveau reste parametre pour couvrir les variantes active/non active.
            news_level: news_level.map(ToOwned::to_owned),
            importance: None,
            country_code: None,
            currency: None,
            actual_value: None,
            forecast_value: None,
            previous_value: None,
            raw_payload_ref: None,
            signature: "critical-signature".to_owned(),
        }
    }

    #[test]
    fn news_list_params_normalizes_filters_and_limits() {
        // Ce test verrouille la normalisation des query params HTTP vers le modele interne.
        let params = NewsListParams::new(
            Some(999),
            Some(" Financial_Juice ".to_owned()),
            Some(" HEADLINE ".to_owned()),
            Some(42),
        );

        // La limite est bornee a 500 et les filtres sont nettoyes en minuscules.
        assert_eq!(params.limit, 500);
        assert_eq!(params.source.as_deref(), Some("financial_juice"));
        assert_eq!(params.event_kind.as_deref(), Some("headline"));
        assert_eq!(params.before_id, Some(42));
    }

    #[test]
    fn format_clock_label_for_offset_projects_timestamp_to_hour_and_minute() {
        // `UtcOffset::from_hms` rend explicite l'offset cible du test au lieu de dependre du fuseau local.
        let offset = UtcOffset::from_hms(2, 0, 0).expect("offset should be valid");
        // On parse un RFC3339 complet pour coller exactement au format utilise par le pipeline reel.
        let parsed = OffsetDateTime::parse("2026-04-23T11:13:00Z", &Rfc3339)
            .expect("timestamp should parse");

        // L'assertion verrouille la projection `UTC -> UTC+2` qui pilote le marqueur horaire.
        assert_eq!(format_clock_label_for_offset(parsed, offset), "13:13");
    }

    #[test]
    fn news_level_contains_active_word_uses_word_boundaries() {
        // La valeur critique observee doit activer le prefixe information importante.
        assert!(news_level_contains_active_word(Some(
            "active active-critical"
        )));
        // Un niveau general doit rester dans le libelle standard.
        assert!(!news_level_contains_active_word(Some("news-general")));
        // Une sous-chaine comme `inactive` ne doit pas etre confondue avec le mot `active`.
        assert!(!news_level_contains_active_word(Some("inactive")));
        // L'absence de niveau garde le libelle standard.
        assert!(!news_level_contains_active_word(None));
    }

    #[test]
    fn narration_prefix_from_timestamp_uses_business_label() {
        // Un timestamp invalide force le fallback horaire, ce qui rend le test independant du fuseau local.
        assert_eq!(
            build_news_narration_prefix_from_timestamp("not-a-date", Some("active")),
            "00:00. Information importante"
        );
        assert_eq!(
            build_news_narration_prefix_from_timestamp("not-a-date", Some("news-general")),
            "00:00. Nouvelle information"
        );
    }

    #[test]
    fn narration_prefix_uses_europe_paris_time_independent_from_server_timezone() {
        // Le timestamp observe en production doit sortir en heure d'ete Europe/Paris, pas en UTC Docker.
        assert_eq!(
            build_news_narration_prefix_from_timestamp(
                "2026-05-04T19:21:46.967Z",
                Some("news-general"),
            ),
            "21:21. Nouvelle information"
        );
        // Le meme contrat couvre l'heure d'hiver pour eviter de figer un offset +2 toute l'annee.
        assert_eq!(
            build_news_narration_prefix_from_timestamp("2026-01-04T19:21:46Z", Some("active")),
            "20:21. Information importante"
        );
    }

    #[test]
    fn important_prefix_is_idempotent_for_active_news() {
        // Une traduction deja enrichie ne doit jamais dupliquer le libelle important.
        let source_event = test_source_event_with_level(Some("active"));
        let expected_prefix = build_news_narration_prefix(&source_event);
        let narration = format!("{expected_prefix}. Source critique.");
        let normalized = normalize_news_narration_prefix(narration.clone(), Some(&source_event));

        assert_eq!(normalized, narration);
    }

    #[test]
    fn important_prefix_repairs_legacy_clock_then_important_order() {
        // Cette regression couvre le format historique `HH:MM. texte` sans libelle metier.
        let source_event = test_source_event_with_level(Some("active"));
        let expected_prefix = build_news_narration_prefix(&source_event);
        let clock_label = expected_prefix
            .strip_suffix(". Information importante")
            .expect("active prefix should end with important marker");
        let legacy_narration = format!("{clock_label}. Source critique.");
        let normalized = normalize_news_narration_prefix(legacy_narration, Some(&source_event));

        assert_eq!(normalized, format!("{expected_prefix}. Source critique."));
    }

    #[test]
    fn important_prefix_repairs_legacy_important_before_clock_order() {
        // Cette regression couvre l'ancien mauvais ordre `Important HH:MM. texte`.
        let source_event = test_source_event_with_level(Some("active"));
        let expected_prefix = build_news_narration_prefix(&source_event);
        let clock_label = expected_prefix
            .strip_suffix(". Information importante")
            .expect("active prefix should end with important marker");
        let legacy_narration = format!("Important {clock_label}. Source critique.");
        let normalized = normalize_news_narration_prefix(legacy_narration, Some(&source_event));

        assert_eq!(normalized, format!("{expected_prefix}. Source critique."));
    }

    #[test]
    fn standard_prefix_repairs_legacy_clock_only_narration() {
        // Les anciennes narrations non actives doivent recevoir le meme libelle que l'intro audio.
        let source_event = test_source_event_with_level(Some("news-general"));
        let expected_prefix = build_news_narration_prefix(&source_event);
        let clock_label = expected_prefix
            .strip_suffix(". Nouvelle information")
            .expect("standard prefix should end with standard marker");
        let legacy_narration = format!("{clock_label}. Source standard.");
        let normalized = normalize_news_narration_prefix(legacy_narration, Some(&source_event));

        assert_eq!(normalized, format!("{expected_prefix}. Source standard."));
    }
}

mod http_tests {
    // Ces imports donnent acces aux domaines et fixtures necessaires aux tests HTTP news.
    use crate::{
        features::test_support::{app_state, seed_headline},
        features::{ingestion::runtime::BrowserCollectorStatus, news::domain::EventKind},
    };

    // On importe ici directement les handlers news testes.
    use crate::features::news::http::{NewsQuery, get_news, list_news, list_sources};
    // Les tests reutilisent les memes extracteurs `axum` que le runtime pour rester au plus pres
    // de la vraie signature des handlers.
    use axum::{
        Json,
        extract::{Path, Query, State},
    };

    #[tokio::test]
    async fn list_news_enforces_limit_and_filters() {
        // On construit un etat applicatif minimal mais realiste avec SQLite en memoire.
        let state = app_state().await;
        // On seed plusieurs headlines pour verifier le filtre `source` et la limite normalisee.
        let first = seed_headline(&state, "financial_juice", "first headline").await;
        let _second = seed_headline(&state, "financial_juice_browser", "second headline").await;

        // On appelle directement le handler avec les extracteurs qu'`axum` fournirait en prod.
        let Json(response) = list_news(
            State(state.clone()),
            Query(NewsQuery {
                limit: Some(1_000),
                source: Some("financial_juice".to_owned()),
                event_kind: Some(EventKind::Headline.as_str().to_owned()),
                before_id: None,
            }),
        )
        .await
        .expect("list_news should succeed");

        // Les assertions expliquent le contrat attendu:
        // - la limite est bornee par le modele,
        // - le filtrage source fonctionne,
        // - l'item retourne est bien le bon.
        assert_eq!(response.items.len(), 1);
        assert!(!response.has_more);
        assert_eq!(response.items[0].id, first.id);
    }

    #[tokio::test]
    async fn get_news_returns_single_item() {
        // Meme logique: on prepare un etat local, puis on seed un seul evenement cible.
        let state = app_state().await;
        let expected = seed_headline(&state, "financial_juice", "detail headline").await;

        // `Path(expected.id)` simule la valeur extraite depuis l'URL `/api/news/{id}`.
        let Json(item) = get_news(State(state), Path(expected.id))
            .await
            .expect("get_news should succeed");

        // On verifie que le handler retourne bien l'evenement demande, sans mutation surprise.
        assert_eq!(item.id, expected.id);
        assert_eq!(item.title, "detail headline");
    }

    #[tokio::test]
    async fn list_sources_disables_browser_when_collector_is_blocked() {
        // La vue admin `/api/sources` doit voir l'etat effectif du fallback.
        let state = app_state().await;
        state.financial_juice_runtime.set_enabled(false);
        state.financial_juice_browser_runtime.update_status(
            BrowserCollectorStatus::Blocked,
            Some("cloudflare challenge".to_owned()),
        );

        let Json(statuses) = list_sources(State(state)).await;
        let browser_status = statuses
            .iter()
            .find(|source| source.name == "financial_juice_browser")
            .expect("browser source should be listed");

        assert!(!browser_status.enabled);
        assert_eq!(browser_status.status.as_deref(), Some("blocked"));
        assert_eq!(browser_status.service_accessible, Some(true));
        assert_eq!(browser_status.main_feed_available, Some(false));
        assert_eq!(browser_status.stale, Some(false));
        assert_eq!(
            browser_status.message.as_deref(),
            Some("cloudflare challenge")
        );
    }

    #[tokio::test]
    async fn list_sources_disables_browser_without_collector_heartbeat() {
        // Si FinancialJuice primaire tombe et que le process browser ne heartbeat pas, le detail admin
        // doit exposer une raison explicite au lieu de paraitre actif par simple fallback.
        let state = app_state().await;
        state.financial_juice_runtime.set_enabled(false);

        let Json(statuses) = list_sources(State(state)).await;
        let browser_status = statuses
            .iter()
            .find(|source| source.name == "financial_juice_browser")
            .expect("browser source should be listed");

        assert!(!browser_status.enabled);
        assert_eq!(browser_status.status.as_deref(), Some("stopped"));
        assert_eq!(browser_status.service_accessible, Some(false));
        assert_eq!(browser_status.main_feed_available, Some(false));
        assert_eq!(browser_status.stale, Some(false));
        assert!(
            browser_status
                .message
                .as_deref()
                .unwrap_or_default()
                .contains("has not reported status")
        );
    }
}

mod retention_tests {
    use crate::features::news::retention::duration_until_next_daily_run;
    use time::{OffsetDateTime, format_description::well_known::Rfc3339};

    #[test]
    fn daily_retention_waits_until_today_two_am_when_not_reached() {
        let now = OffsetDateTime::parse("2026-05-05T01:30:00+02:00", &Rfc3339)
            .expect("fixture timestamp should parse");

        assert_eq!(duration_until_next_daily_run(now).as_secs(), 30 * 60);
    }

    #[test]
    fn daily_retention_waits_until_tomorrow_two_am_after_threshold() {
        let now = OffsetDateTime::parse("2026-05-05T03:15:00+02:00", &Rfc3339)
            .expect("fixture timestamp should parse");

        assert_eq!(
            duration_until_next_daily_run(now).as_secs(),
            (22 * 60 + 45) * 60
        );
    }
}
