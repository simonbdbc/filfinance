//! Handlers operateur pour les controles runtime d'ingestion.

// `HeaderMap`, `Json` et `State` convertissent la requete Axum en DTO, headers et etat partage.
use axum::{
    Json,
    extract::State,
    http::{HeaderMap, header::AUTHORIZATION},
};
// `Deserialize` parse les bodies JSON, `Serialize` formate les reponses de l'entree navigateur.
use serde::{Deserialize, Serialize};
// `Rfc3339` valide les dates envoyees par le collecteur navigateur.
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use tracing::warn;

use crate::{
    app_state::AppState,
    errors::AppError,
    features::{
        ingestion::collectors::emit_items_to_pipeline,
        news::domain::{EventKind, NewsEvent},
    },
    shared::{text::normalize_whitespace, time::now_rfc3339},
};

use super::runtime::{
    BrowserCollectorStatus, FinancialJuiceBrowserRuntimeStatus, FinancialJuiceRuntimeStatus,
    XRuntimeSettings, XRuntimeStatus, browser_collector_status_stale_after_ms,
};

/// Body accepte par le switch admin FinancialJuice.
#[derive(Debug, Deserialize)]
pub struct UpdateFinancialJuiceRuntimeRequest {
    /// Etat effectif demande pour le collecteur FinancialJuice.
    pub enabled: bool,
}

/// Body accepte par le panneau admin X.
#[derive(Debug, Deserialize)]
pub struct UpdateXRuntimeRequest {
    /// Etat effectif demande pour le collecteur X.
    pub enabled: Option<bool>,
    /// Comptes X surveilles, avec ou sans prefixe `@`.
    pub account_usernames: Option<Vec<String>>,
    /// Mots cles optionnels qui filtrent les posts des comptes surveilles.
    pub keywords: Option<Vec<String>>,
}

/// Body machine-to-machine envoye par le collecteur navigateur FinancialJuice.
#[derive(Debug, Deserialize)]
pub struct BrowserNewsBatchRequest {
    /// Evenements detectes par Playwright dans l'ordre source.
    pub events: Vec<BrowserNewsEventRequest>,
}

/// Evenement minimal navigateur avant conversion vers `NewsEvent`.
#[derive(Debug, Deserialize)]
pub struct BrowserNewsEventRequest {
    /// Identifiant source si FinancialJuice ou le DOM en expose un.
    pub external_id: Option<String>,
    /// Titre visible de la news.
    pub title: String,
    /// Description ou detail optionnel.
    pub description: Option<String>,
    /// URL detail FinancialJuice si disponible.
    pub url: Option<String>,
    /// Timestamp source en RFC3339 quand il est connu.
    pub published_at: Option<String>,
    /// Niveau FinancialJuice brut, par exemple `active active-critical`.
    pub news_level: Option<String>,
}

/// Statut operationnel envoye par le process navigateur.
#[derive(Debug, Deserialize)]
pub struct BrowserCollectorStatusRequest {
    /// Etat explicite du collecteur externe.
    pub status: BrowserCollectorStatus,
    /// Message court associe au statut.
    pub message: Option<String>,
}

/// Reponse stable de l'entree d'ingestion navigateur.
#[derive(Debug, Serialize)]
pub struct BrowserNewsBatchResponse {
    /// Nombre d'evenements acceptes et envoyes dans la queue canonique.
    pub accepted: usize,
    /// Nombre d'evenements ignores localement car vides ou invalides.
    pub dropped: usize,
}

/// Reponse machine-to-machine lue par le collecteur avant d'ouvrir Chromium.
#[derive(Debug, Serialize)]
pub struct BrowserCollectorControlResponse {
    /// Source controlee par cette entree.
    pub source: String,
    /// Indique si le fallback navigateur est configure cote backend.
    pub configured_enabled: bool,
    /// Indique si le collecteur Rust classique est actif.
    pub primary_enabled: bool,
    /// Indique si le collecteur navigateur doit collecter maintenant.
    pub fallback_active: bool,
    /// Frequence de polling a appliquer par le collecteur externe.
    pub poll_interval_ms: u64,
    /// Message operationnel court pour logs collector.
    pub message: String,
}

/// Retourne l'etat effectif et le fallback env du collecteur FinancialJuice.
pub async fn financial_juice_runtime(
    State(state): State<AppState>,
) -> Json<FinancialJuiceRuntimeStatus> {
    Json(state.financial_juice_runtime.snapshot())
}

