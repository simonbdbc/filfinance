//! Configuration JWT Better Auth et validation des contraintes de securite.

// Les helpers d'environnement gardent le parsing NEWS_* uniforme dans tous les sous-modules.
use super::{
    AuthConfig,
    environment::{env_bool, env_csv, env_optional, env_or},
};

// Les erreurs de validation doivent remonter au bootstrap applicatif.
use crate::errors::AppError;

impl AuthConfig {
    /// Construit la configuration Better Auth depuis l'environnement.
    pub fn from_env() -> Result<Self, AppError> {
        // Le flag reste desactive par defaut pour ne pas casser le bootstrap local tant que
        // l'environnement Better Auth n'a pas ete configure sur chaque machine de dev.
        let enabled = env_bool("NEWS_AUTH_ENABLED", false)?;
        // Le provider est explicite pour refuser une ancienne configuration auth active.
        let provider = env_or("NEWS_AUTH_PROVIDER", "better_auth");
        // Le JWKS Better Auth est expose sous `/api/auth/jwks`.
        let jwks_url = env_optional("NEWS_AUTH_JWKS_URL");
        // L'issuer correspond au `BETTER_AUTH_URL` public.
        let jwt_issuer = env_optional("NEWS_AUTH_ISSUER");
        // L'audience identifie l'API Fil Finance consommee par le client.
        let jwt_audience = env_or("NEWS_AUTH_AUDIENCE", "");
        // Les roles moderateur sont configurables pour garder une marge d'exploitation.
        let moderator_roles = {
            // `env_csv` ignore les valeurs vides et trim chaque entree.
            let configured_roles = env_csv("NEWS_AUTH_MODERATOR_ROLES");
            // Sans configuration, `moderator` et `admin` couvrent le cadrage de l'EPIC.
            if configured_roles.is_empty() {
                vec!["moderator".to_owned(), "admin".to_owned()]
            } else {
                configured_roles
            }
        };
        // On construit la struct avant validation pour garder une seule fonction de regles.
        let config = Self {
            enabled,
            provider,
            jwks_url,
            jwt_issuer,
            jwt_audience,
            moderator_roles,
        };
        // La validation explicite evite un serveur demarre avec auth active mais inverifiable.
        validate_auth_config(&config)?;
        // La configuration valide peut etre retournee au bootstrap.
        Ok(config)
    }

    /// Retourne une configuration desactivee pratique pour les tests unitaires.
    #[cfg(test)]
    pub fn disabled_for_tests() -> Self {
        // Cette fixture encode clairement que les tests existants ne couvrent pas l'auth.
        Self {
            enabled: false,
            provider: "better_auth".to_owned(),
            jwks_url: None,
            jwt_issuer: None,
            jwt_audience: String::new(),
            moderator_roles: vec!["moderator".to_owned(), "admin".to_owned()],
        }
    }
}

/// Valide les champs auth seulement quand l'auth est activee.
fn validate_auth_config(config: &AuthConfig) -> Result<(), AppError> {
    // Auth desactivee signifie que le backend garde le comportement MVP ouvert.
    if !config.enabled {
        return Ok(());
    }

    // Le backend ne sait verifier que les JWT emis par le service Better Auth Fil Finance.
    if config.provider.trim() != "better_auth" {
        return Err(AppError::Config(
            "NEWS_AUTH_PROVIDER must be 'better_auth' when NEWS_AUTH_ENABLED=true".to_owned(),
        ));
    }

    // Le JWKS est indispensable pour verifier les signatures asymetriques Better Auth.
    if config
        .jwks_url
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_none()
    {
        return Err(AppError::Config(
            "NEWS_AUTH_JWKS_URL is required when NEWS_AUTH_ENABLED=true".to_owned(),
        ));
    }

    // L'issuer permet de refuser un token emis par un autre service.
    if config
        .jwt_issuer
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_none()
    {
        return Err(AppError::Config(
            "NEWS_AUTH_ISSUER is required when NEWS_AUTH_ENABLED=true".to_owned(),
        ));
    }

    // Une audience vide rendrait la validation JWT ambigue.
    if config.jwt_audience.trim().is_empty() {
        return Err(AppError::Config(
            "NEWS_AUTH_AUDIENCE cannot be empty when NEWS_AUTH_ENABLED=true".to_owned(),
        ));
    }

    // Sans role moderateur, aucune surface sensible ne pourrait etre autorisee proprement.
    if config.moderator_roles.is_empty() {
        return Err(AppError::Config(
            "NEWS_AUTH_MODERATOR_ROLES cannot be empty when NEWS_AUTH_ENABLED=true".to_owned(),
        ));
    }

    // Tous les controles sont passes.
    Ok(())
}
