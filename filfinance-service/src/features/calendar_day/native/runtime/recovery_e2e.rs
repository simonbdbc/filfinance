//! Recuperation de bout en bout du calendrier natif.
//!
//! Le scenario reste hermetique: les collecteurs et OpenRouter sont des
//! doublures typees locales, aucun workflow externe, secret ou appel sortant n'est requis.
//! La seule horloge qui pilote leases et reprises est fixe, ce qui rend le
//! passage crash/restart deterministe meme lorsque les horodatages d'audit
//! internes sont generes par SQLite.

use std::{path::PathBuf, sync::Arc, time::Duration};

use crate::features::calendar_day::native::feeds::{
    self, CalendarFeedDocument, CalendarFeedSource,
};
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
    routing::{get, post},
};
use futures_util::StreamExt;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use time::{Duration as TimeDuration, OffsetDateTime};
use tokio::{
    net::TcpListener,
    sync::{broadcast, mpsc, oneshot, watch},
    time::timeout,
};
use tokio_tungstenite::{connect_async, tungstenite::Message as ClientMessage};
use tower::ServiceExt;
use url::Url;

use crate::{
    app_state::AppState,
    config::HlsConfig,
    features::{
        audio::hls::HlsOriginState,
        calendar_day::{
            self, CalendarDayAdminBroadcastEvent, CalendarDayStore, CalendarNativeAudioReadyTarget,
            contracts::{self, CalendarSourceKind, CalendarSourceSnapshot},
            native::{
                collectors::{
                    CalendarCollectorAudit, CalendarCollectorOutcome,
                    CalendarCollectorRefusalReason, CalendarCollectorSource,
                },
                work::{
                    CalendarWorkErrorCode, CalendarWorkKind, CalendarWorkLeaseMutation,
                    CalendarWorkStore, ClaimedCalendarWork,
                },
            },
        },
        ingestion::{
            pipeline,
            runtime::{
                FinancialJuiceBrowserRuntimeState, FinancialJuiceRuntimeState, XRuntimeState,
            },
            x_settings::XSettingsStore,
        },
        live::dto::EdgeLiveMessage,
        news::{NewsStore, domain::NewsEvent},
        test_support::app_state,
        translation::TranslationRuntimeState,
    },
};

use super::{
    CalendarNativeHandle, CalendarNativeNotifyOutcome, CalendarNativeWake, summary_input_item,
    work_target_id,
};

const LOCAL_DAY: &str = "2026-06-15";
const FIXED_NOW_UNIX_SECONDS: i64 = 1_900_000_000;
const LEASE_DURATION: Duration = Duration::from_secs(120);

