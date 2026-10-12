//! Facade de verification des JWT Better Auth et cache JWKS local.

mod cache;
mod claims;
mod validation;
mod verifier;

pub(super) use verifier::verify_better_auth_jwt;
