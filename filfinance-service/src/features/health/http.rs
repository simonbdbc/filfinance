//! Handlers HTTP des healthchecks public et admin.
//!
//! Le endpoint public reste volontairement minimal pour les probes d'infrastructure.
//! Le endpoint admin agrege en revanche des snapshots runtime; il doit donc rester
//! lisible et ne jamais exposer de secret brut.

// `Json` construit une reponse axum serialisee, et `State` extrait l'`AppState` partage.
use axum::{Json, extract::State};
// `Value` permet d'assembler un payload JSON heterogene sans multiplier les DTO internes.
use serde_json::{Value, json};

use crate::{
    // `AppState` contient les handles partages: config, repositories et runtimes observables.
    app_state::AppState,
    // Les probes techniques reduisent les erreurs internes a des libelles admin non sensibles.
    errors::AppError,
    features::{
        // Le collecteur navigateur est stale selon une fenetre derivee de sa cadence de polling.
        ingestion::runtime::browser_collector_status_stale_after_ms,
        // Les statuts sources admin sont deja compactes et sans credentials.
        news::status::admin_source_statuses,
    },
    // Tous les timestamps exposes au client admin utilisent le meme format RFC3339 projet.
    shared::time::now_rfc3339,
};

/// Retourne un etat minimal permettant de verifier que le process repond.
pub async fn health() -> Json<serde_json::Value> {
    // La reponse reste volontairement minimale car ce endpoint sert aux healthchecks d'infra.
    Json(json!({ "status": "ok" }))
}

/// Retourne un diagnostic detaille reserve a l'admin.
pub async fn admin_health(State(state): State<AppState>) -> Json<Value> {
    let news_database = news_database_health(&state).await;
    // Le statut global ne depend ici que du stockage bloquant pour ne pas faire tomber
    // l'admin health sur un provider optionnel comme TTS, HLS ou edge.
    let status = if news_database.ok { "ok" } else { "degraded" };

    // `json!` garde ce handler local: le payload admin est composite et ne justifie pas
    // une struct publique stable tant que le contrat reste reserve a l'exploitation.
    Json(json!({
        "status": status,
        "checked_at": now_rfc3339(),
        "app": app_health(&state),
        "dependencies": {
            "news_database": news_database.payload,
            "translation": translation_health(&state),
            "tts": tts_health(&state),
            "hls": hls_health(&state),
            "sources": source_health(&state),
            "browser_collector": browser_collector_health(&state),
            "edge": edge_health(&state),
            "local_storage": local_storage_health(&state),
            "pools": pools_health(&state),
        }
    }))
}

// Petit conteneur interne pour separer le verdict machine (`ok`) du payload lisible admin.
struct HealthProbe {
    // `ok` sert au statut global sans reparcourir le JSON.
    ok: bool,
    // `Value` evite une hierarchie de DTO pour des diagnostics heterogenes.
    payload: Value,
}

async fn news_database_health(state: &AppState) -> HealthProbe {
    // SQLite est toujours requis pour le pipeline news/audio local.
    dependency_health("sqlite", true, state.news.health_check().await)
}

fn dependency_health(
    backend: &'static str,
    configured: bool,
    result: Result<(), AppError>,
) -> HealthProbe {
    // Une dependance non configuree est traitee comme indisponible, meme sans erreur I/O.
    let ok = configured && result.is_ok();
    // `err()` consomme le `Result` apres calcul de `ok`; on ne garde qu'un resume non sensible.
    let error_summary = result.err().map(|error| app_error_summary(&error));
    HealthProbe {
        ok,
        payload: json!({
            "status": if ok { "ok" } else { "error" },
            "backend": backend,
            "configured": configured,
            "reachable": ok,
            "error_summary": error_summary,
        }),
    }
}

fn app_health(state: &AppState) -> Value {
    // Cette section expose uniquement des metadonnees de runtime, jamais les valeurs secretes.
    json!({
        "version": env!("CARGO_PKG_VERSION"),
        "server_environment": state.config.runtime.server_env.as_str(),
        "auth_enabled": state.config.auth.enabled,
        "identity_provider": "better_auth",
    })
}

