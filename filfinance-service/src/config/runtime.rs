//! Configuration runtime et garde-fous de securite lies a l'environnement serveur.

use super::{
    AuthConfig, RuntimeConfig, ServerEnvironment,
    environment::{env_or, env_u64, env_usize},
};
use crate::errors::AppError;

impl ServerEnvironment {
    /// Parse la valeur `NEWS_SERVER_ENV`.
    pub fn parse(value: &str) -> Result<Self, AppError> {
        match value.trim().to_ascii_lowercase().as_str() {
            "local" | "dev" | "development" | "test" => Ok(Self::Local),
            "prod" | "production" => Ok(Self::Production),
            other => Err(AppError::Config(format!(
                "invalid NEWS_SERVER_ENV value: {other}; expected local|production"
            ))),
        }
    }

    /// Valeur stable utile dans les logs et les diagnostics.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Production => "production",
        }
    }

    /// Indique si les garde-fous production doivent s'appliquer.
    pub fn is_production(&self) -> bool {
        matches!(self, Self::Production)
    }
}

/// Construit les parametres runtime transverses depuis l'environnement.
pub(super) fn runtime_config_from_env() -> Result<RuntimeConfig, AppError> {
    Ok(RuntimeConfig {
        server_env: ServerEnvironment::parse(&env_or("NEWS_SERVER_ENV", "local"))?,
        listen_addr: env_or("NEWS_LISTEN_ADDR", "0.0.0.0:8088"),
        ingest_buffer: env_usize("NEWS_INGEST_BUFFER", 2048)?,
        broadcast_buffer: env_usize("NEWS_BROADCAST_BUFFER", 2048)?,
        dedup_ttl_secs: env_u64("NEWS_DEDUP_TTL_SECS", 300)?,
        slow_request_threshold_ms: env_u64("NEWS_HTTP_SLOW_REQUEST_MS", 750)?,
    })
}

/// Refuse les combinaisons de configuration qui ouvriraient l'app en production.
pub(super) fn validate_runtime_auth_policy(
    runtime: &RuntimeConfig,
    auth: &AuthConfig,
) -> Result<(), AppError> {
    if runtime.server_env.is_production() && !auth.enabled {
        return Err(AppError::Config(
            "NEWS_AUTH_ENABLED must be true when NEWS_SERVER_ENV=production".to_owned(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn runtime_for(server_env: ServerEnvironment) -> RuntimeConfig {
        RuntimeConfig {
            server_env,
            listen_addr: "127.0.0.1:0".to_owned(),
            ingest_buffer: 16,
            broadcast_buffer: 16,
            dedup_ttl_secs: 300,
            slow_request_threshold_ms: 750,
        }
    }

    fn auth_config(enabled: bool) -> AuthConfig {
        AuthConfig {
            enabled,
            provider: "better_auth".to_owned(),
            jwks_url: None,
            jwt_issuer: None,
            jwt_audience: "https://api.filfinance.test".to_owned(),
            moderator_roles: vec!["moderator".to_owned(), "admin".to_owned()],
        }
    }

    #[test]
    fn parses_supported_server_environments() {
        assert_eq!(
            ServerEnvironment::parse("production").unwrap(),
            ServerEnvironment::Production
        );
        assert_eq!(
            ServerEnvironment::parse("prod").unwrap(),
            ServerEnvironment::Production
        );
        assert_eq!(
            ServerEnvironment::parse("local").unwrap(),
            ServerEnvironment::Local
        );
        assert_eq!(
            ServerEnvironment::parse("development").unwrap(),
            ServerEnvironment::Local
        );
    }

    #[test]
    fn rejects_unknown_server_environment() {
        assert!(ServerEnvironment::parse("staging").is_err());
    }

    #[test]
    fn refuses_disabled_auth_in_production() {
        let result = validate_runtime_auth_policy(
            &runtime_for(ServerEnvironment::Production),
            &auth_config(false),
        );

        assert!(result.is_err());
    }

    #[test]
    fn accepts_enabled_auth_in_production() {
        let result = validate_runtime_auth_policy(
            &runtime_for(ServerEnvironment::Production),
            &auth_config(true),
        );

        assert!(result.is_ok());
    }

    #[test]
    fn accepts_disabled_auth_outside_production() {
        let result = validate_runtime_auth_policy(
            &runtime_for(ServerEnvironment::Local),
            &auth_config(false),
        );

        assert!(result.is_ok());
    }
}
