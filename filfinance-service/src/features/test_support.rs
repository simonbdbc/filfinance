//! Fixtures transverses reservees aux tests de features.

// `Arc` reproduit la possession partagee de la configuration runtime.
use std::sync::Arc;

// Les channels Tokio permettent de fabriquer un `AppState` coherent sans lancer les workers.
use tokio::sync::{broadcast, mpsc, watch};

use crate::{
    app_state::AppState,
    config::{
        AppConfig, AuthConfig, CalendarDayConfig, EdgeConfig, FinancialJuiceBrowserConfig,
        FinancialJuiceSiteConfig, HlsConfig, HttpConfig, LiveFanoutConfig, LoggingConfig,
        RuntimeConfig, ServerEnvironment, SourceConfig, SourcesConfig, StorageConfig,
        TelegramConfig, TranslationConfig, TtsConfig, XConfig,
    },
    features::{
        audio::{hls::HlsOriginState, tts::TtsRuntimeState},
        calendar_day::CalendarDayStore,
        ingestion::runtime::{
            FinancialJuiceBrowserRuntimeState, FinancialJuiceRuntimeState, XRuntimeState,
        },
        ingestion::x_settings::XSettingsStore,
        news::{
            NewsStore,
            domain::{NewsEvent, PersistedNewsEvent},
        },
        translation::TranslationRuntimeState,
    },
};

/// Construit un `AppState` complet, local et sans integrations externes.
pub(crate) async fn app_state() -> AppState {
    // La config est volontairement proche de la prod tout en restant hermetique aux reseaux externes.
    let config = Arc::new(AppConfig {
        runtime: RuntimeConfig {
            server_env: ServerEnvironment::Local,
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
            // La fixture garde un user-agent realiste du service pour couvrir les tests HTTP.
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
            warmup_source_text: "Federal Reserve signals rates stay higher for longer".to_owned(),
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
            enabled: false,
            api_base_url: "https://api.telegram.org".to_owned(),
            bot_token: None,
            chat_ids: Vec::new(),
            queue_buffer: 16,
            rate_limit_ms: 250,
            retry_attempts: 2,
            retry_backoff_ms: 750,
        },
        // Le publisher edge n'est pas exerce par ces tests unitaires.
        edge: EdgeConfig::disabled_for_tests(),
        // Le relais Redis n'est pas demarre par les fixtures unitaires en memoire.
        live_fanout: LiveFanoutConfig {
            enabled: false,
            redis_url: None,
            redis_channel: "filfinance:test:live".to_owned(),
            redis_snapshot_key: "filfinance:test:live:snapshot".to_owned(),
        },
        // Le calendrier du jour n'est pas exerce par les fixtures transverses.
        calendar_day: CalendarDayConfig::disabled_for_tests(),
        // Les handlers sont appeles directement; l'auth serveur est donc neutralisee.
        auth: AuthConfig::disabled_for_tests(),
    });

    // Le store news en memoire evite toute ecriture disque et garde les tests rapides.
    let news = NewsStore::connect(&config.storage)
        .await
        .expect("in-memory sqlite should open");
    news.migrate()
        .await
        .expect("news schema migration should succeed");

    let calendar_day = CalendarDayStore::from_news_store(&news);
    calendar_day
        .migrate()
        .await
        .expect("calendar day schema migration should succeed");

    // Ces channels fabriquent un `AppState` complet, meme si le test n'utilise pas tous les bus.
    let (ingest_tx, _ingest_rx) = mpsc::channel(config.runtime.ingest_buffer);
    // Le sender est conserve dans `AppState` seulement en test: les watchdogs doivent attendre
    // leur timeout reel et non voir le canal ferme comme un SIGTERM.
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
    let x_settings_store = XSettingsStore::from_news_store(&news);
    x_settings_store
        .migrate()
        .await
        .expect("X settings migration should succeed");

    AppState {
        calendar_logs: crate::features::calendar_logs::CalendarLogs::disabled(),
        config,
        news,
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

/// Insere une headline canonique et retourne la ligne persistente.
pub(crate) async fn seed_headline(
    state: &AppState,
    source: &str,
    title: &str,
) -> PersistedNewsEvent {
    // `new_headline` force les tests a passer par le modele canonique du service.
    let event = NewsEvent::new_headline(
        source.to_owned(),
        title.to_owned(),
        format!("https://example.test/{source}/{title}"),
        None,
        None,
    );

    // `insert_event` retourne `Option` car la dedup peut ignorer un doublon.
    state
        .news
        .insert_event(&event)
        .await
        .expect("insert should succeed")
        .expect("event should be stored")
}

/// Insere une headline rattachee a un run de test E2E synthetique.
pub(crate) async fn seed_test_run_headline(
    state: &AppState,
    source: &str,
    title: &str,
) -> PersistedNewsEvent {
    // La reference brute garde le format attendu par `extract_test_run_id`.
    let mut event = NewsEvent::new_headline(
        source.to_owned(),
        title.to_owned(),
        format!("https://example.test/{source}/{title}"),
        None,
        None,
    );
    event.raw_payload_ref = Some("test_run:run-api-test".to_owned());
    event.recompute_signature();

    state
        .news
        .insert_event(&event)
        .await
        .expect("insert should succeed")
        .expect("event should be stored")
}
