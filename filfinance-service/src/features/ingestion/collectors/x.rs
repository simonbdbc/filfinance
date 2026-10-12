//! Collecteur X branche sur l'API officielle recent search.

// `HashMap` relie les `author_id` des posts aux profils retournes dans `includes.users`.
use std::{collections::HashMap, sync::Mutex, time::Duration};

// `Client` execute les appels X; `RETRY_AFTER` sert a respecter les limites de debit.
use reqwest::{
    Client, StatusCode,
    header::{HeaderValue, RETRY_AFTER},
};
// Deserialize lit uniquement les champs necessaires du JSON X.
use serde::Deserialize;
// Le backoff est local au collecteur X, `watch` arrete le worker et l'intervalle cadence le polling.
use tokio::{
    sync::watch,
    task::JoinHandle,
    time::{MissedTickBehavior, interval, sleep},
};
// `OffsetDateTime` fabrique la fenetre initiale de recherche.
use time::{Duration as TimeDuration, OffsetDateTime, format_description::well_known::Rfc3339};
// Logs structures pour les fetchs, demarrages et rate limits.
use tracing::{debug, info, warn};
use url::Url;

use crate::{
    app_state::AppState,
    config::XConfig,
    errors::AppError,
    features::{
        ingestion::{collectors::emit_items_to_pipeline, runtime::XRuntimeSettings},
        news::domain::NewsEvent,
    },
    shared::text::normalize_whitespace,
};

/// Collecteur X recent search avec curseur `since_id` conserve en memoire.
pub struct XCollector {
    // La config X ajoute secrets et requete autour du contrat commun `SourceConfig`.
    config: XConfig,
    // Le client HTTP est clone car `reqwest::Client` partage deja son pool interne.
    client: Client,
    // `since_id` evite de redemander les posts deja vus apres le bootstrap initial.
    since_id: Mutex<Option<String>>,
    // Empreinte des listes actives; si elles changent, on relance une fenetre initiale.
    settings_fingerprint: Mutex<Option<String>>,
}

impl XCollector {
    /// Construit une instance prete pour le runner commun.
    pub fn new(config: XConfig, client: Client) -> Self {
        Self {
            config,
            client,
            since_id: Mutex::new(None),
            settings_fingerprint: Mutex::new(None),
        }
    }

    /// Nom logique de la source X.
    fn name(&self) -> &str {
        &self.config.source.name
    }

    /// Lit le curseur courant et le remet a zero si les listes X ont change.
    fn current_since_id(&self, settings: &XRuntimeSettings) -> Option<String> {
        let next_fingerprint = settings_fingerprint(settings);
        let mut fingerprint = self
            .settings_fingerprint
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if fingerprint.as_deref() != Some(next_fingerprint.as_str()) {
            *fingerprint = Some(next_fingerprint);
            *self
                .since_id
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) = None;
            return None;
        }

        self.since_id
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    /// Memorise le plus recent id renvoye par X pour les prochains tours de polling.
    fn remember_newest_id(&self, newest_id: Option<String>) {
        if let Some(newest_id) = newest_id {
            *self
                .since_id
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(newest_id);
        }
    }
    /// Execute un cycle X recent search avec les parametres runtime courants.
    async fn poll(
        &self,
        state: &AppState,
        settings: &XRuntimeSettings,
    ) -> Result<Vec<NewsEvent>, AppError> {
        let token = self
            .config
            .bearer_token
            .as_deref()
            .ok_or_else(|| AppError::Config("NEWS_X_BEARER_TOKEN is required".to_owned()))?;
        let since_id = self.current_since_id(settings);
        let url = build_recent_search_url(&self.config, settings, since_id.as_deref())?;

        let response = self
            .client
            .get(&url)
            .bearer_auth(token)
            .timeout(Duration::from_millis(self.config.source.request_timeout_ms))
            .send()
            .await
            .map_err(|error| AppError::fetch(self.name(), error.to_string()))?;

        if response.status() == StatusCode::TOO_MANY_REQUESTS {
            let retry_after_ms = retry_after_delay_ms(response.headers().get(RETRY_AFTER))
                .unwrap_or(self.config.source.rate_limit_backoff_ms);
            warn!(
                source = %self.name(),
                retry_after_ms,
                "X API rate limited, backing off before next poll"
            );
            sleep(Duration::from_millis(retry_after_ms)).await;
            return Ok(Vec::new());
        }

        if response.status() == StatusCode::PAYMENT_REQUIRED {
            let was_enabled = state.x_runtime.disable_after_payment_required();
            warn!(
                source = %self.name(),
                was_enabled,
                "X collector disabled after API returned 402 Payment Required"
            );
            return Ok(Vec::new());
        }

        let status = response.status();
        if !status.is_success() {
            return Err(AppError::fetch(
                self.name(),
                format!("X API recent search returned {status}"),
            ));
        }

        let payload = response
            .json::<XRecentSearchResponse>()
            .await
            .map_err(|error| AppError::parse(self.name(), error.to_string()))?;
        let newest_id = payload
            .meta
            .as_ref()
            .and_then(|meta| meta.newest_id.clone());
        let result_count = payload
            .meta
            .as_ref()
            .and_then(|meta| meta.result_count)
            .unwrap_or(0);
        self.remember_newest_id(newest_id);

        let events = normalize_recent_search_response(self.name(), payload);
        debug!(
            source = %self.name(),
            result_count,
            normalized_events = events.len(),
            "X recent search response fetched"
        );
        Ok(events)
    }
}

