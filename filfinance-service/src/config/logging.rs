//! Configuration locale des logs applicatifs.

use crate::errors::AppError;

use super::{
    LoggingConfig,
    environment::{env_bool, env_or, env_usize},
};

impl LoggingConfig {
    /// Charge les options de logs depuis `NEWS_LOG_*`.
    pub fn from_env() -> Result<Self, AppError> {
        let retention_days = env_usize("NEWS_LOG_RETENTION_DAYS", 2)?;
        if retention_days != 2 {
            return Err(AppError::Config(
                "NEWS_LOG_RETENTION_DAYS must be 2 for the current J/J-1 retention policy"
                    .to_owned(),
            ));
        }

        let admin_max_lines = env_usize("NEWS_LOG_ADMIN_MAX_LINES", 500)?;
        if admin_max_lines == 0 {
            return Err(AppError::Config(
                "NEWS_LOG_ADMIN_MAX_LINES must be greater than 0".to_owned(),
            ));
        }

        Ok(Self {
            file_enabled: env_bool("NEWS_LOG_FILE_ENABLED", true)?,
            directory: env_or("NEWS_LOG_DIR", "data/logs"),
            retention_days,
            admin_max_lines,
        })
    }
}