/// Valide le crash apres le commit calendrier, la reprise SQLite et l'absence
/// d'interference avec le flux live. Les doublures ne manipulent que les
/// contrats Rust: le texte source reste en memoire et SQLite ne recoit que son
/// audit redige (statut, hash et taille) ainsi que les metadonnees de
/// provenance explicitement autorisees pour chaque item.
#[tokio::test]
async fn native_calendar_recovery_e2e_is_idempotent_and_keeps_live_news_unblocked() {
    let paths = RecoveryPaths::new();
    let fixed_now = fixed_now();

    // Premier processus: l'API admin reelle admet une intention durable sans
    // attendre le worker. Le reveil est deja sature pour couvrir `try_send`.
    let RecoveryApp {
        state: state_one,
        native_wake_rx: _native_wake_rx_one,
        ingest_rx: _ingest_rx_one,
        calendar_audio_rx: _calendar_audio_rx_one,
    } = recovery_app(&paths).await;
    assert_eq!(
        state_one.calendar_day_native.try_notify(),
        CalendarNativeNotifyOutcome::Enqueued
    );
    let mut admin_ws_events = state_one.calendar_day_admin_tx.subscribe();
    let app_one = recovery_router(state_one.clone());
    let (admission_status, admission_body) = post_json(
        app_one,
        "/api/admin/calendar-day/run",
        json!({
            "localDay": LOCAL_DAY,
            "force": true,
            "reason": "native_recovery_e2e"
        }),
    )
    .await;
    assert_eq!(admission_status, StatusCode::OK);
    let run_id = admission_body["activePendingRunId"]
        .as_str()
        .expect("admin admission must expose the pending native run")
        .to_owned();
    assert!(
        receive_admin_run_started(&mut admin_ws_events, &run_id).await,
        "the broadcast consumed by the admin WebSocket must receive the native admission"
    );
    assert_eq!(
        state_one.calendar_day_native.try_notify(),
        CalendarNativeNotifyOutcome::Full,
        "a saturated calendar wake queue must never make the handler wait"
    );

    let first_store = CalendarWorkStore::new(state_one.news.clone_pool_for_feature_store());
    let ingestion_work = claim_next(&first_store, "native-e2e-first", fixed_now)
        .await
        .expect("the admin request must admit ingestion work");
    assert_eq!(ingestion_work.work.kind, CalendarWorkKind::Ingestion);
    assert_eq!(ingestion_work.work.idempotency_key, run_id);

    // Les deux fake fronts sont explicites: le collecteur fournit un snapshot
    // en memoire et OpenRouter ne retourne qu'un JSON de contrat previsible.
    let collectors = FakeCalendarCollectors::covered(LOCAL_DAY);
    let snapshots = collectors.collect();
    let fake_openrouter = FakeOpenRouter;
    let mut ingestion = fake_openrouter
        .ingestion(LOCAL_DAY, &snapshots)
        .expect("fake OpenRouter ingestion must satisfy the typed contract");
    assert!(feeds::validate_output(&ingestion, &[&collectors.document]));
    let source_binding = contracts::CalendarCoveredSourceBinding::for_local_day(
        LOCAL_DAY,
        CalendarSourceKind::Feed(CalendarFeedSource::ForexFactoryExport),
        Url::parse(CalendarFeedSource::ForexFactoryExport.endpoint())
            .expect("canonical export URL must parse"),
    )
    .expect("covered source binding must be valid");
    contracts::project_native_ingestion_provenance(
        &mut ingestion,
        std::slice::from_ref(&source_binding),
    )
    .expect("fake OpenRouter provenance must be projected before persistence");
    let ingestion_commit = state_one
        .calendar_day
        .commit_native_ingestion(
            &first_store,
            &ingestion_work.lease,
            &run_id,
            &ingestion,
            collectors.audits(),
            &state_one.config.calendar_day.alert_offsets_secs,
            fixed_now,
        )
        .await
        .expect("atomic calendar commit must include downstream work intents");
    assert_eq!(ingestion_commit.items.len(), 1);
    assert_native_commit_shape(&state_one, &first_store).await;

    // Point d'interruption demande: le commit calendrier est fait, mais aucun
    // handoff Piper/HLS n'a eu lieu. Le signal d'arret empeche le watchdog
    // historique de muter la base pendant le redemarrage du test.
    state_one
        .shutdown_test_keepalive
        .send(true)
        .expect("first test process must observe shutdown");
    drop(first_store);
    drop(state_one);

    // Deuxieme processus sur le meme fichier SQLite et le meme repertoire HLS.
    // Il relit d'abord SQLite: les cles d'idempotence deja commitees gardent
    // exactement un item, une alerte et un travail, sans second handoff.
    let RecoveryApp {
        state: state_two,
        native_wake_rx: _native_wake_rx_two,
        ingest_rx: ingest_rx_two,
        calendar_audio_rx: _calendar_audio_rx_two,
    } = recovery_app(&paths).await;
    let second_store = CalendarWorkStore::new(state_two.news.clone_pool_for_feature_store());
    assert_eq!(
        second_store
            .recover_expired_leases(fixed_now)
            .await
            .expect("startup lease scan should succeed"),
        0
    );
    assert_native_commit_shape(&state_two, &second_store).await;

    // Le pipeline live tourne pendant que le calendrier est volontairement
    // lent/sature. Il a son propre channel et son broadcast ne depend ni de
    // Piper, ni de HLS, ni du reveil durable calendrier.
    assert_eq!(
        state_two.calendar_day_native.try_notify(),
        CalendarNativeNotifyOutcome::Enqueued
    );
    assert_eq!(
        state_two.calendar_day_native.try_notify(),
        CalendarNativeNotifyOutcome::Full
    );
    let mut live_news = state_two.broadcast_tx.subscribe();
    let pipeline_shutdown = state_two.shutdown_rx.clone();
    // `oneshot` transporte un unique signal; `spawn` transfere le Sender au
    // worker pendant que ce scenario attend son Receiver avant toute injection.
    let (pipeline_ready_tx, pipeline_ready_rx) = oneshot::channel();
    let live_pipeline = tokio::spawn(pipeline::run_with_startup_signal(
        state_two.clone(),
        ingest_rx_two,
        pipeline_shutdown,
        pipeline_ready_tx,
    ));
    // Ce delai borne uniquement le demarrage. Les deux `expect` distinguent
    // un worker non planifie d'un worker arrete avant d'avoir envoye son signal.
    timeout(Duration::from_secs(5), pipeline_ready_rx)
        .await
        .expect("live pipeline must become ready before measuring its hot path")
        .expect("live pipeline startup signal must remain connected");
    // Le budget d'une seconde couvre admission, persistance et broadcast apres
    // readiness, alors que le reveil calendrier reste sature et non consomme.
    let live_event = timeout(Duration::from_secs(1), async {
        state_two
            .ingest_tx
            .send(NewsEvent::new_headline(
                "native_recovery_live",
                "Live news remains independent from calendar recovery",
                "https://example.test/native-recovery-live",
                None,
                None,
            ))
            .await
            .expect("live news channel must accept an event while calendar is saturated");
        live_news
            .recv()
            .await
            .expect("live news broadcast channel must remain open")
    })
    .await
    .expect("calendar saturation must not delay the live news broadcast");
    assert_eq!(live_event.source, "native_recovery_live");

    // Une reservation de stream suivie d'un crash reste une seconde barriere:
    // le prochain process doit reutiliser exactement la meme valeur au lieu de
    // creer un second clip HLS pour le meme travail.
    let reserved_before_crash = claim_next(&second_store, "native-e2e-crash", fixed_now)
        .await
        .expect("an item audio work must be ready after ingestion");
    assert_eq!(reserved_before_crash.work.kind, CalendarWorkKind::ItemAudio);
    let expected_stream_id = format!(
        "calendar-native-{}-{}",
        reserved_before_crash.work.kind.as_str(),
        reserved_before_crash.work.id
    );
    let reserved = second_store
        .reserve_stream_id(&reserved_before_crash.lease, &expected_stream_id, fixed_now)
        .await
        .expect("stream reservation must be fenced");
    assert_eq!(
        reserved_stream_id(reserved).as_deref(),
        Some(expected_stream_id.as_str())
    );
    state_two
        .shutdown_test_keepalive
        .send(true)
        .expect("second test process must observe shutdown");
    timeout(Duration::from_secs(1), live_pipeline)
        .await
        .expect("live pipeline must stop cleanly")
        .expect("live pipeline must not panic");
    drop(second_store);
    drop(state_two);

    // Dernier processus: les leases expirees sont reprises avant tout nouveau
    // claim; la doublure Piper/HLS ecrit des manifests deterministes dans le
    // meme repertoire et acquitte seulement apres la persistence canonique.
    let RecoveryApp {
        state: state_three,
        native_wake_rx: _native_wake_rx_three,
        ingest_rx: _ingest_rx_three,
        calendar_audio_rx: _calendar_audio_rx_three,
    } = recovery_app(&paths).await;
    let third_store = CalendarWorkStore::new(state_three.news.clone_pool_for_feature_store());
    let after_lease = fixed_now + TimeDuration::seconds(LEASE_DURATION.as_secs() as i64 + 1);
    assert_eq!(
        third_store
            .recover_expired_leases(after_lease)
            .await
            .expect("expired audio lease must be returned before new admission"),
        1
    );
    let piper_hls = DeterministicPiperHls::new(paths.hls_dir.clone());
    let completed_streams = piper_hls
        .drain(&state_three, &third_store, after_lease, &fake_openrouter)
        .await;
    assert_eq!(
        completed_streams.len(),
        3,
        "one item, alert and summary clip"
    );
    assert!(completed_streams.contains(&expected_stream_id));
    assert!(
        piper_hls.manifest_path(&expected_stream_id).is_file(),
        "the recovered item stream must reuse its pre-crash manifest identity"
    );
    assert!(
        paths
            .hls_dir
            .join("calendar-summary")
            .join(LOCAL_DAY)
            .join("current.m3u8")
            .is_file(),
        "the summary current manifest must survive in the shared HLS directory"
    );

    let mut published_alerts = state_three.live_metadata_tx.subscribe();
    calendar_day::publish_due_calendar_alerts(&state_three)
        .await
        .expect("ready calendar alert must publish through its stable key");
    let published_alert = timeout(Duration::from_secs(1), published_alerts.recv())
        .await
        .expect("calendar alert publication must remain non-blocking")
        .expect("calendar alert metadata broadcast must remain open");
    let EdgeLiveMessage::CalendarAlertReady { payload } = published_alert else {
        panic!("expected a calendar alert publication");
    };
    assert!(payload.event_id.starts_with("calendar_alert_ready:"));
    assert_eq!(payload.priority, "high");

    assert_terminal_persistence(&state_three, &third_store).await;
    assert_routes_and_hls(&state_three).await;
    assert_admin_calendar_websocket(&state_three).await;

    state_three
        .shutdown_test_keepalive
        .send(true)
        .expect("final test process must observe shutdown");
    paths.cleanup().await;
}

