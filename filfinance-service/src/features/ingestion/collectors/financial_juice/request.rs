//! Helpers HTTP propres au collecteur FinancialJuice.

use reqwest::{Response, StatusCode, Url, header::HeaderMap};
use url::form_urlencoded;

use crate::errors::AppError;

const CLOUDFLARE_MITIGATION_HEADER: &str = "cf-mitigated";
const CLOUDFLARE_CHALLENGE_VALUE: &str = "challenge";
const FINANCIAL_JUICE_CHALLENGE_MESSAGE: &str =
    "FinancialJuice returned Cloudflare managed challenge (403); browser verification is required";
const FINANCIAL_JUICE_RATE_LIMIT_MESSAGE_PREFIX: &str =
    "FinancialJuice returned HTTP 429 Too Many Requests";

/// Construit une URL avec query string encodée explicitement.
pub(super) fn build_url_with_query(base: &str, params: &[(&str, String)]) -> Result<Url, AppError> {
    let mut url = Url::parse(base)
        .map_err(|error| AppError::fetch("financial_juice".to_owned(), error.to_string()))?;
    {
        // On limite la portée de `pairs` dans ce bloc pour libérer clairement l'emprunt mutable sur `url`.
        let mut pairs = url.query_pairs_mut();
        for (key, value) in params {
            pairs.append_pair(key, value);
        }
    }
    Ok(url)
}

/// Encode un corps `application/x-www-form-urlencoded` de façon déterministe.
pub(super) fn encode_form_body(params: &[(&str, String)]) -> String {
    // Le serializer encodera correctement les caractères spéciaux WebForms sans concaténation fragile.
    let mut serializer = form_urlencoded::Serializer::new(String::new());
    for (key, value) in params {
        serializer.append_pair(key, value);
    }
    serializer.finish()
}

/// Convertit un statut HTTP FinancialJuice en erreur source avec un diagnostic exploitable.
pub(super) fn ensure_success_response(
    source_name: &str,
    response: Response,
) -> Result<Response, AppError> {
    // Cloudflare expose ce header uniquement quand la page est une challenge HTML intermediaire.
    if response.status() == StatusCode::FORBIDDEN
        && is_cloudflare_managed_challenge(response.headers())
    {
        return Err(AppError::fetch(
            source_name.to_owned(),
            FINANCIAL_JUICE_CHALLENGE_MESSAGE,
        ));
    }
    if response.status() == StatusCode::TOO_MANY_REQUESTS {
        return Err(AppError::fetch(
            source_name.to_owned(),
            format!(
                "{} for url ({})",
                FINANCIAL_JUICE_RATE_LIMIT_MESSAGE_PREFIX,
                response.url()
            ),
        ));
    }

    response
        .error_for_status()
        .map_err(|error| AppError::fetch(source_name.to_owned(), error.to_string()))
}

/// Indique si l'erreur remontee correspond a la challenge Cloudflare FinancialJuice.
pub(super) fn is_financial_juice_access_challenge(error: &AppError) -> bool {
    // On garde ce test textuel local au collecteur pour ne pas polluer l'enum globale d'erreurs.
    matches!(
        error,
        AppError::Fetch { message, .. }
            if message.contains(FINANCIAL_JUICE_CHALLENGE_MESSAGE)
    )
}

/// Indique si l'erreur remontee correspond a un rate-limit HTTP FinancialJuice.
pub(super) fn is_financial_juice_rate_limited(error: &AppError) -> bool {
    matches!(
        error,
        AppError::Fetch { message, .. }
            if message.contains(FINANCIAL_JUICE_RATE_LIMIT_MESSAGE_PREFIX)
                || message.contains("429 Too Many Requests")
    )
}

fn is_cloudflare_managed_challenge(headers: &HeaderMap) -> bool {
    // `to_str` peut echouer sur un header invalide; dans ce cas ce n'est pas notre signal attendu.
    headers
        .get(CLOUDFLARE_MITIGATION_HEADER)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.eq_ignore_ascii_case(CLOUDFLARE_CHALLENGE_VALUE))
}