fn translation_health(state: &AppState) -> Value {
    // Le snapshot est une copie immuable du runtime; le handler ne verrouille pas le worker.
    let snapshot = state.translation_runtime.snapshot();
    // Les compteurs distinguent le debit, les erreurs et le backlog sans exposer le texte source.
    json!({
        "status": runtime_status(snapshot.enabled, &snapshot.last_error_at, &snapshot.last_success_at),
        "enabled": snapshot.enabled,
        "provider": snapshot.provider,
        "profile": snapshot.profile,
        "source_language": snapshot.source_language,
        "target_language": snapshot.target_language,
        "worker_count": snapshot.worker_count,
        "warmup_enabled": snapshot.warmup_enabled,
        "received_total": snapshot.received_total,
        "translated_total": snapshot.translated_total,
        "skipped_total": snapshot.skipped_total,
        "fallback_total": snapshot.fallback_total,
        "failed_total": snapshot.failed_total,
        "pending_total": snapshot.pending_total,
        "last_success_at": snapshot.last_success_at,
        "last_error_at": snapshot.last_error_at,
        "last_error_summary": runtime_error_summary(&snapshot.last_error),
        "warmup_completed_at": snapshot.warmup_completed_at,
        "warmup_duration_ms": snapshot.warmup_duration_ms,
        "translation_duration_p50_ms": snapshot.translation_duration_p50_ms,
        "translation_duration_p95_ms": snapshot.translation_duration_p95_ms,
    })
}

fn tts_health(state: &AppState) -> Value {
    // Meme pattern que la traduction: lecture instantanee du runtime puis projection admin.
    let snapshot = state.tts_runtime.snapshot();
    // La voix et les percentiles suffisent pour diagnostiquer le provider TTS sans payload audio.
    json!({
        "status": runtime_status(snapshot.enabled, &snapshot.last_error_at, &snapshot.last_success_at),
        "enabled": snapshot.enabled,
        "provider": snapshot.provider,
        "current_voice": snapshot.current_voice,
        "worker_count": snapshot.worker_count,
        "warmup_enabled": snapshot.warmup_enabled,
        "output_sample_rate_hz": snapshot.output_sample_rate_hz,
        "received_total": snapshot.received_total,
        "skipped_total": snapshot.skipped_total,
        "generated_total": snapshot.generated_total,
        "failed_total": snapshot.failed_total,
        "pending_total": snapshot.pending_total,
        "last_success_at": snapshot.last_success_at,
        "last_error_at": snapshot.last_error_at,
        "last_error_summary": runtime_error_summary(&snapshot.last_error),
        "warmup_completed_at": snapshot.warmup_completed_at,
        "warmup_duration_ms": snapshot.warmup_duration_ms,
        "generation_duration_p50_ms": snapshot.generation_duration_p50_ms,
        "generation_duration_p95_ms": snapshot.generation_duration_p95_ms,
    })
}

fn hls_health(state: &AppState) -> Value {
    // L'origin HLS porte le data plane audio; son snapshot mesure publications et lectures.
    let snapshot = state.hls_origin.snapshot();
    // Les credentials HLS sont exposes comme booleens de configuration, jamais comme secret.
    json!({
        "status": runtime_status(snapshot.enabled, &snapshot.last_error_at, &snapshot.last_success_at),
        "enabled": snapshot.enabled,
        "access_control_enabled": state.config.hls.access_control_enabled,
        "access_credentials_configured": credential_configured(
            state.config.hls.access_ticket_secret.as_deref(),
            None,
        ),
        "segments_dir_configured": !state.config.hls.segments_dir.trim().is_empty(),
        "replay_segment_target_duration_secs": snapshot.replay_segment_target_duration_secs,
        "clip_segment_target_duration_secs": snapshot.clip_segment_target_duration_secs,
        "clip_count": snapshot.clip_count,
        "replay_segment_count": snapshot.replay_segment_count,
        "replay_published_total": snapshot.replay_published_total,
        "clip_playlist_requests_total": snapshot.clip_playlist_requests_total,
        "clip_segment_requests_total": snapshot.clip_segment_requests_total,
        "replay_playlist_requests_total": snapshot.replay_playlist_requests_total,
        "replay_segment_requests_total": snapshot.replay_segment_requests_total,
        "failed_total": snapshot.failed_total,
        "media_encoder_failed_total": snapshot.ffmpeg_failed_total,
        "last_success_at": snapshot.last_success_at,
        "last_error_at": snapshot.last_error_at,
        "last_error_summary": runtime_error_summary(&snapshot.last_error),
        "publish_duration_p50_ms": snapshot.publish_duration_p50_ms,
        "publish_duration_p95_ms": snapshot.publish_duration_p95_ms,
        "render_duration_p50_ms": snapshot.render_duration_p50_ms,
        "render_duration_p95_ms": snapshot.render_duration_p95_ms,
    })
}

