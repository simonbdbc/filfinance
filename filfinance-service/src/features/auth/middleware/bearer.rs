//! Parsing du header HTTP Authorization Bearer.

use axum::http::{HeaderMap, header};

use crate::errors::AppError;

/// Extrait le token Bearer depuis le header HTTP standard.
pub(in crate::features::auth) fn bearer_token_from_headers(
    headers: &HeaderMap,
) -> Result<&str, AppError> {
    // Better Auth et le client Expo utilisent le contrat HTTP standard `Authorization: Bearer <jwt>`.
    let raw_header = headers
        .get(header::AUTHORIZATION)
        .ok_or_else(|| AppError::unauthorized("missing Authorization bearer token"))?
        .to_str()
        .map_err(|_| AppError::unauthorized("invalid Authorization header encoding"))?;

    // `split_once` garde le parsing strict: un header sans scheme ou sans token est invalide.
    let (scheme, token) = raw_header
        .trim()
        .split_once(' ')
        .ok_or_else(|| AppError::unauthorized("invalid Authorization bearer token"))?;
    // Le scheme HTTP est compare sans tenir compte de la casse, comme le permet le standard.
    if !scheme.eq_ignore_ascii_case("bearer") {
        return Err(AppError::unauthorized(
            "Authorization header must use the Bearer scheme",
        ));
    }

    // Le token lui-meme ne doit pas etre vide apres normalisation des espaces.
    let token = token.trim();
    if token.is_empty() {
        return Err(AppError::unauthorized("empty Authorization bearer token"));
    }

    Ok(token)
}
