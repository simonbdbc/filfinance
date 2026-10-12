//! Facade du client HTTP Telegram.

mod endpoint;
mod error;
mod request;
mod retry;

// Le sender conserve ses imports historiques via `telegram::client`.
pub(super) use endpoint::telegram_send_message_endpoint;
pub(super) use retry::send_to_chat_with_retry;
// Les tests de notification injectent la politique de retry sans faire d'appel HTTP reel.
pub(in crate::features::notifications) use error::TelegramSendError;
pub(in crate::features::notifications) use retry::execute_send_with_retry;