fn pools_health(state: &AppState) -> Value {
    let (news_size, news_idle) = state.news.pool_snapshot();
    json!({
        "news_database": {
            "backend": "sqlite",
            "max_connections": state.config.storage.max_connections,
            "size": news_size,
            "idle": news_idle,
        },
    })
}

fn source_health(state: &AppState) -> Value {
    // `Vec<Value>` permet de mapper chaque source admin sans figer un DTO public supplementaire.
    let sources: Vec<Value> = admin_source_statuses(state)
        .into_iter()
        .map(|source| {
            // Les statuts sources sont utiles au support: actif, stale, accessible ou fallback.
            json!({
                "source": source.name,
                "configured_enabled": source.configured_enabled,
                "enabled": source.enabled,
                "status": source.status,
                "message": source.message,
                "updated_at": source.updated_at,
                "service_accessible": source.service_accessible,
                "main_feed_available": source.main_feed_available,
                "stale": source.stale,
            })
        })
        .collect();
    // On renvoie un tableau JSON brut pour conserver la forme attendue par l'admin.
    json!(sources)
}

fn browser_collector_health(state: &AppState) -> Value {
    // Le snapshot combine config locale, etat du collecteur primaire et fraicheur du fallback.
    let snapshot = state.financial_juice_browser_runtime.snapshot(
        browser_ingest_credentials_configured(state),
        state.financial_juice_runtime.is_enabled(),
        // La fenetre stale depend de la cadence configuree pour eviter un faux rouge trop rapide.
        browser_collector_status_stale_after_ms(
            state
                .config
                .sources
                .financial_juice_browser
                .source
                .poll_interval_ms,
        ),
    );

    // Le token d'ingestion est expose comme booleen; le message garde le diagnostic operationnel.
    json!({
        "source": snapshot.source,
        "configured_enabled": snapshot.configured_enabled,
        "primary_enabled": snapshot.primary_enabled,
        "fallback_active": snapshot.fallback_active,
        "ingest_credentials_configured": snapshot.ingest_token_configured,
        "status": snapshot.status,
        "message": snapshot.message,
        "updated_at": snapshot.updated_at,
        "effective_enabled": snapshot.effective_enabled,
        "service_accessible": snapshot.service_accessible,
        "main_feed_available": snapshot.main_feed_available,
        "stale": snapshot.stale,
        "effective_message": snapshot.effective_message,
    })
}

fn edge_health(state: &AppState) -> Value {
    // On emprunte la config edge pour eviter de cloner des `Option<String>` potentiellement longues.
    let edge = &state.config.edge;
    // Le status "configured" signifie qu'au moins une partie edge est active: publication ou ticket.
    json!({
        "status": if edge.publish_enabled || edge.ticket_enabled { "configured" } else { "disabled" },
        "publish_enabled": edge.publish_enabled,
        "publish_endpoint_configured": credential_configured(
            edge.publish_url.as_deref(),
            Some("https://filfinance-live-edge.your-cloudflare-account.workers.dev/publish/live"),
        ),
        "publish_credentials_configured": credential_configured(
            edge.publish_secret.as_deref(),
            Some("your-edge-publish-secret"),
        ),
        "publish_timeout_ms": edge.publish_timeout_ms,
        "publish_retry_attempts": edge.publish_retry_attempts,
        "publish_retry_backoff_ms": edge.publish_retry_backoff_ms,
        "ticket_enabled": edge.ticket_enabled,
        "ticket_credentials_configured": credential_configured(
            edge.ticket_secret.as_deref(),
            Some("your-edge-ticket-secret-at-least-32-characters"),
        ),
        "ticket_ttl_secs": edge.ticket_ttl_secs,
    })
}