#[tokio::test]
async fn historical_html_refusal_does_not_ban_new_official_exports() {
    let state = app_state().await;
    let calendar_day = "2026-06-15";
    let calendar_day::CalendarDayIngestTriggerOutcome::Started(run) =
        super::admit_ingestion(&state, calendar_day, "terminal_refusal_test", true, 0)
            .await
            .expect("first admission must create native work")
    else {
        panic!("first admission must not be skipped")
    };
    let store = CalendarWorkStore::new(state.news.clone_pool_for_feature_store());
    let now = OffsetDateTime::now_utc();
    let claimed = store
        .claim("terminal-refusal-test", now, Duration::from_secs(30), 1)
        .await
        .expect("calendar work must be claimable")
        .pop()
        .expect("the admitted work must exist");
    let refusal = CalendarCollectorAudit {
        source: CalendarCollectorSource::Feed(
            crate::features::calendar_day::native::feeds::CalendarFeedSource::BeaJson,
        ),
        outcome: CalendarCollectorOutcome::Refused(CalendarCollectorRefusalReason::Forbidden),
        http_status: Some(403),
        raw_sha256: None,
        raw_length_bytes: None,
    };
    let code = CalendarWorkErrorCode::try_new("calendar_native_sources_refused")
        .expect("terminal source refusal must use a closed error code");
    state
        .calendar_day
        .commit_native_terminal_error(
            &store,
            &claimed.lease,
            &run.run_id,
            calendar_day,
            "ingestion",
            &[refusal],
            &code,
            now,
        )
        .await
        .expect("terminal refusal must commit atomically");
    let refusal_artifact = state
        .calendar_day
        .latest_native_source_audit_artifact(calendar_day)
        .await
        .expect("terminal refusal audit must be readable")
        .expect("terminal refusal must expose a redacted source audit artifact");
    let refusal_audit: Value = serde_json::from_str(&refusal_artifact.content_json)
        .expect("terminal refusal audit artifact must be JSON");
    assert_eq!(refusal_audit["sources"][0]["source"], "bea_json");
    assert_eq!(refusal_audit["sources"][0]["outcome"], "refused");
    assert_eq!(refusal_audit["sources"][0]["detail"], "forbidden");
    assert!(
        refusal_audit["sources"][0]["rawSha256"].is_null(),
        "a refusal audit must never expose a source body or a synthetic hash"
    );
    let quarantines = state
        .calendar_day
        .native_collector_quarantines(calendar_day)
        .await
        .expect("terminal refusal must quarantine the source for the day");
    assert!(matches!(
        quarantines.as_slice(),
        [
            crate::features::calendar_day::native::collectors::CalendarCollectorQuarantine {
                source: CalendarCollectorSource::Feed(
                    crate::features::calendar_day::native::feeds::CalendarFeedSource::BeaJson
                ),
                reason: CalendarCollectorRefusalReason::Forbidden,
                http_status: Some(403),
            }
        ]
    ));

    assert!(matches!(
        super::admit_ingestion(&state, calendar_day, "forced_retry", true, 0)
            .await
            .expect("historical HTML refusal must not block newly authorised exports"),
        calendar_day::CalendarDayIngestTriggerOutcome::Started(_)
    ));
}

/// Frontiere locale qui imite un collecteur couvert sans egress HTTP. Le texte
/// ne quitte jamais le snapshot; l'audit qui rejoint SQLite ne porte que sa
/// taille et son SHA-256.
struct FakeCalendarCollectors {
    snapshot: CalendarSourceSnapshot,
    audits: Vec<CalendarCollectorAudit>,
    document: CalendarFeedDocument,
}