/// Lance le worker X dynamique. Il reste vivant meme quand la source est desactivee.
pub(crate) fn spawn(
    state: AppState,
    config: XConfig,
    client: Client,
    mut shutdown_rx: watch::Receiver<bool>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        if *shutdown_rx.borrow() {
            info!(source = %config.source.name, "X collector skipped after shutdown");
            return;
        }
        let source_name = config.source.name.clone();

        tokio::select! {
            changed = shutdown_rx.changed() => {
                if changed.is_err() || *shutdown_rx.borrow() {
                    info!(source = %source_name, "X collector received shutdown signal");
                }
            }
            _ = run_collector_loop(state, config, client) => {}
        }
    })
}

/// Boucle effective X; le wrapper public l'annule a la reception du shutdown process.
async fn run_collector_loop(state: AppState, config: XConfig, client: Client) {
    info!(
        source = %config.source.name,
        configured_enabled = config.source.enabled,
        url = %config.source.url,
        poll_interval_ms = config.source.poll_interval_ms,
        request_timeout_ms = config.source.request_timeout_ms,
        "X collector started"
    );

    let collector = XCollector::new(config.clone(), client);
    let mut ticker = interval(Duration::from_millis(config.source.poll_interval_ms));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
    let mut logged_disabled = false;

    loop {
        ticker.tick().await;
        let settings = state.x_runtime.settings();
        if !settings.enabled {
            if !logged_disabled {
                info!(source = %config.source.name, "X collector runtime disabled");
                logged_disabled = true;
            }
            continue;
        }

        logged_disabled = false;
        match collector.poll(&state, &settings).await {
            Ok(items) => {
                if !emit_items_to_pipeline(&config.source.name, items, &state.ingest_tx).await {
                    return;
                }
            }
            Err(error) => {
                warn!(%error, source = %config.source.name, "X collector poll failed")
            }
        }
    }
}

/// Reponse partielle de `/2/tweets/search/recent`.
#[derive(Debug, Deserialize)]
struct XRecentSearchResponse {
    #[serde(default)]
    data: Vec<XPost>,
    #[serde(default)]
    includes: XIncludes,
    #[serde(default)]
    meta: Option<XMeta>,
}

/// Bloc `includes` optionnel retourne par X.
#[derive(Debug, Default, Deserialize)]
struct XIncludes {
    #[serde(default)]
    users: Vec<XUser>,
}

/// Post X minimal pour alimenter le pipeline news.
#[derive(Debug, Deserialize)]
struct XPost {
    id: String,
    text: String,
    #[serde(default)]
    author_id: Option<String>,
    #[serde(default)]
    created_at: Option<String>,
}

/// Auteur X associe via `expansions=author_id`.
#[derive(Debug, Deserialize)]
struct XUser {
    id: String,
    username: String,
}

/// Metadata recent search utiles au curseur.
#[derive(Debug, Deserialize)]
struct XMeta {
    #[serde(default)]
    newest_id: Option<String>,
    #[serde(default)]
    result_count: Option<u64>,
}

/// Construit l'URL recent search avec les champs et le curseur attendus.
fn build_recent_search_url(
    config: &XConfig,
    settings: &XRuntimeSettings,
    since_id: Option<&str>,
) -> Result<String, AppError> {
    let mut url = Url::parse(&config.source.url)
        .map_err(|error| AppError::Config(format!("invalid NEWS_X_RECENT_SEARCH_URL: {error}")))?;
    let query = build_x_query(config, settings)?;

    {
        let mut pairs = url.query_pairs_mut();
        pairs
            .append_pair("query", &query)
            .append_pair("max_results", &config.max_results.to_string())
            .append_pair(
                "tweet.fields",
                "created_at,author_id,lang,entities,referenced_tweets",
            )
            .append_pair("expansions", "author_id")
            .append_pair("user.fields", "username")
            .append_pair("sort_order", "recency");

        if let Some(since_id) = since_id {
            pairs.append_pair("since_id", since_id);
        } else {
            pairs.append_pair("start_time", &startup_start_time(config)?);
        }
    }

    Ok(url.to_string())
}

/// Construit la requete X a partir d'une requete brute ou d'une liste de comptes.
fn build_x_query(config: &XConfig, settings: &XRuntimeSettings) -> Result<String, AppError> {
    let base = if !settings.account_usernames.is_empty() {
        build_account_and_keyword_query(&settings.account_usernames, &settings.keywords)
    } else if let Some(query) = config.query.as_deref() {
        normalize_whitespace(query)
    } else {
        return Err(AppError::Config(
            "X runtime requires at least one account or NEWS_X_QUERY".to_owned(),
        ));
    };
    let mut terms = vec![format!("({base})")];

    if let Some(language) = config
        .language
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        terms.push(format!("lang:{language}"));
    }

    if config.exclude_reposts {
        terms.push("-is:retweet".to_owned());
    }

    let query = terms.join(" ");
    if query.chars().count() > 1_024 {
        return Err(AppError::Config(
            "NEWS_X query is longer than the 1024 character X API limit".to_owned(),
        ));
    }

    Ok(query)
}

