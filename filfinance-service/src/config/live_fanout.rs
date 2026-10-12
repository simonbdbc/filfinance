//! Configuration du fan-out live Redis reserve au pilote Bunny DEV.

use super::{
    LiveFanoutConfig,
    environment::{env_bool, env_optional, env_or},
};
use crate::errors::AppError;

impl LiveFanoutConfig {
    /// Charge le relais Redis sans rendre la production dependante de Bunny.
    pub fn from_env() -> Result<Self, AppError> {
        let enabled = env_bool("NEWS_LIVE_FANOUT_ENABLED", false)?;
        let redis_url = env_optional("NEWS_LIVE_REDIS_URL");
        let config = Self {
            enabled,
            redis_url,
            redis_channel: env_or("NEWS_LIVE_REDIS_CHANNEL", "filfinance:live:events"),
            redis_snapshot_key: env_or("NEWS_LIVE_REDIS_SNAPSHOT_KEY", "filfinance:live:snapshot"),
        };

        if config.enabled {
            let value = config
                .redis_url
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| {
                    AppError::Config(
                        "NEWS_LIVE_REDIS_URL is required when NEWS_LIVE_FANOUT_ENABLED=true"
                            .to_owned(),
                    )
                })?;
            let parsed = url::Url::parse(value).map_err(|error| {
                AppError::Config(format!("invalid NEWS_LIVE_REDIS_URL: {error}"))
            })?;
            if parsed.scheme() != "redis" || parsed.host_str().is_none() {
                return Err(AppError::Config(
                    "NEWS_LIVE_REDIS_URL must be a redis:// URL with a host".to_owned(),
                ));
            }
            if config.redis_channel.trim().is_empty() || config.redis_snapshot_key.trim().is_empty()
            {
                return Err(AppError::Config(
                    "NEWS_LIVE_REDIS_CHANNEL and NEWS_LIVE_REDIS_SNAPSHOT_KEY must not be empty"
                        .to_owned(),
                ));
            }
        }

        Ok(config)
    }
}
