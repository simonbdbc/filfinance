//! Verification cryptographique des JWT d'acces Better Auth.

// `jsonwebtoken` effectue la verification cryptographique et la validation standard des claims.
use jsonwebtoken::{DecodingKey, decode, decode_header};

// La configuration auth porte le JWKS Better Auth et l'audience attendue.
use crate::config::AuthConfig;
// Le domaine auth fournit l'identite minimale reconstruite apres verification JWT.
use crate::features::auth::domain::AuthenticatedIdentity;
// La taxonomie d'erreur commune garde le mapping HTTP stable.
use crate::errors::AppError;

use super::{
    cache::load_jwks,
    claims::BetterAuthClaims,
    validation::{build_validation, ensure_jwk_can_verify, is_asymmetric_algorithm},
};

/// Verifie un JWT Better Auth et reconstruit le contexte utilisateur minimal.
pub(in crate::features::auth) async fn verify_better_auth_jwt(
    config: &AuthConfig,
    token: &str,
) -> Result<AuthenticatedIdentity, AppError> {
    // Cette fonction reste defensive si elle est appelee directement dans un test avec auth off.
    if !config.enabled {
        return Ok(AuthenticatedIdentity::development());
    }

    // Le header est decode sans faire confiance aux claims; il sert seulement a choisir la cle publique.
    let header =
        decode_header(token).map_err(|_| AppError::unauthorized("invalid bearer token header"))?;
    // Le middleware JWKS repose sur des cles publiques: les tokens HMAC ne sont pas acceptes ici.
    if !is_asymmetric_algorithm(header.alg) {
        return Err(AppError::unauthorized(
            "bearer token uses an unsupported signing algorithm",
        ));
    }

    // Better Auth met `kid` dans les JWT signes; il permet de selectionner la bonne JWK.
    let key_id = header
        .kid
        .as_deref()
        .ok_or_else(|| AppError::unauthorized("bearer token does not include a key id"))?;
    // Le JWKS est charge et cache avant la verification cryptographique.
    let jwks = load_jwks(config, false).await?;
    // Un `kid` inconnu peut signaler une rotation; on rafraichit une fois avant de refuser le token.
    let refreshed_jwks;
    let jwk = if let Some(jwk) = jwks.find(key_id) {
        jwk
    } else {
        refreshed_jwks = load_jwks(config, true).await?;
        refreshed_jwks
            .find(key_id)
            .ok_or_else(|| AppError::unauthorized("bearer token key id is not trusted"))?
    };
    // `key_ops` et `use` sont optionnels dans le standard, mais s'ils existent ils doivent autoriser la verification.
    ensure_jwk_can_verify(jwk)?;
    // `DecodingKey::from_jwk` convertit les composants publics RSA/EC/OKP dans le format du verifier.
    let decoding_key = DecodingKey::from_jwk(jwk)
        .map_err(|_| AppError::unauthorized("bearer token key cannot be decoded"))?;
    // La validation ajoute les contraintes propres au service Better Auth attendu.
    let validation = build_validation(config, header.alg)?;
    // `decode` verifie la signature puis applique expiration, audience, issuer et claims requis.
    let token_data = decode::<BetterAuthClaims>(token, &decoding_key, &validation)
        .map_err(|_| AppError::unauthorized("bearer token failed validation"))?;

    // Les claims verifies portent l'identite, la verification email et le role Better Auth.
    identity_from_claims(token_data.claims)
}

/// Construit l'identite uniquement pour un compte dont Better Auth a verifie l'adresse.
fn identity_from_claims(claims: BetterAuthClaims) -> Result<AuthenticatedIdentity, AppError> {
    if !claims.email_verified {
        return Err(AppError::unauthorized(
            "bearer token belongs to an unverified email",
        ));
    }

    Ok(AuthenticatedIdentity {
        id: claims.sub,
        email: Some(claims.email),
        role: claims.role,
        access_token_expires_at: Some(claims.exp),
    })
}

#[cfg(test)]
mod tests {
    use super::{BetterAuthClaims, identity_from_claims};

    #[test]
    fn identity_rejects_unverified_email_claim() {
        let error = identity_from_claims(BetterAuthClaims {
            exp: 1_800_000_000,
            sub: "user-1".to_owned(),
            email: "listener@example.test".to_owned(),
            email_verified: false,
            role: "listener".to_owned(),
        })
        .expect_err("unverified Better Auth account must be rejected");

        assert!(error.to_string().contains("unverified email"));
    }

    #[test]
    fn identity_propagates_verified_token_expiration() {
        let identity = identity_from_claims(BetterAuthClaims {
            exp: 1_800_000_000,
            sub: "user-1".to_owned(),
            email: "listener@example.test".to_owned(),
            email_verified: true,
            role: "listener".to_owned(),
        })
        .expect("verified Better Auth account should be accepted");

        assert_eq!(identity.access_token_expires_at, Some(1_800_000_000));
    }
}