/// Transforme les listes admin en expression X.
fn build_account_and_keyword_query(account_usernames: &[String], keywords: &[String]) -> String {
    let account_query = account_usernames
        .iter()
        .map(|username| format!("from:{username}"))
        .collect::<Vec<_>>()
        .join(" OR ");

    if keywords.is_empty() {
        return account_query;
    }

    let keyword_query = keywords
        .iter()
        .map(|keyword| format_x_keyword(keyword))
        .collect::<Vec<_>>()
        .join(" OR ");

    format!("({account_query}) ({keyword_query})")
}

/// Echappe un mot cle pour la syntaxe recent search.
fn format_x_keyword(keyword: &str) -> String {
    let cleaned = keyword.replace('"', "").trim().to_owned();
    if cleaned.split_whitespace().count() > 1 {
        format!("\"{cleaned}\"")
    } else {
        cleaned
    }
}

/// Empreinte stable des listes qui determinent le curseur `since_id`.
fn settings_fingerprint(settings: &XRuntimeSettings) -> String {
    format!(
        "accounts={}|keywords={}",
        settings.account_usernames.join(",").to_ascii_lowercase(),
        settings.keywords.join(",").to_ascii_lowercase()
    )
}

/// Calcule le `start_time` du premier appel recent search.
fn startup_start_time(config: &XConfig) -> Result<String, AppError> {
    (OffsetDateTime::now_utc() - TimeDuration::minutes(config.startup_lookback_minutes as i64))
        .format(&Rfc3339)
        .map_err(|error| AppError::Config(format!("failed to format NEWS_X start_time: {error}")))
}

/// Convertit le JSON recent search en evenements canoniques.
fn normalize_recent_search_response(
    source_name: &str,
    payload: XRecentSearchResponse,
) -> Vec<NewsEvent> {
    let users_by_id = payload
        .includes
        .users
        .into_iter()
        .map(|user| (user.id.clone(), user))
        .collect::<HashMap<_, _>>();
    let mut events = payload
        .data
        .into_iter()
        .filter_map(|post| x_post_to_event(source_name, post, &users_by_id))
        .collect::<Vec<_>>();

    events.sort_by(|left, right| {
        left.published_at
            .cmp(&right.published_at)
            .then_with(|| left.url.cmp(&right.url))
    });
    events
}

/// Convertit un post X en headline Fil Finance.
fn x_post_to_event(
    source_name: &str,
    post: XPost,
    users_by_id: &HashMap<String, XUser>,
) -> Option<NewsEvent> {
    let text = normalize_whitespace(&post.text);
    if text.is_empty() {
        return None;
    }

    let username = post
        .author_id
        .as_ref()
        .and_then(|author_id| users_by_id.get(author_id))
        .map(|user| user.username.trim_start_matches('@').to_owned());
    let title = username
        .as_deref()
        .map(|username| format!("@{username}: {text}"))
        .unwrap_or_else(|| text.clone());
    let url = username
        .as_deref()
        .map(|username| format!("https://x.com/{username}/status/{}", post.id))
        .unwrap_or_else(|| format!("https://x.com/i/web/status/{}", post.id));
    let mut event = NewsEvent::new_headline(source_name, title, url, post.created_at, None);
    event.raw_payload_ref = Some(format!("x_post:{}", post.id));
    event.recompute_signature();

    Some(event)
}

/// Convertit `Retry-After` secondes vers millisecondes.
pub(in crate::features::ingestion) fn retry_after_delay_ms(
    value: Option<&HeaderValue>,
) -> Option<u64> {
    value
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.trim().parse::<u64>().ok())
        .map(|seconds| seconds.saturating_mul(1_000))
}

#[cfg(test)]
pub(in crate::features::ingestion) fn test_build_x_query(
    config: &XConfig,
) -> Result<String, AppError> {
    build_x_query(config, &test_settings_from_config(config))
}

#[cfg(test)]
pub(in crate::features::ingestion) fn test_build_recent_search_url(
    config: &XConfig,
    since_id: &str,
) -> Result<String, AppError> {
    build_recent_search_url(config, &test_settings_from_config(config), Some(since_id))
}

#[cfg(test)]
pub(in crate::features::ingestion) fn test_normalize_recent_search_json(
    source_name: &str,
    json: &str,
) -> Result<Vec<NewsEvent>, AppError> {
    let payload = serde_json::from_str::<XRecentSearchResponse>(json)?;
    Ok(normalize_recent_search_response(source_name, payload))
}

#[cfg(test)]
fn test_settings_from_config(config: &XConfig) -> XRuntimeSettings {
    XRuntimeSettings {
        enabled: config.source.enabled,
        disabled_reason: None,
        account_usernames: config.account_usernames.clone(),
        keywords: config.keywords.clone(),
        updated_at: None,
    }
}
