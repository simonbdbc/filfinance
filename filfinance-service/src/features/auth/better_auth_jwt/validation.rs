//! Regles de validation JWT et JWK propres a Better Auth.

use jsonwebtoken::{
    Algorithm, Validation,
    jwk::{KeyOperations, PublicKeyUse},
};

use crate::{config::AuthConfig, errors::AppError};

/// Construit les regles de validation JWT propres au service Better Auth configure.
pub(super) fn build_validation(
    config: &AuthConfig,
    algorithm: Algorithm,
) -> Result<Validation, AppError> {
    // L'issuer empeche d'accepter un JWT emis par un autre service.
    let issuer = config
        .jwt_issuer
        .as_deref()
        .ok_or_else(|| AppError::Config("Better Auth JWT issuer is missing".to_owned()))?;
    // `Validation::new` verrouille deja l'algorithme attendu sur celui du header decode.
    let mut validation = Validation::new(algorithm);
    // Un token expire ne doit beneficier d'aucune grace: les WebSockets ferment exactement a `exp`.
    validation.leeway = 0;
    // Refuse aussi la seconde limite ou `exp == now`, avant toute donnee metier au handshake.
    validation.reject_tokens_expiring_in_less_than = 1;
    // On exige les claims standards qui identifient et bornent le JWT.
    validation.set_required_spec_claims(&["exp", "sub", "aud", "iss"]);
    // L'audience attendue identifie l'API Fil Finance consommee par le client.
    validation.set_audience(&[config.jwt_audience.as_str()]);
    // L'issuer attendu est le `BETTER_AUTH_URL` public.
    validation.set_issuer(&[issuer]);

    Ok(validation)
}

/// Refuse les JWK qui annoncent explicitement un usage incompatible avec la verification.
pub(super) fn ensure_jwk_can_verify(jwk: &jsonwebtoken::jwk::Jwk) -> Result<(), AppError> {
    // `use` est optionnel; s'il est present, il doit designer une cle de signature.
    if matches!(
        jwk.common.public_key_use,
        Some(PublicKeyUse::Encryption | PublicKeyUse::Other(_))
    ) {
        return Err(AppError::unauthorized(
            "bearer token key is not intended for signature verification",
        ));
    }

    // `key_ops` est optionnel; s'il est present, l'operation `verify` doit etre autorisee.
    if let Some(operations) = jwk.common.key_operations.as_ref()
        && !operations.contains(&KeyOperations::Verify)
    {
        return Err(AppError::unauthorized(
            "bearer token key does not allow verification",
        ));
    }

    Ok(())
}

/// Indique si l'algorithme repose sur une cle publique utilisable via JWKS.
pub(super) fn is_asymmetric_algorithm(algorithm: Algorithm) -> bool {
    // Le service emet explicitement du RS256; accepter un autre algorithme elargirait inutilement le contrat.
    algorithm == Algorithm::RS256
}

#[cfg(test)]
mod tests {
    use jsonwebtoken::Algorithm;

    use crate::config::AuthConfig;

    use super::build_validation;

    #[test]
    fn jwt_validation_has_no_post_expiration_grace() {
        let config = AuthConfig {
            enabled: true,
            provider: "better_auth".to_owned(),
            jwks_url: Some("https://api.filfinance.test/api/auth/jwks".to_owned()),
            jwt_issuer: Some("https://api.filfinance.test".to_owned()),
            jwt_audience: "https://api.filfinance.test".to_owned(),
            moderator_roles: vec!["moderator".to_owned(), "admin".to_owned()],
        };

        let validation = build_validation(&config, Algorithm::RS256)
            .expect("Better Auth validation should build");

        assert_eq!(validation.leeway, 0);
        assert_eq!(validation.reject_tokens_expiring_in_less_than, 1);
        assert!(validation.required_spec_claims.contains("exp"));
    }
}