#[tokio::test]
async fn ingestion_rejections_are_atomic_fenced_and_visible_without_downstream_work() {
    use super::super::{collectors::CalendarCollectorSet, openrouter::*};
    use std::sync::atomic::{AtomicUsize, Ordering};
    for (required_present, embedded_failure) in [(false, false), (true, false), (true, true)] {
        let state = app_state().await;
        let calls = Arc::new(AtomicUsize::new(0));
        let captured = calls.clone();
        let export = if required_present {
            r#"[{"title":"Test","country":"USD","date":"2026-06-15T12:00:00Z","impact":"Low"}]"#
        } else {
            r#"[{"title":"Test","country":"USD","date":"2026-06-16T12:00:00Z","impact":"Low"}]"#
        };
        let mock=Router::new().route("/feed",get(move ||async move {export}))
            .route("/optional",get(||async {r#"{"Extra":{"release_dates":["2026-06-15T14:00:00Z"]}}"#}))
            .route("/chat",post(move || {let calls=captured.clone();async move {
                calls.fetch_add(1,Ordering::SeqCst);
                if embedded_failure {
                    return axum::Json(json!({"error":{"message":"private-provider-error"}}));
                }
                axum::Json(json!({"choices":[{"finish_reason":"stop","message":{"content": "{\"items\":[{\"translatedTitle\":\"private-untranslated-content\"}]}"}}]}))
            }}));
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move { axum::serve(listener, mock).await.unwrap() });
        let store = CalendarWorkStore::new(state.news.clone_pool_for_feature_store());
        let runtime = super::NativeCalendarRuntime {
            collectors: CalendarCollectorSet::loopback_for_tests([
                (
                    CalendarCollectorSource::Feed(crate::features::calendar_day::native::feeds::CalendarFeedSource::ForexFactoryExport),
                    Url::parse(&format!("http://{address}/feed")).unwrap(),
                ),
                (
                    CalendarCollectorSource::Feed(crate::features::calendar_day::native::feeds::CalendarFeedSource::BeaJson),
                    Url::parse(&format!("http://{address}/optional")).unwrap(),
                ),
            ])
            .unwrap(),
            openrouter: OpenRouterCalendarClient::new(
                reqwest::Client::new(),
                OpenRouterCalendarClientConfig::new(
                    Url::parse(&format!("http://{address}/chat")).unwrap(),
                    OpenRouterCalendarModel::new("test/model").unwrap(),
                    OpenRouterCalendarLimits::new(
                        Duration::from_secs(2),
                        400_000,
                        1_000_000,
                        0,
                        Duration::ZERO,
                        1,
                    )
                    .unwrap(),
                )
                .unwrap(),
                OpenRouterApiKey::new("test-only").unwrap(),
            ),
            work: store.clone(),
        };
        let mut events = state.calendar_day_admin_tx.subscribe();
        let (status, _) = post_json(
            recovery_router(state.clone()),
            "/api/admin/calendar-day/run",
            json!({"localDay":LOCAL_DAY,"force":true}),
        )
        .await;
        assert!(status.is_success());
        let claimed = store
            .claim(
                "diagnostic-test",
                OffsetDateTime::now_utc(),
                Duration::from_secs(30),
                1,
            )
            .await
            .unwrap()
            .pop()
            .unwrap();
        assert!(matches!(
            super::execute_ingestion(&state, &runtime, &claimed).await,
            super::NativeWorkExecution::Committed
        ));
        assert_eq!(calls.load(Ordering::SeqCst), usize::from(required_present));
        let artifact=sqlx::query_as::<_,calendar_day::CalendarDayRunArtifact>("SELECT * FROM calendar_day_run_artifacts WHERE label='calendar_native_ingestion_diagnostic'")
            .fetch_one(&state.calendar_day.pool).await.unwrap();
        let diagnostic: Value = serde_json::from_str(&artifact.content_json).unwrap();
        assert_eq!(
            diagnostic["reason"],
            if embedded_failure {
                "open_router_embedded_error"
            } else if required_present {
                "non_french_field"
            } else {
                "required_source_missing_day"
            }
        );
        if required_present && !embedded_failure {
            assert_eq!(diagnostic["itemIndex"], 1);
            assert_eq!(diagnostic["field"], "translatedTitle");
        }
        assert!(!artifact.content_json.contains("private"));
        let mut broadcast_seen = false;
        while let Ok(event) = events.try_recv() {
            if let CalendarDayAdminBroadcastEvent::Artifact(a) = event {
                broadcast_seen |= a.id == artifact.id;
            }
        }
        assert!(
            broadcast_seen,
            "post-commit artifact event must be published"
        );
        let (_, snapshot) = get_json(
            recovery_router(state.clone()),
            &format!("/api/admin/calendar-day?local_day={LOCAL_DAY}"),
        )
        .await;
        assert!(
            snapshot["artifacts"]
                .as_array()
                .unwrap()
                .iter()
                .any(|a| a["id"] == artifact.id)
        );
        assert_admin_calendar_websocket(&state).await;
        let code = CalendarWorkErrorCode::try_new("calendar_native_ingestion_rejected").unwrap();
        let detail = super::super::diagnostics::CalendarIngestionDiagnostic::new(
            super::super::diagnostics::CalendarDiagnosticReason::InvalidJson,
        );
        assert!(
            state
                .calendar_day
                .commit_native_terminal_error_with_diagnostic(
                    &store,
                    &claimed.lease,
                    &claimed.work.idempotency_key,
                    LOCAL_DAY,
                    "ingestion",
                    &[],
                    &code,
                    OffsetDateTime::now_utc(),
                    Some(&detail)
                )
                .await
                .is_err()
        );
        let count:i64=sqlx::query_scalar("SELECT count(*) FROM calendar_day_run_artifacts WHERE label='calendar_native_ingestion_diagnostic'").fetch_one(&state.calendar_day.pool).await.unwrap();
        assert_eq!(count, 1);
        for table in [
            "calendar_day_items",
            "calendar_day_summaries",
            "calendar_day_alerts",
            "calendar_day_item_audio",
        ] {
            let count: i64 = sqlx::query_scalar(&format!("SELECT count(*) FROM {table}"))
                .fetch_one(&state.calendar_day.pool)
                .await
                .unwrap();
            assert_eq!(count, 0);
        }
        let downstream: i64 =
            sqlx::query_scalar("SELECT count(*) FROM calendar_native_work WHERE kind<>'ingestion'")
                .fetch_one(&state.calendar_day.pool)
                .await
                .unwrap();
        assert_eq!(downstream, 0);
        server.abort();
    }
}

fn fake_feed_document(local_day: &str) -> CalendarFeedDocument {
    feeds::parse(CalendarFeedSource::ForexFactoryExport, &json!([{
        "title": "ECB Rate Decision", "country": "EUR", "date": format!("{local_day}T12:30:00Z"),
        "impact": "High", "forecast": "", "previous": ""
    }]).to_string(), local_day.parse().expect("fixture day")).expect("real export parser")
}

impl FakeCalendarCollectors {
    fn covered(local_day: &str) -> Self {
        let document = fake_feed_document(local_day);
        let source_text = serde_json::to_string(&document).expect("typed source");
        let snapshot = CalendarSourceSnapshot::from_response_text(
            local_day,
            CalendarSourceKind::Feed(CalendarFeedSource::ForexFactoryExport),
            CalendarFeedSource::ForexFactoryExport.endpoint(),
            200,
            &source_text,
        )
        .expect("fake covered source must satisfy the snapshot contract");
        let hash = format!("{:x}", Sha256::digest(source_text.as_bytes()));
        Self {
            snapshot,
            document,
            audits: vec![CalendarCollectorAudit {
                source: CalendarCollectorSource::Feed(crate::features::calendar_day::native::feeds::CalendarFeedSource::ForexFactoryExport),
                outcome: CalendarCollectorOutcome::Covered,
                http_status: Some(200),
                raw_sha256: Some(hash),
                raw_length_bytes: Some(source_text.len()),
            }],
        }
    }

    fn collect(&self) -> Vec<CalendarSourceSnapshot> {
        vec![self.snapshot.clone()]
    }

    fn audits(&self) -> &[CalendarCollectorAudit] {
        &self.audits
    }
}

/// Doublure OpenRouter: elle fournit seulement des sorties JSON de contrat,
/// sans client HTTP, cle API ou effet de bord externe.
struct FakeOpenRouter;

impl FakeOpenRouter {
    fn ingestion(
        &self,
        local_day: &str,
        snapshots: &[CalendarSourceSnapshot],
    ) -> Result<contracts::CalendarIngestionDraft, contracts::CalendarOrchestrationError> {
        contracts::canonicalize_ingestion_output(
            local_day,
            snapshots,
            &json!({
                "items": [{
                    "eventTime": format!("{local_day}T12:30:00Z"),
                    "sourceTitle": "ECB Rate Decision",
                    "translatedTitle": "Decision de taux de la BCE",
                    "ttsText": "Evenement calendrier. Decision de taux de la BCE.",
                    "country": "EU",
                    "currency": "EUR",
                    "sourceImportance": "high",
                    "aiImportance": "high",
                    "finalImportance": "high",
                    "category": "rates",
                    "reasoning": "Decision majeure pour les marches europeens.",
                    "confidence": 0.92,
                    "sources": [{
                        "source": "forex_factory_export",
                        "externalId": fake_feed_document(local_day).records[0].external_id,
                        "rawTitle": "ECB Rate Decision",
                        "raw": {"transient": "must_not_persist"}
                    }]
                }]
            })
            .to_string(),
        )
    }

    fn summary(
        &self,
        local_day: &str,
        items: &[contracts::CalendarSummaryInputItem],
    ) -> Result<contracts::CalendarSummaryDraft, contracts::CalendarOrchestrationError> {
        contracts::canonicalize_summary_output(
            local_day,
            items,
            &json!({
                "title": "Calendrier economique du jour",
                "summaryText": "La decision de taux de la BCE est l evenement high du jour.",
                "narrationText": "Resume calendrier. La decision de taux de la BCE est attendue aujourd hui."
            })
            .to_string(),
        )
    }
}

/// Doublure Piper/HLS deterministe. Elle conserve les memes regles durables
/// que le worker reel: reservation stable, ecriture du manifest, mise a jour
/// `calendar_day_*`, puis acknowledgement fence de la lease.
struct DeterministicPiperHls {
    hls_dir: PathBuf,
}

impl DeterministicPiperHls {
    fn new(hls_dir: PathBuf) -> Self {
        Self { hls_dir }
    }

    async fn drain(
        &self,
        state: &AppState,
        store: &CalendarWorkStore,
        now: OffsetDateTime,
        openrouter: &FakeOpenRouter,
    ) -> Vec<String> {
        let mut streams = Vec::new();
        loop {
            let Some(claimed) = claim_next(store, "native-e2e-recovery", now).await else {
                break;
            };
            match claimed.work.kind {
                CalendarWorkKind::Summary => {
                    let inputs = state
                        .calendar_day
                        .list_items(&claimed.work.local_day)
                        .await
                        .expect("summary must reload canonical items")
                        .iter()
                        .map(summary_input_item)
                        .collect::<Result<Vec<_>, _>>()
                        .expect("summary inputs must remain typed");
                    let draft = openrouter
                        .summary(&claimed.work.local_day, &inputs)
                        .expect("fake OpenRouter summary must satisfy its contract");
                    state
                        .calendar_day
                        .commit_native_summary(
                            store,
                            &claimed.lease,
                            &claimed.work.idempotency_key,
                            &draft,
                            &state.config.calendar_day.native.model,
                            now,
                        )
                        .await
                        .expect("atomic summary persistence must admit its audio work");
                }
                CalendarWorkKind::ItemAudio
                | CalendarWorkKind::AlertAudio
                | CalendarWorkKind::SummaryAudio => {
                    let stream_id = self.complete_audio_work(state, store, &claimed, now).await;
                    streams.push(stream_id);
                }
                CalendarWorkKind::Ingestion => {
                    panic!("ingestion was already committed before the recovery restart")
                }
            }
        }
        streams
    }

    async fn complete_audio_work(
        &self,
        state: &AppState,
        store: &CalendarWorkStore,
        claimed: &ClaimedCalendarWork,
        now: OffsetDateTime,
    ) -> String {
        let candidate = format!(
            "calendar-native-{}-{}",
            claimed.work.kind.as_str(),
            claimed.work.id
        );
        let reserved = store
            .reserve_stream_id(&claimed.lease, candidate, now)
            .await
            .expect("audio stream reservation must succeed");
        let stream_id = reserved_stream_id(reserved).expect("a stream id must be reserved");
        self.write_clip_manifest(&stream_id)
            .await
            .expect("deterministic Piper/HLS clip manifest must be written");
        let playlist_url = format!("/audio/clips/{stream_id}/playlist.m3u8");
        match claimed.work.kind {
            CalendarWorkKind::ItemAudio => {
                let item_id = work_target_id(&claimed.work.idempotency_key)
                    .expect("item audio work key must carry an id");
                state
                    .calendar_day
                    .commit_native_audio_ready(
                        store,
                        &claimed.lease,
                        CalendarNativeAudioReadyTarget::Item {
                            item_id,
                            stream_id: stream_id.clone(),
                            playlist_url,
                        },
                        now,
                    )
                    .await
                    .expect("item audio persistence and durable completion must succeed");
            }
            CalendarWorkKind::AlertAudio => {
                let alert_id = work_target_id(&claimed.work.idempotency_key)
                    .expect("alert audio work key must carry an id");
                state
                    .calendar_day
                    .commit_native_audio_ready(
                        store,
                        &claimed.lease,
                        CalendarNativeAudioReadyTarget::Alert {
                            alert_id,
                            stream_id: stream_id.clone(),
                            playlist_url,
                        },
                        now,
                    )
                    .await
                    .expect("alert audio persistence and durable completion must succeed");
            }
            CalendarWorkKind::SummaryAudio => {
                let summary_id = work_target_id(&claimed.work.idempotency_key)
                    .expect("summary audio work key must carry an id");
                let manifest = self
                    .write_summary_manifest(&claimed.work.local_day, &stream_id)
                    .await
                    .expect("deterministic summary HLS manifest must be written");
                state
                    .calendar_day
                    .commit_native_audio_ready(
                        store,
                        &claimed.lease,
                        CalendarNativeAudioReadyTarget::Summary {
                            summary_id,
                            stream_id: stream_id.clone(),
                            playlist_url: manifest.current_playlist_url.clone(),
                            playlist_path: manifest.playlist_path,
                            immutable_playlist_url: manifest.immutable_playlist_url,
                            current_playlist_url: manifest.current_playlist_url,
                        },
                        now,
                    )
                    .await
                    .expect("summary audio persistence and durable completion must succeed");
            }
            CalendarWorkKind::Ingestion | CalendarWorkKind::Summary => {
                unreachable!("only audio work reaches deterministic Piper/HLS")
            }
        }
        stream_id
    }

    async fn write_clip_manifest(&self, stream_id: &str) -> Result<(), std::io::Error> {
        let dir = self.hls_dir.join("clips").join(stream_id);
        tokio::fs::create_dir_all(&dir).await?;
        tokio::fs::write(dir.join("segment-000.ts"), b"deterministic-piper-segment").await?;
        tokio::fs::write(
            dir.join("playlist.m3u8"),
            format!(
                "#EXTM3U\n#EXT-X-VERSION:3\n#EXTINF:1.000,\n/audio/clips/{stream_id}/segment-000.ts\n#EXT-X-ENDLIST\n"
            ),
        )
        .await
    }

    async fn write_summary_manifest(
        &self,
        local_day: &str,
        stream_id: &str,
    ) -> Result<DeterministicSummaryManifest, std::io::Error> {
        let dir = self
            .hls_dir
            .join("calendar-summary")
            .join(local_day)
            .join(stream_id);
        tokio::fs::create_dir_all(&dir).await?;
        let segment_name = "segment-000.ts";
        tokio::fs::write(dir.join(segment_name), b"deterministic-summary-segment").await?;
        let playlist = format!(
            "#EXTM3U\n#EXT-X-VERSION:3\n#EXT-X-PLAYLIST-TYPE:VOD\n#EXTINF:1.000,\n/audio/calendar-summary/{local_day}/{stream_id}/{segment_name}\n#EXT-X-ENDLIST\n"
        );
        tokio::fs::write(dir.join("playlist.m3u8"), &playlist).await?;
        let current_dir = self.hls_dir.join("calendar-summary").join(local_day);
        tokio::fs::write(current_dir.join("current.m3u8"), playlist).await?;
        Ok(DeterministicSummaryManifest {
            playlist_path: format!("calendar-summary/{local_day}/{stream_id}/playlist.m3u8"),
            immutable_playlist_url: format!(
                "/audio/calendar-summary/{local_day}/{stream_id}/playlist.m3u8"
            ),
            current_playlist_url: format!("/audio/calendar-summary/{local_day}/current.m3u8"),
        })
    }

    fn manifest_path(&self, stream_id: &str) -> PathBuf {
        self.hls_dir
            .join("clips")
            .join(stream_id)
            .join("playlist.m3u8")
    }
}

struct DeterministicSummaryManifest {
    playlist_path: String,
    immutable_playlist_url: String,
    current_playlist_url: String,
}

struct RecoveryApp {
    state: AppState,
    native_wake_rx: mpsc::Receiver<CalendarNativeWake>,
    ingest_rx: mpsc::Receiver<NewsEvent>,
    calendar_audio_rx: mpsc::Receiver<crate::features::audio::domain::CalendarDayAudioJob>,
}

async fn recovery_app(paths: &RecoveryPaths) -> RecoveryApp {
    let base = app_state().await;
    let mut config = (*base.config).clone();
    config.storage.database_url = format!("sqlite://{}", paths.database.display());
    config.storage.max_connections = 1;
    config.calendar_day.enabled = true;
    config.calendar_day.alert_offsets_secs = vec![60];
    config.calendar_day.native.queue_capacity = 1;
    config.hls = HlsConfig {
        enabled: true,
        access_control_enabled: false,
        access_ticket_secret: None,
        access_ticket_ttl_secs: 3600,
        segments_dir: paths.hls_dir.display().to_string(),
        ffmpeg_command: "ffmpeg".to_owned(),
        replay_segment_target_duration_secs: 4,
        clip_segment_target_duration_secs: 2,
    };
    let config = Arc::new(config);
    let news = NewsStore::connect(&config.storage)
        .await
        .expect("file SQLite database must open");
    news.migrate()
        .await
        .expect("file SQLite news schema must migrate");
    let calendar_day = CalendarDayStore::from_news_store(&news);
    calendar_day
        .migrate()
        .await
        .expect("calendar native schema must migrate");
    let x_settings_store = XSettingsStore::from_news_store(&news);
    x_settings_store
        .migrate()
        .await
        .expect("x settings schema must migrate");

    let (ingest_tx, ingest_rx) = mpsc::channel(config.runtime.ingest_buffer);
    let (shutdown_test_keepalive, shutdown_rx) = watch::channel(false);
    let (broadcast_tx, _) = broadcast::channel(config.runtime.broadcast_buffer);
    let (translated_broadcast_tx, _) = broadcast::channel(config.runtime.broadcast_buffer);
    let (live_metadata_tx, _) = broadcast::channel(config.runtime.broadcast_buffer);
    let (test_trace_tx, _) = broadcast::channel(config.runtime.broadcast_buffer);
    let (calendar_day_admin_tx, _) = broadcast::channel(config.runtime.broadcast_buffer);
    let (calendar_day_audio_tx, calendar_audio_rx) = mpsc::channel(config.runtime.ingest_buffer);
    let (calendar_day_native, native_wake_rx) =
        CalendarNativeHandle::channel(true, config.calendar_day.native.queue_capacity);

    RecoveryApp {
        state: AppState {
            calendar_logs: crate::features::calendar_logs::CalendarLogs::disabled(),
            config: config.clone(),
            news,
            calendar_day,
            calendar_day_native,
            tts_runtime: base.tts_runtime.clone(),
            hls_origin: Arc::new(HlsOriginState::from_config(&config.hls)),
            translation_runtime: Arc::new(TranslationRuntimeState::new(&config.translation)),
            financial_juice_runtime: Arc::new(FinancialJuiceRuntimeState::new(
                config.sources.financial_juice.enabled,
            )),
            financial_juice_browser_runtime: Arc::new(FinancialJuiceBrowserRuntimeState::new(
                config.sources.financial_juice_browser.source.enabled,
            )),
            x_runtime: Arc::new(XRuntimeState::new(&config.sources.x)),
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
        },
        native_wake_rx,
        ingest_rx,
        calendar_audio_rx,
    }
}

async fn assert_native_commit_shape(state: &AppState, store: &CalendarWorkStore) {
    assert_eq!(
        state
            .calendar_day
            .list_items(LOCAL_DAY)
            .await
            .expect("items must be readable")
            .len(),
        1
    );
    let source_audits: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM calendar_day_source_audits WHERE local_day = ?1 AND raw_sha256 IS NOT NULL AND raw_length_bytes > 0",
    )
    .bind(LOCAL_DAY)
    .fetch_one(&state.calendar_day.pool)
    .await
    .expect("redacted source audit count must be readable");
    assert_eq!(source_audits, 1);
    let persisted_source_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM calendar_day_item_sources WHERE calendar_item_id IN (SELECT id FROM calendar_day_items WHERE local_day = ?1)",
    )
    .bind(LOCAL_DAY)
    .fetch_one(&state.calendar_day.pool)
    .await
    .expect("native item source rows must be readable");
    assert_eq!(
        persisted_source_rows, 1,
        "the approved compact source metadata must remain attached to the canonical item"
    );
    let raw_source_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM calendar_day_item_sources WHERE calendar_item_id IN (SELECT id FROM calendar_day_items WHERE local_day = ?1) AND raw_json IS NOT NULL",
    )
    .bind(LOCAL_DAY)
    .fetch_one(&state.calendar_day.pool)
    .await
    .expect("native item raw source count must be readable");
    assert_eq!(
        raw_source_rows, 0,
        "native item provenance must never persist the model raw payload"
    );
    let sources_json: String =
        sqlx::query_scalar("SELECT sources_json FROM calendar_day_items WHERE local_day = ?1")
            .bind(LOCAL_DAY)
            .fetch_one(&state.calendar_day.pool)
            .await
            .expect("native item sources JSON must be readable");
    let sources: Value =
        serde_json::from_str(&sources_json).expect("native item sources JSON must remain valid");
    assert_eq!(sources[0]["source"], "forex_factory_export");
    assert_eq!(
        sources[0]["sourceUrl"],
        CalendarFeedSource::ForexFactoryExport.endpoint()
    );
    assert_eq!(sources[0]["rawTitle"], "ECB Rate Decision");
    assert!(
        sources[0].get("raw").is_none(),
        "sources_json must be metadata-only"
    );
    let source_audit_artifacts: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM calendar_day_run_artifacts WHERE local_day = ?1 AND label = 'calendar_native_source_audits'",
    )
    .bind(LOCAL_DAY)
    .fetch_one(&state.calendar_day.pool)
    .await
    .expect("native source audit artifact count must be readable");
    assert_eq!(source_audit_artifacts, 1);
    let pending = store
        .pending(None, 16)
        .await
        .expect("calendar downstream work must be readable");
    assert_eq!(
        pending
            .iter()
            .filter(|work| work.kind == CalendarWorkKind::ItemAudio)
            .count(),
        1
    );
    assert_eq!(
        pending
            .iter()
            .filter(|work| work.kind == CalendarWorkKind::AlertAudio)
            .count(),
        1
    );
    assert_eq!(
        pending
            .iter()
            .filter(|work| work.kind == CalendarWorkKind::Summary)
            .count(),
        1,
        "ingestion commit must durably admit native summary"
    );
}

