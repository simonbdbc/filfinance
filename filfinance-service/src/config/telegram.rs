//! Configuration du downstream Telegram.

// Les helpers d'environnement gardent le parsing NEWS_* uniforme dans tous les sous-modules.
use super::{
    TelegramConfig,
    environment::{env_bool, env_csv, env_optional, env_or, env_u64, env_usize},
};

// Les erreurs de parsing doivent remonter au bootstrap applicatif.
use crate::errors::AppError;

impl TelegramConfig {
    /// Construit la configuration Telegram depuis l'environnement.
    pub fn from_env() -> Result<Self, AppError> {
        Ok(Self {
            enabled: env_bool("NEWS_TELEGRAM_ENABLED", false)?,
            // Telegram garde une valeur par defaut reelle pour que les tests/doc restent simples.
            api_base_url: env_or("NEWS_TELEGRAM_API_BASE_URL", "https://api.telegram.org"),
            bot_token: env_optional("NEWS_TELEGRAM_BOT_TOKEN"),
            chat_ids: env_csv("NEWS_TELEGRAM_CHAT_IDS"),
            queue_buffer: env_usize("NEWS_TELEGRAM_QUEUE_BUFFER", 512)?,
            rate_limit_ms: env_u64("NEWS_TELEGRAM_RATE_LIMIT_MS", 250)?,
            retry_attempts: env_usize("NEWS_TELEGRAM_RETRY_ATTEMPTS", 2)?,
            retry_backoff_ms: env_u64("NEWS_TELEGRAM_RETRY_BACKOFF_MS", 750)?,
        })
    }
}
