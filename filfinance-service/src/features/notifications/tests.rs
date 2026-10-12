//! Tests d'architecture notifications reserves aux prochaines extractions.

mod telegram_tests {
    // `AtomicUsize` simule un compteur de tentatives partage entre closures async de test.
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    use tokio::sync::{broadcast, mpsc, watch};

    // Les tests montent un `AppState` minimal pour verifier la politique de retry
    // sans dependre d'un vrai serveur Telegram.
    use crate::{
        app_state::AppState,
        config::{
            AppConfig, AuthConfig, CalendarDayConfig, EdgeConfig, FinancialJuiceBrowserConfig,
            FinancialJuiceSiteConfig, HlsConfig, HttpConfig, LiveFanoutConfig, LoggingConfig,
            RuntimeConfig, SourceConfig, SourcesConfig, StorageConfig, TelegramConfig,
            TranslationConfig, TtsConfig, XConfig,
        },
        features::audio::hls::HlsOriginState,
        features::audio::tts::TtsRuntimeState,
        features::calendar_day::CalendarDayStore,
        features::ingestion::runtime::{
            FinancialJuiceBrowserRuntimeState, FinancialJuiceRuntimeState, XRuntimeState,
        },
        features::ingestion::x_settings::XSettingsStore,
        features::live::dto::LiveNewsItem,
        features::news::NewsStore,
        features::translation::TranslationRuntimeState,
    };

    use crate::features::notifications::telegram::{
        TelegramSendError, build_telegram_message, execute_send_with_retry,
    };

    #[test]
    fn build_telegram_message_keeps_format_stable() {
        // Ce test verrouille le contrat texte visible par l'utilisateur et les downstreams support.
        let message = build_telegram_message(&test_live_item());

        assert!(message.contains("[financial_juice:headline][fr]"));
        assert!(message.contains("EN: Headline title"));
        assert!(message.contains("FR: Titre traduit"));
        assert!(message.contains("Audio FR: 08:00. Titre traduit."));
        assert!(message.contains("timestamp: 2026-04-21T08:00:00Z"));
        assert!(message.contains("https://example.test/news"));
    }

