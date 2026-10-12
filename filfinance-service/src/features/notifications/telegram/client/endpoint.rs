//! Construction des endpoints Telegram.

// L'etat applicatif porte la configuration Telegram deja chargee.
use crate::app_state::AppState;

/// Construit le point d'entree `sendMessage` a partir de la base Telegram configurable.
pub(in crate::features::notifications::telegram) fn telegram_send_message_endpoint(
    state: &AppState,
    token: &str,
) -> String {
    // `trim_end_matches('/')` evite les doubles slash si la base URL est configuree avec un suffixe `/`.
    format!(
        "{}/bot{token}/sendMessage",
        state.config.telegram.api_base_url.trim_end_matches('/')
    )
}
