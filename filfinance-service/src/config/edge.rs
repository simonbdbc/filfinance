//! Configuration du publisher live edge et des tickets courts.

// Les helpers d'environnement gardent le parsing NEWS_* uniforme dans tous les sous-modules.
use super::{
    EdgeConfig,
    environment::{env_bool, env_optional, env_u64, env_usize},
};

// Les erreurs de validation doivent remonter au bootstrap applicatif.
use crate::errors::AppError;

impl EdgeConfig {
    /// Construit la configuration du publisher edge depuis l'environnement.
    pub fn from_env() -> Result<Self, AppError> {
        // Le flag reste false tant que `NEWS-096` n'a pas bascule le client sur l'edge.
        let publish_enabled = env_bool("NEWS_EDGE_PUBLISH_ENABLED", false)?;
        // URL et secret sont optionnels en config globale, puis obligatoires seulement si active.
        let publish_url = env_optional("NEWS_EDGE_PUBLISH_URL");
        let publish_secret = env_optional("NEWS_EDGE_PUBLISH_SECRET");
        // Les timings sont volontairement courts: le Worker edge ne doit pas tenir le hot path.
        let publish_timeout_ms = env_u64("NEWS_EDGE_PUBLISH_TIMEOUT_MS", 1_500)?;
        let publish_retry_attempts = env_usize("NEWS_EDGE_PUBLISH_RETRY_ATTEMPTS", 2)?;
        let publish_retry_backoff_ms = env_u64("NEWS_EDGE_PUBLISH_RETRY_BACKOFF_MS", 250)?;
        // Les tickets edge sont actives separement pour permettre une migration progressive.
        let ticket_enabled = env_bool("NEWS_EDGE_TICKET_ENABLED", false)?;
        let ticket_secret = env_optional("NEWS_EDGE_TICKET_SECRET");
        // Une minute suffit: le ticket sert seulement a l'ouverture WebSocket.
        let ticket_ttl_secs = env_u64("NEWS_EDGE_TICKET_TTL_SECS", 60)?;
        // On construit la struct avant validation pour garder les regles dans un helper dedie.
        let config = Self {
            publish_enabled,
            publish_url,
            publish_secret,
            publish_timeout_ms,
            publish_retry_attempts,
            publish_retry_backoff_ms,
            ticket_enabled,
            ticket_secret,
            ticket_ttl_secs,
        };
        // Toute incoherence doit faire echouer le demarrage si le downstream est active.
        validate_edge_config(&config)?;
        // Configuration valide et prete pour le worker de publication.
        Ok(config)
    }

    /// Retourne une configuration edge desactivee pratique pour les tests unitaires.
    #[cfg(test)]
    pub fn disabled_for_tests() -> Self {
        // Les tests historiques n'ont pas besoin d'un Worker Cloudflare pour construire `AppState`.
        Self {
            publish_enabled: false,
            publish_url: None,
            publish_secret: None,
            publish_timeout_ms: 1_500,
            publish_retry_attempts: 0,
            publish_retry_backoff_ms: 250,
            ticket_enabled: false,
            ticket_secret: None,
            ticket_ttl_secs: 60,
        }
    }
}

/// Valide les contraintes propres au publisher metadata edge.
fn validate_edge_config(config: &EdgeConfig) -> Result<(), AppError> {
    // Le publisher sortant est optionnel, mais s'il est actif ses champs reseau deviennent stricts.
    if config.publish_enabled {
        // Une URL vide rendrait le downstream impossible a joindre.
        let publish_url = config
            .publish_url
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                AppError::Config(
                    "NEWS_EDGE_PUBLISH_URL is required when NEWS_EDGE_PUBLISH_ENABLED=true"
                        .to_owned(),
                )
            })?;

        // `Url::parse` attrape les fautes de scheme/host au demarrage plutot qu'au premier live.
        url::Url::parse(publish_url)
            .map_err(|error| AppError::Config(format!("invalid NEWS_EDGE_PUBLISH_URL: {error}")))?;

        // Le secret HMAC est la protection serveur->Worker du endpoint `/publish/live`.
        let publish_secret = config
            .publish_secret
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                AppError::Config(
                    "NEWS_EDGE_PUBLISH_SECRET is required when NEWS_EDGE_PUBLISH_ENABLED=true"
                        .to_owned(),
                )
            })?;

        if publish_secret.len() < 32 {
            return Err(AppError::Config(
                "NEWS_EDGE_PUBLISH_SECRET must contain at least 32 characters".to_owned(),
            ));
        }

        // Un timeout nul desactiverait pratiquement toutes les tentatives HTTP.
        if config.publish_timeout_ms == 0 {
            return Err(AppError::Config(
                "NEWS_EDGE_PUBLISH_TIMEOUT_MS must be greater than 0".to_owned(),
            ));
        }

        // Si des retries sont configures, le backoff doit etre non nul pour eviter une boucle serree.
        if config.publish_retry_attempts > 0 && config.publish_retry_backoff_ms == 0 {
            return Err(AppError::Config(
                "NEWS_EDGE_PUBLISH_RETRY_BACKOFF_MS must be greater than 0 when retries are enabled"
                    .to_owned(),
            ));
        }
    }

    // Les tickets courts deviennent obligatoires uniquement quand l'endpoint est active.
    if config.ticket_enabled {
        let ticket_secret = config
            .ticket_secret
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                AppError::Config(
                    "NEWS_EDGE_TICKET_SECRET is required when NEWS_EDGE_TICKET_ENABLED=true"
                        .to_owned(),
                )
            })?;

        // Une longueur minimale evite de signer les tickets avec un placeholder trivial.
        if ticket_secret.len() < 32 {
            return Err(AppError::Config(
                "NEWS_EDGE_TICKET_SECRET must contain at least 32 characters".to_owned(),
            ));
        }

        // TTL borne: trop court casse les connexions lentes, trop long augmente la fenetre de fuite.
        if config.ticket_ttl_secs == 0 || config.ticket_ttl_secs > 300 {
            return Err(AppError::Config(
                "NEWS_EDGE_TICKET_TTL_SECS must be between 1 and 300".to_owned(),
            ));
        }
    }

    // Tous les controles du downstream edge sont passes.
    Ok(())
}