async fn assert_terminal_persistence(state: &AppState, store: &CalendarWorkStore) {
    assert!(
        store
            .pending(None, 16)
            .await
            .expect("pending work must be readable")
            .is_empty(),
        "no calendar work may remain orphaned after recovery"
    );
    let non_terminal_work: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM calendar_native_work WHERE status <> 'completed'")
            .fetch_one(&state.calendar_day.pool)
            .await
            .expect("calendar work terminal state count must be readable");
    assert_eq!(
        non_terminal_work, 0,
        "no leased or dead-letter work is allowed"
    );
    let items: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM calendar_day_items WHERE local_day = ?1")
            .bind(LOCAL_DAY)
            .fetch_one(&state.calendar_day.pool)
            .await
            .expect("item count must be readable");
    let summaries: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM calendar_day_summaries WHERE local_day = ?1")
            .bind(LOCAL_DAY)
            .fetch_one(&state.calendar_day.pool)
            .await
            .expect("summary count must be readable");
    let alerts: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM calendar_day_alerts WHERE local_day = ?1 AND status = 'published'",
    )
    .bind(LOCAL_DAY)
    .fetch_one(&state.calendar_day.pool)
    .await
    .expect("alert count must be readable");
    let item_streams: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM calendar_day_item_audio WHERE stream_id IS NOT NULL",
    )
    .fetch_one(&state.calendar_day.pool)
    .await
    .expect("item audio stream count must be readable");
    assert_eq!((items, summaries, alerts, item_streams), (1, 1, 1, 1));
}