fn local_storage_health(state: &AppState) -> Value {
    // Cette section aide a verifier les chemins locaux sans exposer le chemin complet ou les secrets.
    json!({
        "database": {
            "kind": database_kind(&state.config.storage.database_url),
            "max_connections": state.config.storage.max_connections,
        },
        "logging": {
            "file_enabled": state.config.logging.file_enabled,
            "directory_configured": !state.config.logging.directory.trim().is_empty(),
            "retention_days": state.config.logging.retention_days,
            "admin_max_lines": state.config.logging.admin_max_lines,
        },
        "hls": {
            "enabled": state.config.hls.enabled,
            "segments_dir_configured": !state.config.hls.segments_dir.trim().is_empty(),
            "access_control_enabled": state.config.hls.access_control_enabled,
        },
    })
}

fn browser_ingest_credentials_configured(state: &AppState) -> bool {
    // On reutilise le helper generique pour traiter vide, absent et placeholder comme non configures.
    credential_configured(
        state
            .config
            .sources
            .financial_juice_browser
            .ingest_token
            .as_deref(),
        Some("your-browser-collector-token"),
    )
}

fn credential_configured(value: Option<&str>, placeholder: Option<&str>) -> bool {
    // `Option::map` nettoie la valeur seulement si elle existe.
    value
        .map(str::trim)
        // `is_some_and` exprime le predicat sans extraire manuellement le `Some`.
        .is_some_and(|value| !value.is_empty() && Some(value) != placeholder)
}

fn database_kind(database_url: &str) -> &'static str {
    // Le health admin expose la categorie de stockage, pas l'URL complete.
    if database_url == "sqlite::memory:" {
        "sqlite_memory"
    } else if database_url.starts_with("sqlite://") {
        "sqlite_file"
    } else {
        "unknown"
    }
}

fn runtime_status(
    enabled: bool,
    last_error_at: &Option<String>,
    last_success_at: &Option<String>,
) -> &'static str {
    // Un runtime desactive n'est pas degrade: il suit seulement la configuration.
    if !enabled {
        "disabled"
    // Une erreur sans succes connu signale une degradation visible en admin.
    } else if last_error_at.is_some() && last_success_at.is_none() {
        "degraded"
    } else {
        "ok"
    }
}

fn app_error_summary(error: &AppError) -> &'static str {
    // `match` force la couverture exhaustive des variantes d'erreur quand `AppError` evolue.
    match error {
        AppError::Sqlx(_) => "database query failed",
        AppError::Reqwest(_) => "http request failed",
        AppError::Io(_) => "io operation failed",
        AppError::Config(_) => "configuration check failed",
        AppError::Unauthorized(_) => "authorization check failed",
        AppError::Forbidden(_) => "permission check failed",
        AppError::NotFound(_) => "resource not found",
        AppError::Conflict(_) => "conflicting operation already running",
        AppError::Fetch { .. } => "upstream fetch failed",
        AppError::Parse { .. } => "upstream parse failed",
        AppError::Storage(_) => "storage operation failed",
        AppError::Downstream(_) => "downstream operation failed",
        AppError::SerdeJson(_) => "json operation failed",
    }
}

fn runtime_error_summary(error: &Option<String>) -> Option<&'static str> {
    // `as_deref()?` transforme `Option<String>` en `Option<&str>` et sort tot si absent.
    let error = error.as_deref()?;
    // La casse est normalisee pour garder des checks simples sur des erreurs providers libres.
    let normalized = error.to_ascii_lowercase();
    // Les libelles restent generiques pour eviter de renvoyer des details internes ou secrets.
    if normalized.contains("piper") {
        Some("tts provider error recorded")
    } else if normalized.contains("ffmpeg") {
        Some("media encoder error recorded")
    } else if normalized.contains("database") || normalized.contains("sqlite") {
        Some("database error recorded")
    } else if normalized.contains("cloudflare") {
        Some("cloudflare challenge recorded")
    } else {
        Some("runtime error recorded")
    }
}