/// Met a jour l'override runtime FinancialJuice pour le process courant.
pub async fn update_financial_juice_runtime(
    State(state): State<AppState>,
    Json(request): Json<UpdateFinancialJuiceRuntimeRequest>,
) -> Result<Json<FinancialJuiceRuntimeStatus>, AppError> {
    if request.enabled {
        ensure_financial_juice_credentials_available(&state)?;
    }

    state.financial_juice_runtime.set_enabled(request.enabled);
    Ok(Json(state.financial_juice_runtime.snapshot()))
}

fn ensure_financial_juice_credentials_available(state: &AppState) -> Result<(), AppError> {
    let config = &state.config.sources.financial_juice;
    let has_email = config
        .email
        .as_deref()
        .map(str::trim)
        .is_some_and(|value| !value.is_empty());
    let has_password = config
        .password
        .as_deref()
        .map(str::trim)
        .is_some_and(|value| !value.is_empty());

    if has_email && has_password {
        return Ok(());
    }

    Err(AppError::Config(
        "FinancialJuice cannot be enabled at runtime without NEWS_FINANCIAL_JUICE_EMAIL and NEWS_FINANCIAL_JUICE_PASSWORD"
            .to_owned(),
    ))
}

/// Retourne le dernier statut declare par le collecteur navigateur externe.
pub async fn financial_juice_browser_runtime(
    State(state): State<AppState>,
) -> Json<FinancialJuiceBrowserRuntimeStatus> {
    Json(financial_juice_browser_runtime_snapshot(&state))
}

/// Retourne au collecteur externe s'il doit travailler ou rester en veille.
pub async fn financial_juice_browser_control(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<BrowserCollectorControlResponse>, AppError> {
    ensure_financial_juice_browser_authorized(&state, &headers)?;
    let fallback_active = financial_juice_browser_fallback_active(&state);
    let primary_enabled = state.financial_juice_runtime.is_enabled();

    Ok(Json(BrowserCollectorControlResponse {
        source: state
            .config
            .sources
            .financial_juice_browser
            .source
            .name
            .clone(),
        configured_enabled: state.config.sources.financial_juice_browser.source.enabled,
        primary_enabled,
        fallback_active,
        poll_interval_ms: state
            .config
            .sources
            .financial_juice_browser
            .source
            .poll_interval_ms,
        message: if fallback_active {
            "fallback active: financial_juice primary is disabled".to_owned()
        } else if primary_enabled {
            "fallback idle: financial_juice primary is active".to_owned()
        } else {
            "fallback disabled by NEWS_FINANCIAL_JUICE_BROWSER_ENABLED=false".to_owned()
        },
    }))
}

/// Recoit un heartbeat/statut machine-to-machine du collecteur navigateur.
pub async fn update_financial_juice_browser_status(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<BrowserCollectorStatusRequest>,
) -> Result<Json<FinancialJuiceBrowserRuntimeStatus>, AppError> {
    ensure_financial_juice_browser_authorized(&state, &headers)?;
    state
        .financial_juice_browser_runtime
        .update_status(request.status, normalize_optional_text(request.message));

    let snapshot = financial_juice_browser_runtime_snapshot(&state);
    if snapshot.fallback_active && !snapshot.effective_enabled {
        warn!(
            source = %snapshot.source,
            status = ?snapshot.status,
            service_accessible = snapshot.service_accessible,
            main_feed_available = snapshot.main_feed_available,
            stale = snapshot.stale,
            message = %snapshot.effective_message.as_deref().unwrap_or(""),
            "financial juice browser fallback is not usable"
        );
    }

    Ok(Json(snapshot))
}

/// Injecte les headlines lues par le collecteur navigateur dans le pipeline canonique.
pub async fn ingest_financial_juice_browser_news(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<BrowserNewsBatchRequest>,
) -> Result<Json<BrowserNewsBatchResponse>, AppError> {
    ensure_financial_juice_browser_authorized(&state, &headers)?;
    ensure_financial_juice_browser_fallback_active(&state)?;
    validate_browser_batch_size(&state, request.events.len())?;

    let mut dropped = 0usize;
    let mut events = Vec::new();
    for raw_event in request.events {
        match browser_news_event_from_request(&state, raw_event) {
            Ok(Some(event)) => events.push(event),
            Ok(None) => dropped += 1,
            Err(_) => dropped += 1,
        }
    }

    let accepted = events.len();
    if accepted > 0
        && !emit_items_to_pipeline(
            &state.config.sources.financial_juice_browser.source.name,
            events,
            &state.ingest_tx,
        )
        .await
    {
        return Err(AppError::storage("ingest channel closed"));
    }

    state.financial_juice_browser_runtime.update_status(
        BrowserCollectorStatus::Ok,
        Some("news batch accepted; #mainFeed available".to_owned()),
    );

    Ok(Json(BrowserNewsBatchResponse { accepted, dropped }))
}

fn financial_juice_browser_runtime_snapshot(
    state: &AppState,
) -> FinancialJuiceBrowserRuntimeStatus {
    state.financial_juice_browser_runtime.snapshot(
        financial_juice_browser_ingest_token_configured(state),
        state.financial_juice_runtime.is_enabled(),
        browser_collector_status_stale_after_ms(
            state
                .config
                .sources
                .financial_juice_browser
                .source
                .poll_interval_ms,
        ),
    )
}

fn ensure_financial_juice_browser_fallback_active(state: &AppState) -> Result<(), AppError> {
    if financial_juice_browser_fallback_active(state) {
        return Ok(());
    }

    if state.financial_juice_runtime.is_enabled() {
        return Err(AppError::forbidden(
            "FinancialJuice browser collector is a fallback and cannot ingest while financial_juice is enabled",
        ));
    }

    Err(AppError::forbidden(
        "FinancialJuice browser collector fallback is disabled",
    ))
}

fn financial_juice_browser_fallback_active(state: &AppState) -> bool {
    state.config.sources.financial_juice_browser.source.enabled
        && !state.financial_juice_runtime.is_enabled()
}

fn ensure_financial_juice_browser_authorized(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<(), AppError> {
    let expected = state
        .config
        .sources
        .financial_juice_browser
        .ingest_token
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            AppError::forbidden(
                "FinancialJuice browser ingest token is not configured on the backend",
            )
        })?;

    let actual = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim)
        .ok_or_else(|| AppError::unauthorized("missing browser collector bearer token"))?;

    if actual == expected {
        Ok(())
    } else {
        Err(AppError::unauthorized(
            "invalid browser collector bearer token",
        ))
    }
}

