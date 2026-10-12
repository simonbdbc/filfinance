//! Tests de contrats auth et middleware HTTP.

// `HeaderMap`, `HeaderValue` et `header` construisent les memes headers que les clients HTTP.
use axum::http::{HeaderMap, HeaderValue, header};

use crate::{
    config::AuthConfig,
    features::auth::{domain::CurrentUser, middleware::bearer_token_from_headers},
};

#[test]
fn current_user_moderator_access_uses_configured_roles() {
    // Les roles moderateur restent configurables autour des roles signes par Better Auth.
    let config = AuthConfig {
        enabled: true,
        provider: "better_auth".to_owned(),
        jwks_url: Some("https://api.filfinance.test/api/auth/jwks".to_owned()),
        jwt_issuer: Some("https://api.filfinance.test".to_owned()),
        jwt_audience: "https://api.filfinance.test".to_owned(),
        moderator_roles: vec!["moderator".to_owned(), "admin".to_owned()],
    };
    // La fixture represente un utilisateur deja extrait d'un JWT valide.
    let user = CurrentUser {
        id: "user-1".to_owned(),
        email: None,
        role: "moderator".to_owned(),
        access_token_expires_at: Some(1_800_000_000),
    };

    assert!(user.can_access_moderator_routes(&config));
}

#[test]
fn current_user_admin_access_requires_admin_role() {
    let admin = CurrentUser {
        id: "admin-1".to_owned(),
        email: None,
        role: "admin".to_owned(),
        access_token_expires_at: Some(1_800_000_000),
    };
    let moderator = CurrentUser {
        id: "moderator-1".to_owned(),
        email: None,
        role: "moderator".to_owned(),
        access_token_expires_at: Some(1_800_000_000),
    };

    assert!(admin.can_access_admin_routes());
    assert!(!moderator.can_access_admin_routes());
}

#[test]
fn current_user_from_identity_uses_signed_better_auth_role() {
    let identity = crate::features::auth::domain::AuthenticatedIdentity {
        id: "user-1".to_owned(),
        email: Some("moderator@example.test".to_owned()),
        role: "moderator".to_owned(),
        access_token_expires_at: Some(1_800_000_000),
    };

    let user = CurrentUser::from_identity(&identity);

    assert_eq!(user.id, "user-1");
    assert_eq!(user.role, "moderator");
    assert_eq!(user.email.as_deref(), Some("moderator@example.test"));
    assert_eq!(user.access_token_expires_at, Some(1_800_000_000));
}

#[test]
fn bearer_token_from_headers_reads_standard_authorization_header() {
    // Cette assertion verrouille le contrat client: `Authorization: Bearer <jwt>`.
    let mut headers = HeaderMap::new();
    headers.insert(
        header::AUTHORIZATION,
        HeaderValue::from_static("Bearer token-value"),
    );

    let token = bearer_token_from_headers(&headers).expect("bearer token should be parsed");

    assert_eq!(token, "token-value");
}

#[test]
fn bearer_token_from_headers_rejects_missing_header() {
    // Sans header, le middleware doit produire une vraie erreur d'auth et non une 500.
    let error = bearer_token_from_headers(&HeaderMap::new())
        .expect_err("missing bearer should be unauthorized");

    assert!(
        error.to_string().contains("missing Authorization"),
        "the error should explain the missing bearer contract"
    );
}
