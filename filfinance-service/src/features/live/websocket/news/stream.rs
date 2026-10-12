//! Facade du flux WebSocket user-facing des news traduites.

mod broadcasts;
mod connection;
mod heartbeat;
mod input;
mod lag;

pub(in crate::features::live::websocket) use connection::handle_news_socket;