fn financial_juice_browser_ingest_token_configured(state: &AppState) -> bool {
    state
        .config
        .sources
        .financial_juice_browser
        .ingest_token
        .as_deref()
        .map(str::trim)
        .is_some_and(|value| !value.is_empty() && value != "your-browser-collector-token")
}

fn validate_browser_batch_size(state: &AppState, len: usize) -> Result<(), AppError> {
    let max_batch_size = state.config.sources.financial_juice_browser.max_batch_size;
    if len <= max_batch_size {
        return Ok(());
    }

    Err(AppError::Config(format!(
        "FinancialJuice browser batch contains {len} events, max is {max_batch_size}"
    )))
}

fn browser_news_event_from_request(
    state: &AppState,
    request: BrowserNewsEventRequest,
) -> Result<Option<NewsEvent>, AppError> {
    let title = normalize_whitespace(&request.title);
    if title.is_empty() {
        return Ok(None);
    }

    let published_at = normalize_optional_rfc3339(request.published_at)?;
    let mut event = NewsEvent {
        source: state
            .config
            .sources
            .financial_juice_browser
            .source
            .name
            .clone(),
        event_kind: EventKind::Headline,
        title,
        url: normalize_optional_text(request.url).unwrap_or_else(|| {
            state
                .config
                .sources
                .financial_juice_browser
                .source
                .url
                .clone()
        }),
        published_at,
        event_time: None,
        ingested_at: now_rfc3339(),
        body: normalize_optional_text(request.description),
        news_level: normalize_optional_text(request.news_level),
        raw_payload_ref: normalize_optional_text(request.external_id)
            .map(|external_id| format!("financial_juice_browser:{external_id}")),
        macro_metadata: None,
        signature: String::new(),
    };
    event.recompute_signature();

    Ok(Some(event))
}

fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value
        .map(|raw| normalize_whitespace(&raw))
        .filter(|value| !value.is_empty())
}