async fn assert_routes_and_hls(state: &AppState) {
    let app = recovery_router(state.clone());
    let (public_status, public_body) = get_json(
        app.clone(),
        &format!("/api/calendar-day?local_day={LOCAL_DAY}"),
    )
    .await;
    assert_eq!(public_status, StatusCode::OK);
    assert_eq!(public_body["itemCount"], 1);
    assert_eq!(public_body["summary"]["status"], "audio_ready");
    let (admin_status, admin_body) = get_json(
        app.clone(),
        &format!("/api/admin/calendar-day?local_day={LOCAL_DAY}"),
    )
    .await;
    assert_eq!(admin_status, StatusCode::OK);
    assert_eq!(admin_body["items"].as_array().map(Vec::len), Some(1));
    assert_eq!(admin_body["alerts"].as_array().map(Vec::len), Some(1));
    assert!(
        admin_body["artifacts"]
            .as_array()
            .is_some_and(|artifacts| artifacts
                .iter()
                .any(|artifact| artifact["label"] == "calendar_native_source_audits")),
        "the existing admin snapshot must expose the redacted native source audit"
    );
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/audio/calendar-summary/{LOCAL_DAY}/current.m3u8"))
                .body(Body::empty())
                .expect("HLS request must build"),
        )
        .await
        .expect("HLS route must complete");
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("HLS body must be readable");
    assert!(
        std::str::from_utf8(&body)
            .expect("HLS body must be UTF-8")
            .contains("#EXTM3U")
    );
}

