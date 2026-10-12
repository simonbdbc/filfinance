//! Sonde opt-in, sans AppState, migration, scheduler ni worker produit.
use super::super::diagnostics::{CalendarDiagnosticReason, CalendarIngestionDiagnostic};
use super::*;
use sha2::{Digest, Sha256};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

#[tokio::test]
#[ignore = "Requires explicit isolated server observation; two paid-capable AI calls, no product writes"]
async fn native_calendar_ingestion_diagnostic() -> Result<(), &'static str> {
    const DAY: &str = "2026-09-07";
    if std::env::var("FILFINANCE_CALENDAR_DIAGNOSTIC_ACK").as_deref() != Ok(DAY) {
        return Err("explicit diagnostic acknowledgement required");
    }
    // Create-new is durable outside the disposable container: never repeat a refused fetch.
    let _guard = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open("/diagnostic-state/2026-09-07.started")
        .map_err(|_| "diagnostic already attempted or guard unavailable")?;
    let config = crate::config::CalendarDayConfig::from_env()
        .map_err(|_| "invalid calendar config")?
        .native;
    if config.endpoint != "https://openrouter.ai/api/v1/chat/completions" {
        return Err("diagnostic endpoint must be official OpenRouter");
    }
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::new()
                .filename("/prod-data/news.db")
                .read_only(true)
                .create_if_missing(false),
        )
        .await
        .map_err(|_| "read-only database unavailable")?;
    let store = calendar_day::CalendarDayStore { pool };
    ensure_product_calendar_idle(&store).await?;
    let quarantines = store
        .native_collector_quarantines(DAY)
        .await
        .map_err(|_| "quarantine read failed")?;
    let http = Client::builder()
        .no_proxy()
        .cookie_store(false)
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|_| "diagnostic client failed")?;
    // Public metadata only; no key is sent to this endpoint, no fallback provider/model.
    let mut response = http
        .get(format!(
            "https://openrouter.ai/api/v1/models/{}/endpoints",
            config.model
        ))
        .send()
        .await
        .map_err(|_| "model metadata unavailable")?;
    if !response.status().is_success() {
        return Err("model metadata HTTP failure");
    }
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| "model metadata read failed")?
    {
        if body.len() + chunk.len() > 1_000_000 {
            return Err("model metadata too large");
        }
        body.extend_from_slice(&chunk);
    }
    let metadata: serde_json::Value =
        serde_json::from_slice(&body).map_err(|_| "model metadata invalid")?;
    let supported = metadata["data"]["endpoints"]
        .as_array()
        .is_some_and(|endpoints| {
            endpoints.iter().any(|e| {
                e["supported_parameters"].as_array().is_some_and(|p| {
                    p.iter().any(|v| v == "structured_outputs")
                        && p.iter().any(|v| v == "response_format")
                })
            })
        });
    if !supported {
        return Err("model has no endpoint declaring required response format; no fallback");
    }
    let collected = CalendarCollectorSet::production()
        .map_err(|_| "collectors unavailable")?
        .collect_with_quarantines(
            CalendarLocalDay::parse(DAY).map_err(|_| "invalid day")?,
            &quarantines,
        )
        .await;
    println!(
        "{}",
        serde_json::json!({"stage":"sources","audits":collected.audits(),"requiredSourceFailure":collected.required_source_failure()})
    );
    if collected
        .audits()
        .iter()
        .any(|a| matches!(a.outcome, CalendarCollectorOutcome::Refused(_)))
    {
        return Err(
            "source refusal: guard retained; operator quarantine review required before any rerun",
        );
    }
    if collected.required_source_failure().is_some() {
        return Err("required source unavailable; no AI call");
    }
    let super::super::collectors::CalendarCollectorSetResult::Covered { sources, .. } = collected
    else {
        return Err("no coverage");
    };
    let snapshots = sources
        .iter()
        .map(|s| snapshot_from_covered_source(DAY, s))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "snapshot contract")?;
    let bindings = sources
        .iter()
        .map(|s| source_binding_from_covered_source(DAY, s))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "binding contract")?;
    let documents = sources
        .iter()
        .filter_map(|s| s.document())
        .collect::<Vec<_>>();
    let prompt = build_ingestion_prompt(DAY, &snapshots).map_err(|_| "prompt contract")?;
    let limits = OpenRouterCalendarLimits::new(
        Duration::from_millis(config.request_timeout_ms),
        400_000,
        config.max_response_bytes,
        0,
        Duration::ZERO,
        1,
    )
    .map_err(|_| "diagnostic limits")?;
    let client_config = OpenRouterCalendarClientConfig::new(
        Url::parse(&config.endpoint).map_err(|_| "endpoint")?,
        OpenRouterCalendarModel::new(config.model).map_err(|_| "model")?,
        limits,
    )
    .map_err(|_| "client config")?;
    let key = OpenRouterApiKey::new(config.api_key.ok_or("missing key")?.expose().to_owned())
        .map_err(|_| "invalid key")?;
    let client = OpenRouterCalendarClient::new(http, client_config, key);
    for (profile, request) in [
        (
            "baseline",
            OpenRouterCalendarRequest::diagnostic_baseline(prompt.content()),
        ),
        (
            "canonical",
            OpenRouterCalendarRequest::ingestion(prompt.content()),
        ),
    ] {
        ensure_product_calendar_idle(&store).await?;
        let request = request.map_err(|_| "invalid diagnostic request")?;
        match client.complete_json(&request).await {
            Ok(completion) => {
                let validation =
                    contracts::canonicalize_ingestion_output(DAY, &snapshots, completion.content());
                let diagnostic = match validation {
                    Err(error) => Some(CalendarIngestionDiagnostic::contract(&error)),
                    Ok(mut draft) => {
                        if !super::super::feeds::validate_output(&draft, &documents) {
                            Some(CalendarIngestionDiagnostic::new(
                                CalendarDiagnosticReason::Provenance,
                            ))
                        } else {
                            contracts::project_native_ingestion_provenance(&mut draft, &bindings)
                                .err()
                                .map(|e| CalendarIngestionDiagnostic::contract(&e))
                        }
                    }
                };
                println!(
                    "{}",
                    serde_json::json!({"stage":"ingestion","profile":profile,"finishReason":"stop","bytes":completion.content().len(),"sha256":format!("{:x}",Sha256::digest(completion.content())),"diagnostic":diagnostic})
                );
            }
            Err(error) => println!(
                "{}",
                serde_json::json!({"stage":"openrouter","profile":profile,"error":error.to_string()})
            ),
        }
    }
    store.pool.close().await;
    Ok(())
}

async fn ensure_product_calendar_idle(
    store: &calendar_day::CalendarDayStore,
) -> Result<(), &'static str> {
    let active: i64 = sqlx::query_scalar("SELECT count(*) FROM calendar_native_work WHERE kind IN ('ingestion','summary') AND status IN ('pending','leased')")
        .fetch_one(&store.pool).await.map_err(|_| "calendar activity check failed")?;
    if active != 0 {
        return Err("product calendar active; isolated diagnostic suspended");
    }
    Ok(())
}
