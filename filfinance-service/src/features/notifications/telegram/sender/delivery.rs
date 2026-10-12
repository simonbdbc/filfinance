//! Envoi d'une traduction vers les chats Telegram.

use reqwest::Client;
use tokio::time::Instant;

use crate::{
    app_state::AppState,
    errors::AppError,
    features::{live::dto::LiveNewsItem, translation::domain::PersistedNewsTranslation},
};

use super::super::{
    build_telegram_message,
    client::{send_to_chat_with_retry, telegram_send_message_endpoint},
};
use super::trace::{
    emit_telegram_send_failed, emit_telegram_send_started, emit_telegram_send_succeeded,
};

/// Formate puis envoie un evenement vers tous les chats Telegram cibles.
pub(super) async fn send_news_item(
    client: &Client,
    state: &AppState,
    item: &PersistedNewsTranslation,
    last_http_send_at: &mut Option<Instant>,
) -> Result<(), AppError> {
    // Si le token est absent alors que le worker a quand meme ete lance, on no-op proprement.
    let token = match &state.config.telegram.bot_token {
        Some(token) => token,
        None => return Ok(()),
    };
    // L'endpoint est derive ici pour garder la possibilite de pointer vers un faux serveur en test.
    let endpoint = telegram_send_message_endpoint(state, token);
    let source_event = state.news.get_news_by_id(item.news_event_id).await?;
    emit_telegram_send_started(state, source_event.as_ref());

    let live_item = LiveNewsItem::from_translation(
        item.clone(),
        source_event.as_ref(),
        state.config.translation.max_body_chars,
    );
    // Telegram et le WebSocket partagent exactement le meme `LiveNewsItem`, ce qui reduit le drift.
    let text = build_telegram_message(&live_item);

    // Chaque chat est traite independamment pour garder une diffusion simple et observable.
    for chat_id in &state.config.telegram.chat_ids {
        if let Err(error) =
            send_to_chat_with_retry(client, state, &endpoint, chat_id, &text, last_http_send_at)
                .await
        {
            emit_telegram_send_failed(state, source_event.as_ref(), chat_id, &error);
            return Err(error);
        }
    }

    emit_telegram_send_succeeded(state, source_event.as_ref());
    Ok(())
}