/// Ouvre le vrai endpoint WebSocket admin avec l'auth locale des fixtures.
///
/// Ce controle garde la surface WebSocket dans la gate de recuperation sans
/// dependance a un JWT, un reseau externe ou un changement du contrat public.
async fn assert_admin_calendar_websocket(state: &AppState) {
    let listener = TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("calendar admin websocket listener must bind");
    let address = listener
        .local_addr()
        .expect("calendar admin websocket listener must expose its address");
    let app = Router::new()
        .route(
            "/ws/admin/calendar-day",
            get(calendar_day::ws_admin_calendar_day),
        )
        .with_state(state.clone());
    let server = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("calendar admin websocket server must run");
    });

    let (mut client, _) = connect_async(format!(
        "ws://{address}/ws/admin/calendar-day?local_day={LOCAL_DAY}"
    ))
    .await
    .expect("calendar admin websocket must connect");
    let first = timeout(Duration::from_secs(1), client.next())
        .await
        .expect("calendar admin websocket must send hello promptly")
        .expect("calendar admin websocket must remain open")
        .expect("calendar admin websocket hello frame must be readable");
    let ClientMessage::Text(first) = first else {
        panic!("calendar admin websocket must start with a text hello frame");
    };
    let hello: Value = serde_json::from_str(&first).expect("calendar WebSocket hello must be JSON");
    assert_eq!(hello["type"], "hello");

    let second = timeout(Duration::from_secs(1), client.next())
        .await
        .expect("calendar admin websocket must send a snapshot promptly")
        .expect("calendar admin websocket must remain open for its snapshot")
        .expect("calendar admin websocket snapshot frame must be readable");
    let ClientMessage::Text(second) = second else {
        panic!("calendar admin websocket snapshot must be a text frame");
    };
    let snapshot: Value =
        serde_json::from_str(&second).expect("calendar WebSocket snapshot must be JSON");
    assert_eq!(snapshot["type"], "snapshot");
    assert_eq!(snapshot["payload"]["state"]["localDay"], LOCAL_DAY);
    let diagnostic_exists:i64=sqlx::query_scalar("SELECT count(*) FROM calendar_day_run_artifacts WHERE local_day=?1 AND label='calendar_native_ingestion_diagnostic'")
        .bind(LOCAL_DAY).fetch_one(&state.calendar_day.pool).await.unwrap();
    if diagnostic_exists > 0 {
        assert!(
            snapshot["payload"]["artifacts"]
                .as_array()
                .unwrap()
                .iter()
                .any(|a| a["label"] == "calendar_native_ingestion_diagnostic")
        );
    }
    assert!(
        snapshot["payload"]["artifacts"]
            .as_array()
            .is_some_and(|artifacts| artifacts
                .iter()
                .any(|artifact| artifact["label"] == "calendar_native_source_audits")),
        "the existing admin WebSocket snapshot must expose the redacted source audit"
    );

    server.abort();
}

