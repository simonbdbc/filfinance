//! Traces E2E liées à l'envoi Telegram.

use crate::{app_state::AppState, errors::AppError, features::news::domain::PersistedNewsEvent};

/// Trace le début d'envoi Telegram quand l'item source appartient à un run de test.
pub(super) fn emit_telegram_send_started(
    state: &AppState,
    source_event: Option<&PersistedNewsEvent>,
) {
    if let Some(source_event) = source_event {
        state.emit_test_trace_for_event(
            source_event,
            "telegram_send_started",
            "info",
            "Debut de l'envoi Telegram pour le run de test.",
            Some(format!(
                "chat_count={}",
                state.config.telegram.chat_ids.len()
            )),
        );
    }
}

/// Trace un échec d'envoi Telegram par chat cible.
pub(super) fn emit_telegram_send_failed(
    state: &AppState,
    source_event: Option<&PersistedNewsEvent>,
    chat_id: &str,
    error: &AppError,
) {
    if let Some(source_event) = source_event {
        state.emit_test_trace_for_event(
            source_event,
            "telegram_send_failed",
            "error",
            "L'envoi Telegram a echoue pour au moins un chat cible.",
            Some(format!("chat_id={chat_id} error={error}")),
        );
    }
}

/// Trace le succès Telegram global quand tous les chats cibles ont accepté le message.
pub(super) fn emit_telegram_send_succeeded(
    state: &AppState,
    source_event: Option<&PersistedNewsEvent>,
) {
    if let Some(source_event) = source_event {
        state.emit_test_trace_for_event(
            source_event,
            "telegram_send_succeeded",
            "ok",
            "Le message de test a ete envoye a Telegram.",
            Some(format!(
                "chat_count={}",
                state.config.telegram.chat_ids.len()
            )),
        );
    }
}
