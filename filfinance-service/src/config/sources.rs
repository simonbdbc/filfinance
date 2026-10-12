//! Configuration des collecteurs de marche et vues API de leurs statuts.

// Les helpers d'environnement gardent le parsing NEWS_* uniforme dans tous les sous-modules.
use super::{
    FinancialJuiceBrowserConfig, FinancialJuiceSiteConfig, SourceConfig, SourcesConfig, XConfig,
    environment::{env_bool, env_csv, env_i32, env_optional, env_or, env_u64, env_usize},
};

// Les statuts de source restent le contrat JSON expose par la feature news.
use crate::{errors::AppError, features::news::domain::SourceStatus};

impl SourcesConfig {
    /// Construit la configuration des collecteurs depuis l'environnement.
    pub fn from_env() -> Result<Self, AppError> {
        // La configuration FinancialJuice active ne porte plus que la source news authentifiee.
        let financial_juice = FinancialJuiceSiteConfig {
            name: "financial_juice".to_owned(),
            enabled: env_bool("NEWS_FINANCIAL_JUICE_ENABLED", true)?,
            home_url: env_or(
                "NEWS_FINANCIAL_JUICE_HOME_URL",
                "https://www.financialjuice.com/home",
            ),
            poll_interval_ms: env_u64("NEWS_FINANCIAL_JUICE_POLL_INTERVAL_MS", 5_000)?,
            request_timeout_ms: env_u64("NEWS_FINANCIAL_JUICE_REQUEST_TIMEOUT_MS", 5_000)?,
            rate_limit_backoff_ms: env_u64("NEWS_FINANCIAL_JUICE_RATE_LIMIT_BACKOFF_MS", 60_000)?,
            session_refresh_interval_ms: env_u64(
                "NEWS_FINANCIAL_JUICE_SESSION_REFRESH_INTERVAL_MS",
                30 * 60 * 1_000,
            )?,
            live_retry_backoff_ms: env_u64("NEWS_FINANCIAL_JUICE_LIVE_RETRY_BACKOFF_MS", 5_000)?,
            startup_backfill_page_limit: env_usize(
                "NEWS_FINANCIAL_JUICE_STARTUP_BACKFILL_PAGE_LIMIT",
                48,
            )?,
            time_offset_minutes: env_i32("NEWS_FINANCIAL_JUICE_TIME_OFFSET_MINUTES", 0)?,
            email: env_optional("NEWS_FINANCIAL_JUICE_EMAIL"),
            password: env_optional("NEWS_FINANCIAL_JUICE_PASSWORD"),
        };
        let financial_juice_browser = FinancialJuiceBrowserConfig {
            source: SourceConfig {
                name: "financial_juice_browser".to_owned(),
                enabled: env_bool("NEWS_FINANCIAL_JUICE_BROWSER_ENABLED", false)?,
                url: env_or(
                    "NEWS_FINANCIAL_JUICE_BROWSER_HOME_URL",
                    &financial_juice.home_url,
                ),
                poll_interval_ms: env_u64("NEWS_FINANCIAL_JUICE_BROWSER_POLL_INTERVAL_MS", 5_000)?,
                request_timeout_ms: env_u64(
                    "NEWS_FINANCIAL_JUICE_BROWSER_REQUEST_TIMEOUT_MS",
                    15_000,
                )?,
                rate_limit_backoff_ms: env_u64(
                    "NEWS_FINANCIAL_JUICE_BROWSER_RATE_LIMIT_BACKOFF_MS",
                    60_000,
                )?,
            },
            ingest_token: env_optional("NEWS_FINANCIAL_JUICE_BROWSER_INGEST_TOKEN"),
            max_batch_size: env_usize("NEWS_FINANCIAL_JUICE_BROWSER_MAX_BATCH_SIZE", 100)?,
        };
        let x = XConfig {
            source: SourceConfig {
                name: "x".to_owned(),
                enabled: env_bool("NEWS_X_ENABLED", false)?,
                url: env_or(
                    "NEWS_X_RECENT_SEARCH_URL",
                    "https://api.x.com/2/tweets/search/recent",
                ),
                poll_interval_ms: env_u64("NEWS_X_POLL_INTERVAL_MS", 15_000)?,
                request_timeout_ms: env_u64("NEWS_X_REQUEST_TIMEOUT_MS", 5_000)?,
                rate_limit_backoff_ms: env_u64("NEWS_X_RATE_LIMIT_BACKOFF_MS", 60_000)?,
            },
            bearer_token: env_optional("NEWS_X_BEARER_TOKEN"),
            query: env_optional("NEWS_X_QUERY"),
            account_usernames: env_csv("NEWS_X_ACCOUNT_USERNAMES")
                .into_iter()
                .map(normalize_x_username)
                .collect(),
            keywords: env_csv("NEWS_X_KEYWORDS")
                .into_iter()
                .map(normalize_x_keyword)
                .filter(|value| !value.is_empty())
                .collect(),
            language: env_optional("NEWS_X_LANGUAGE"),
            exclude_reposts: env_bool("NEWS_X_EXCLUDE_REPOSTS", true)?,
            max_results: env_usize("NEWS_X_MAX_RESULTS", 20)?,
            startup_lookback_minutes: env_u64("NEWS_X_STARTUP_LOOKBACK_MINUTES", 120)?,
        };

        validate_financial_juice_site_config(&financial_juice)?;
        validate_financial_juice_browser_config(&financial_juice_browser)?;
        validate_x_config(&x)?;

        Ok(Self {
            financial_juice,
            financial_juice_browser,
            x,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn financial_juice_browser_accepts_valid_permanent_collector_config() {
        let config = financial_juice_browser_config();

        validate_financial_juice_browser_config(&config)
            .expect("permanent browser collector config should be valid");
    }

    #[test]
    fn financial_juice_browser_requires_ingest_token_when_enabled() {
        let mut config = financial_juice_browser_config();
        config.ingest_token = None;

        let error = validate_financial_juice_browser_config(&config)
            .expect_err("enabled browser collector without ingest token should be rejected");

        assert!(
            error
                .to_string()
                .contains("NEWS_FINANCIAL_JUICE_BROWSER_INGEST_TOKEN is required")
        );
    }

    #[test]
    fn financial_juice_browser_rejects_invalid_batch_size() {
        let mut config = financial_juice_browser_config();
        config.max_batch_size = 0;

        let error = validate_financial_juice_browser_config(&config)
            .expect_err("invalid browser batch size should be rejected");

        assert!(
            error
                .to_string()
                .contains("NEWS_FINANCIAL_JUICE_BROWSER_MAX_BATCH_SIZE must be between 1 and 500")
        );
    }

    fn financial_juice_browser_config() -> FinancialJuiceBrowserConfig {
        FinancialJuiceBrowserConfig {
            source: SourceConfig {
                name: "financial_juice_browser".to_owned(),
                enabled: true,
                url: "https://www.financialjuice.com/home".to_owned(),
                poll_interval_ms: 5_000,
                request_timeout_ms: 15_000,
                rate_limit_backoff_ms: 60_000,
            },
            ingest_token: Some("ingest-token".to_owned()),
            max_batch_size: 100,
        }
    }
}

/// Valide la coherence de la configuration FinancialJuice site avant le demarrage.
fn validate_financial_juice_site_config(config: &FinancialJuiceSiteConfig) -> Result<(), AppError> {
    if !config.enabled {
        return Ok(());
    }

    if config.home_url.trim().is_empty() {
        return Err(AppError::Config(
            "NEWS_FINANCIAL_JUICE_HOME_URL cannot be empty".to_owned(),
        ));
    }

    if config.startup_backfill_page_limit == 0 {
        return Err(AppError::Config(
            "NEWS_FINANCIAL_JUICE_STARTUP_BACKFILL_PAGE_LIMIT must be greater than 0".to_owned(),
        ));
    }

    if config.session_refresh_interval_ms == 0 {
        return Err(AppError::Config(
            "NEWS_FINANCIAL_JUICE_SESSION_REFRESH_INTERVAL_MS must be greater than 0".to_owned(),
        ));
    }

    if config.live_retry_backoff_ms == 0 {
        return Err(AppError::Config(
            "NEWS_FINANCIAL_JUICE_LIVE_RETRY_BACKOFF_MS must be greater than 0".to_owned(),
        ));
    }

    if config
        .email
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_none()
    {
        return Err(AppError::Config(
            "NEWS_FINANCIAL_JUICE_EMAIL is required when FinancialJuice site is enabled".to_owned(),
        ));
    }

    if config
        .password
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_none()
    {
        return Err(AppError::Config(
            "NEWS_FINANCIAL_JUICE_PASSWORD is required when FinancialJuice site is enabled"
                .to_owned(),
        ));
    }

    Ok(())
}

/// Valide l'entree d'ingestion machine-to-machine du collecteur navigateur.
fn validate_financial_juice_browser_config(
    config: &FinancialJuiceBrowserConfig,
) -> Result<(), AppError> {
    if config.source.url.trim().is_empty() {
        return Err(AppError::Config(
            "NEWS_FINANCIAL_JUICE_BROWSER_HOME_URL cannot be empty".to_owned(),
        ));
    }

    if config.max_batch_size == 0 || config.max_batch_size > 500 {
        return Err(AppError::Config(
            "NEWS_FINANCIAL_JUICE_BROWSER_MAX_BATCH_SIZE must be between 1 and 500".to_owned(),
        ));
    }

    if config.source.enabled
        && config
            .ingest_token
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_none()
    {
        return Err(AppError::Config(
            "NEWS_FINANCIAL_JUICE_BROWSER_INGEST_TOKEN is required when the browser collector is enabled"
                .to_owned(),
        ));
    }

    Ok(())
}

/// Valide le collecteur X uniquement quand il est active.
fn validate_x_config(config: &XConfig) -> Result<(), AppError> {
    if !config.source.enabled {
        return Ok(());
    }

    if config.source.url.trim().is_empty() {
        return Err(AppError::Config(
            "NEWS_X_RECENT_SEARCH_URL cannot be empty".to_owned(),
        ));
    }

    if config
        .bearer_token
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_none()
    {
        return Err(AppError::Config(
            "NEWS_X_BEARER_TOKEN is required when X ingestion is enabled".to_owned(),
        ));
    }

    if config.query.is_none() && config.account_usernames.is_empty() {
        return Err(AppError::Config(
            "NEWS_X_QUERY or NEWS_X_ACCOUNT_USERNAMES is required when X ingestion is enabled"
                .to_owned(),
        ));
    }

    if config.max_results < 10 || config.max_results > 100 {
        return Err(AppError::Config(
            "NEWS_X_MAX_RESULTS must be between 10 and 100".to_owned(),
        ));
    }

    if config.startup_lookback_minutes == 0 || config.startup_lookback_minutes > 7 * 24 * 60 {
        return Err(AppError::Config(
            "NEWS_X_STARTUP_LOOKBACK_MINUTES must be between 1 and 10080".to_owned(),
        ));
    }

    if config.source.poll_interval_ms == 0 {
        return Err(AppError::Config(
            "NEWS_X_POLL_INTERVAL_MS must be greater than 0".to_owned(),
        ));
    }

    if let Some(invalid) = config
        .account_usernames
        .iter()
        .find(|username| !is_valid_x_username(username))
    {
        return Err(AppError::Config(format!(
            "invalid NEWS_X_ACCOUNT_USERNAMES entry: {invalid}"
        )));
    }

    Ok(())
}

/// Normalise un handle X en supprimant le prefixe `@` facultatif.
fn normalize_x_username(value: String) -> String {
    value.trim().trim_start_matches('@').to_owned()
}

/// Normalise un mot cle X issu de l'environnement.
fn normalize_x_keyword(value: String) -> String {
    value.trim().trim_matches('"').to_owned()
}

/// Valide la forme historique des handles X.
fn is_valid_x_username(value: &str) -> bool {
    let len = value.chars().count();
    (1..=15).contains(&len)
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

impl SourcesConfig {
    /// Retourne la vue API des sources connues du service.
    pub fn statuses(&self) -> Vec<SourceStatus> {
        vec![
            SourceStatus::from_financial_juice_site(&self.financial_juice),
            SourceStatus::from_config(&self.financial_juice_browser.source),
            SourceStatus::from_config(&self.x.source),
        ]
    }
}