/// Les handlers sont ceux exposes par les routes publiques/admin. L'auth est
/// volontairement hors de ce test de reprise: la fixture locale sans JWT
/// n'obtient qu'un role moderateur alors que l'endpoint admin exige `admin`.
/// Les tests auth couvrent deja ce middleware separement.
fn recovery_router(state: AppState) -> Router {
    Router::new()
        .route(
            "/api/admin/calendar-day/run",
            post(calendar_day::run_admin_calendar_day),
        )
        .route(
            "/api/admin/calendar-day",
            get(calendar_day::get_admin_calendar_day),
        )
        .route("/api/calendar-day", get(calendar_day::get_calendar_day))
        .route(
            "/audio/calendar-summary/{local_day}/current.m3u8",
            get(crate::features::audio::hls::calendar_summary_current_playlist),
        )
        .with_state(state)
}

async fn claim_next(
    store: &CalendarWorkStore,
    worker_id: &str,
    now: OffsetDateTime,
) -> Option<ClaimedCalendarWork> {
    store
        .claim(worker_id, now, LEASE_DURATION, 1)
        .await
        .expect("calendar work claim must succeed")
        .into_iter()
        .next()
}

fn reserved_stream_id(mutation: CalendarWorkLeaseMutation) -> Option<String> {
    match mutation {
        CalendarWorkLeaseMutation::Applied(work) => work.reserved_stream_id,
        CalendarWorkLeaseMutation::FencedOut => None,
    }
}

async fn receive_admin_run_started(
    events: &mut broadcast::Receiver<CalendarDayAdminBroadcastEvent>,
    run_id: &str,
) -> bool {
    timeout(Duration::from_secs(1), async {
        loop {
            match events.recv().await {
                Ok(CalendarDayAdminBroadcastEvent::RunStarted(run)) if run.run_id == run_id => {
                    return true;
                }
                Ok(_) | Err(broadcast::error::RecvError::Lagged(_)) => {}
                Err(broadcast::error::RecvError::Closed) => return false,
            }
        }
    })
    .await
    .unwrap_or(false)
}

async fn post_json(app: Router, uri: &str, body: Value) -> (StatusCode, Value) {
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .expect("admin request must build"),
        )
        .await
        .expect("admin route must complete");
    response_json(response).await
}

async fn get_json(app: Router, uri: &str) -> (StatusCode, Value) {
    let response = app
        .oneshot(
            Request::builder()
                .uri(uri)
                .body(Body::empty())
                .expect("GET request must build"),
        )
        .await
        .expect("GET route must complete");
    response_json(response).await
}

async fn response_json(response: axum::response::Response) -> (StatusCode, Value) {
    let status = response.status();
    let body = to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("JSON response must be readable");
    let body = serde_json::from_slice(&body).expect("route response must be JSON");
    (status, body)
}

fn fixed_now() -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(FIXED_NOW_UNIX_SECONDS)
        .expect("fixed timestamp must be representable")
}

struct RecoveryPaths {
    root: PathBuf,
    database: PathBuf,
    hls_dir: PathBuf,
}

impl RecoveryPaths {
    fn new() -> Self {
        let unique = format!(
            "filfinance-calendar-native-recovery-{}-{}",
            std::process::id(),
            OffsetDateTime::now_utc().unix_timestamp_nanos()
        );
        let root = std::env::temp_dir().join(unique);
        std::fs::create_dir_all(&root).expect("test recovery root must be created");
        Self {
            database: root.join("calendar.sqlite"),
            hls_dir: root.join("hls"),
            root,
        }
    }

    async fn cleanup(&self) {
        let _ = tokio::fs::remove_dir_all(&self.root).await;
    }
}
