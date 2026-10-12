//! Facade des DTO live user-facing partages par WebSocket, edge, Telegram et HTTP.

mod conversion;
mod history;
mod text;
mod types;

pub use history::LiveNewsHistoryResponse;
pub use types::LiveNewsItem;