    #[tokio::test]
    async fn send_news_item_retries_on_server_error() {
        // On simule une premiere erreur temporaire puis un succes pour verifier le retry borne.
        let attempts = Arc::new(AtomicUsize::new(0));
        let state = test_state(
            "https://api.telegram.test".to_owned(),
            vec!["chat-a".to_owned()],
        )
        .await;
        let mut last_http_send_at = None;

        execute_send_with_retry(&state, "chat-a", &mut last_http_send_at, || {
            let attempts = attempts.clone();
            async move {
                let attempt = attempts.fetch_add(1, Ordering::SeqCst);
                if attempt == 0 {
                    Err(TelegramSendError::Retriable {
                        message: "temporary gateway failure".to_owned(),
                        retry_after_ms: Some(0),
                    })
                } else {
                    Ok(())
                }
            }
        })
        .await
        .expect("telegram send should eventually succeed");

        assert_eq!(attempts.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn send_news_item_stops_after_bounded_retry_budget() {
        // Ici la closure echoue toujours: on doit arreter apres le budget configure, pas boucler sans fin.
        let attempts = Arc::new(AtomicUsize::new(0));
        let state = test_state(
            "https://api.telegram.test".to_owned(),
            vec!["chat-a".to_owned()],
        )
        .await;
        let mut last_http_send_at = None;

        let error = execute_send_with_retry(&state, "chat-a", &mut last_http_send_at, || {
            let attempts = attempts.clone();
            async move {
                attempts.fetch_add(1, Ordering::SeqCst);
                Err(TelegramSendError::Retriable {
                    message: "temporary failure".to_owned(),
                    retry_after_ms: Some(0),
                })
            }
        })
        .await
        .expect_err("retry budget should be exhausted");

        assert_eq!(attempts.load(Ordering::SeqCst), 3);
        assert!(
            error.to_string().contains("temporary failure"),
            "downstream error should keep the last telegram context"
        );
    }

    #[tokio::test]
    async fn send_news_item_stops_immediately_on_permanent_error() {
        // Une erreur permanente ne doit pas consommer de retries inutiles.
        let attempts = Arc::new(AtomicUsize::new(0));
        let state = test_state(
            "https://api.telegram.test".to_owned(),
            vec!["chat-a".to_owned()],
        )
        .await;
        let mut last_http_send_at = None;

        let error = execute_send_with_retry(&state, "chat-a", &mut last_http_send_at, || {
            let attempts = attempts.clone();
            async move {
                attempts.fetch_add(1, Ordering::SeqCst);
                Err(TelegramSendError::Permanent(
                    "forbidden bot access".to_owned(),
                ))
            }
        })
        .await
        .expect_err("permanent error should fail immediately");

        assert_eq!(attempts.load(Ordering::SeqCst), 1);
        assert!(
            error.to_string().contains("forbidden bot access"),
            "permanent downstream error should be preserved"
        );
    }

    async fn test_state(api_base_url: String, chat_ids: Vec<String>) -> AppState {
        // Le montage du state reproduit les dependencies reelles du worker tout en restant en memoire.
        let config = Arc::new(AppConfig {
            runtime: RuntimeConfig {
                server_env: crate::config::ServerEnvironment::Local,
                listen_addr: "127.0.0.1:0".to_owned(),
                ingest_buffer: 32,
                broadcast_buffer: 16,
                dedup_ttl_secs: 300,
                slow_request_threshold_ms: 750,
            },
            storage: StorageConfig {
                database_url: "sqlite::memory:".to_owned(),
                max_connections: 1,
                data_audit_retention_days: 7,
            },
            logging: LoggingConfig {
                file_enabled: false,
                directory: "data/test-logs".to_owned(),
                retention_days: 2,
                admin_max_lines: 100,
            },
            http: HttpConfig {
                // La fixture Telegram reutilise le nom runtime pour rester coherente avec la config.
                user_agent: "filfinance-service-test".to_owned(),
                connect_timeout_ms: 1_000,
            },
            sources: SourcesConfig {
                financial_juice: FinancialJuiceSiteConfig {
                    name: "financial_juice".to_owned(),
                    enabled: true,
                    home_url: "https://www.financialjuice.com/home".to_owned(),
                    poll_interval_ms: 5_000,
                    request_timeout_ms: 5_000,
                    rate_limit_backoff_ms: 15_000,
                    session_refresh_interval_ms: 30 * 60 * 1_000,
                    live_retry_backoff_ms: 5_000,
                    startup_backfill_page_limit: 48,
                    time_offset_minutes: 0,
                    email: Some("demo@example.com".to_owned()),
                    password: Some("secret".to_owned()),
                },
                financial_juice_browser: FinancialJuiceBrowserConfig {
                    source: SourceConfig {
                        name: "financial_juice_browser".to_owned(),
                        enabled: true,
                        url: "https://www.financialjuice.com/home".to_owned(),
                        poll_interval_ms: 5_000,
                        request_timeout_ms: 15_000,
                        rate_limit_backoff_ms: 60_000,
                    },
                    ingest_token: Some("test-browser-token".to_owned()),
                    max_batch_size: 100,
                },
                x: XConfig {
                    source: SourceConfig {
                        name: "x".to_owned(),
                        enabled: false,
                        url: "https://api.x.com/2/tweets/search/recent".to_owned(),
                        poll_interval_ms: 15_000,
                        request_timeout_ms: 5_000,
                        rate_limit_backoff_ms: 60_000,
                    },
                    bearer_token: None,
                    query: None,
                    account_usernames: vec!["federalreserve".to_owned()],
                    keywords: vec!["Fed".to_owned(), "rates".to_owned()],
                    language: Some("en".to_owned()),
                    exclude_reposts: true,
                    max_results: 20,
                    startup_lookback_minutes: 120,
                },
            },
            translation: TranslationConfig {
                enabled: true,
                source_language: "en".to_owned(),
                target_language: "fr".to_owned(),
                provider: "heuristic_fr".to_owned(),
                profile: "quality".to_owned(),
                worker_count: 1,
                local_command_program: None,
                local_command_args: Vec::new(),
                local_command_timeout_ms: 5_000,
                warmup_enabled: false,
                warmup_source_text: "Federal Reserve signals rates stay higher for longer"
                    .to_owned(),
                max_body_chars: 280,
            },
            tts: TtsConfig {
                enabled: false,
                provider: "piper_local_session".to_owned(),
                voice: "fr_FR-upmc-medium-jessica".to_owned(),
                local_command_program: "python3".to_owned(),
                local_command_args: vec!["scripts/piper_local_session.py".to_owned()],
                models_dir: "data/models/piper".to_owned(),
                worker_count: 1,
                rate_words_per_minute: 185,
                output_sample_rate_hz: 22_050,
                command_timeout_ms: 12_000,
                warmup_enabled: false,
                warmup_text: "Test audio. Les clips HLS audio sont operationnels.".to_owned(),
            },
            hls: HlsConfig::disabled_for_tests(),
            telegram: TelegramConfig {
                enabled: true,
                api_base_url,
                bot_token: Some("test-token".to_owned()),
                chat_ids,
                queue_buffer: 16,
                rate_limit_ms: 0,
                retry_attempts: 2,
                retry_backoff_ms: 1,
            },
            // Le publisher edge reste hors scope des tests Telegram.
            edge: EdgeConfig::disabled_for_tests(),
            // La fixture Telegram reste entierement locale et ne contacte pas Redis.
            live_fanout: LiveFanoutConfig {
                enabled: false,
                redis_url: None,
                redis_channel: "filfinance:test:live".to_owned(),
                redis_snapshot_key: "filfinance:test:live:snapshot".to_owned(),
            },
            // Le calendrier du jour reste hors scope des tests Telegram.
            calendar_day: CalendarDayConfig::disabled_for_tests(),
            // Le worker Telegram n'exerce pas le middleware HTTP; on garde l'auth desactivee en fixture.
            auth: AuthConfig::disabled_for_tests(),
        });
        let storage = NewsStore::connect(&config.storage)
            .await
            .expect("storage should open");
        storage
            .migrate()
            .await
            .expect("storage migration should succeed");
        let calendar_day = CalendarDayStore::from_news_store(&storage);
        calendar_day
            .migrate()
            .await
            .expect("calendar day schema migration should succeed");
        let (ingest_tx, _ingest_rx) = mpsc::channel(config.runtime.ingest_buffer);
        // Cette fixture utilise le meme keepalive que les tests transverses afin que les taches
        // calendrier eventuelles ne confondent pas la fin de construction avec un shutdown.
        let (shutdown_test_keepalive, shutdown_rx) = watch::channel(false);
        let (broadcast_tx, _) = broadcast::channel(config.runtime.broadcast_buffer);
        let (translated_broadcast_tx, _) = broadcast::channel(config.runtime.broadcast_buffer);
        let (live_metadata_tx, _) = broadcast::channel(config.runtime.broadcast_buffer);
        let (test_trace_tx, _) = broadcast::channel(config.runtime.broadcast_buffer);
        let (calendar_day_admin_tx, _) = broadcast::channel(config.runtime.broadcast_buffer);
        let (calendar_day_audio_tx, _calendar_day_audio_rx) =
            mpsc::channel(config.runtime.ingest_buffer);
        let (calendar_day_native, _calendar_day_native_rx) =
            crate::features::calendar_day::native::runtime::CalendarNativeHandle::channel(
                false,
                config.calendar_day.native.queue_capacity,
            );
        let tts_runtime = Arc::new(TtsRuntimeState::new(&config.tts));
        let hls_origin = Arc::new(HlsOriginState::from_config(&config.hls));
        let translation_runtime = Arc::new(TranslationRuntimeState::new(&config.translation));
        let financial_juice_runtime = Arc::new(FinancialJuiceRuntimeState::new(
            config.sources.financial_juice.enabled,
        ));
        let financial_juice_browser_runtime = Arc::new(FinancialJuiceBrowserRuntimeState::new(
            config.sources.financial_juice_browser.source.enabled,
        ));
        let x_runtime = Arc::new(XRuntimeState::new(&config.sources.x));
        let x_settings_store = XSettingsStore::from_news_store(&storage);
        x_settings_store
            .migrate()
            .await
            .expect("X settings migration should succeed");

        AppState {
            calendar_logs: crate::features::calendar_logs::CalendarLogs::disabled(),
            config,
            news: storage,
            calendar_day,
            calendar_day_native,
            tts_runtime,
            hls_origin,
            translation_runtime,
            financial_juice_runtime,
            financial_juice_browser_runtime,
            x_runtime,
            x_settings_store,
            ingest_tx,
            shutdown_rx,
            shutdown_test_keepalive,
            broadcast_tx,
            translated_broadcast_tx,
            live_metadata_tx,
            live_fanout: None,
            test_trace_tx,
            calendar_day_admin_tx,
            calendar_day_audio_tx,
        }
    }

    fn test_live_item() -> LiveNewsItem {
        LiveNewsItem {
            id: 11,
            news_event_id: 1,
            language: "fr".to_owned(),
            status: "ready".to_owned(),
            source: "financial_juice".to_owned(),
            event_kind: "headline".to_owned(),
            source_title: "Headline title".to_owned(),
            source_body: Some("Body line".to_owned()),
            translated_title: "Titre traduit".to_owned(),
            translated_body: None,
            narration_text: "08:00. Titre traduit.".to_owned(),
            translation_mode: "heuristic_fr".to_owned(),
            error_message: None,
            // Le message Telegram ignore aujourd'hui le niveau, mais la fixture doit respecter le DTO complet.
            news_level: None,
            test_run_id: None,
            url: Some("https://example.test/news".to_owned()),
            timestamp: "2026-04-21T08:00:00Z".to_owned(),
            created_at: "2026-04-21T08:00:01Z".to_owned(),
            updated_at: "2026-04-21T08:00:01Z".to_owned(),
        }
    }
}