fn normalize_optional_rfc3339(value: Option<String>) -> Result<Option<String>, AppError> {
    let Some(value) = normalize_optional_text(value) else {
        return Ok(None);
    };

    OffsetDateTime::parse(&value, &Rfc3339).map_err(|error| {
        AppError::Config(format!(
            "invalid FinancialJuice browser published_at `{value}`: {error}"
        ))
    })?;

    Ok(Some(value))
}

/// Retourne l'etat effectif et les listes persistées du collecteur X.
pub async fn x_runtime(State(state): State<AppState>) -> Json<XRuntimeStatus> {
    Json(x_runtime_snapshot(&state))
}

/// Met a jour le runtime X et persiste les listes dans SQLite.
pub async fn update_x_runtime(
    State(state): State<AppState>,
    Json(request): Json<UpdateXRuntimeRequest>,
) -> Result<Json<XRuntimeStatus>, AppError> {
    let mut settings = state.x_runtime.settings();

    if let Some(enabled) = request.enabled {
        settings.enabled = enabled;
    }

    if let Some(account_usernames) = request.account_usernames {
        settings.account_usernames = normalize_x_usernames(account_usernames)?;
    }

    if let Some(keywords) = request.keywords {
        settings.keywords = normalize_x_keywords(keywords)?;
    }

    validate_x_runtime_settings(&settings)?;
    if settings.enabled {
        ensure_x_bearer_token_available(&state)?;
    }

    let persisted = state.x_settings_store.save(&settings).await?;
    state.x_runtime.replace_settings(persisted);

    Ok(Json(x_runtime_snapshot(&state)))
}

fn x_runtime_snapshot(state: &AppState) -> XRuntimeStatus {
    state
        .x_runtime
        .snapshot(x_bearer_token_configured(state), true)
}

fn ensure_x_bearer_token_available(state: &AppState) -> Result<(), AppError> {
    if x_bearer_token_configured(state) {
        return Ok(());
    }

    Err(AppError::Config(
        "X cannot be enabled at runtime without NEWS_X_BEARER_TOKEN".to_owned(),
    ))
}

fn x_bearer_token_configured(state: &AppState) -> bool {
    state
        .config
        .sources
        .x
        .bearer_token
        .as_deref()
        .map(str::trim)
        .is_some_and(|value| !value.is_empty() && value != "your-x-api-bearer-token")
}

fn validate_x_runtime_settings(settings: &XRuntimeSettings) -> Result<(), AppError> {
    if settings.account_usernames.is_empty() {
        return Err(AppError::Config(
            "At least one X account is required".to_owned(),
        ));
    }

    if settings.account_usernames.len() > 50 {
        return Err(AppError::Config(
            "X account list cannot contain more than 50 entries".to_owned(),
        ));
    }

    if settings.keywords.len() > 100 {
        return Err(AppError::Config(
            "X keyword list cannot contain more than 100 entries".to_owned(),
        ));
    }

    Ok(())
}

fn normalize_x_usernames(values: Vec<String>) -> Result<Vec<String>, AppError> {
    let mut normalized = Vec::new();
    for raw in values {
        let value = raw.trim().trim_start_matches('@').to_owned();
        if value.is_empty() {
            continue;
        }
        if !is_valid_x_username(&value) {
            return Err(AppError::Config(format!("invalid X account: {raw}")));
        }
        if !normalized
            .iter()
            .any(|existing: &String| existing.eq_ignore_ascii_case(&value))
        {
            normalized.push(value);
        }
    }

    Ok(normalized)
}

fn normalize_x_keywords(values: Vec<String>) -> Result<Vec<String>, AppError> {
    let mut normalized = Vec::new();
    for raw in values {
        let value = raw.trim().trim_matches('"').to_owned();
        if value.is_empty() {
            continue;
        }
        if value.chars().count() > 80 {
            return Err(AppError::Config(format!("X keyword is too long: {raw}")));
        }
        if !normalized
            .iter()
            .any(|existing: &String| existing.eq_ignore_ascii_case(&value))
        {
            normalized.push(value);
        }
    }

    Ok(normalized)
}

fn is_valid_x_username(value: &str) -> bool {
    let len = value.chars().count();
    (1..=15).contains(&len)
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

#[cfg(test)]
pub(in crate::features::ingestion) fn test_browser_news_event_from_request(
    state: &AppState,
    request: BrowserNewsEventRequest,
) -> Result<Option<NewsEvent>, AppError> {
    browser_news_event_from_request(state, request)
}
