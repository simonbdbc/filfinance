//! Erreurs intermediaires du client Telegram.

/// Erreur intermediaire servant a distinguer un retry utile d'un echec permanent.
pub(in crate::features::notifications) enum TelegramSendError {
    // On garde un message humain et un `retry_after_ms` optionnel pour reutiliser le meme type
    // aussi bien avec des erreurs reseau qu'avec des 429 Telegram.
    Retriable {
        message: String,
        retry_after_ms: Option<u64>,
    },
    Permanent(String),
}
