//! Facade du replay et de l'émission des items WebSocket user-facing.

// `emit` porte l'envoi socket, `payload` l'enrichissement, `replay` le backlog reconnect.
mod emit;
mod payload;
mod replay;

pub(super) use emit::emit_live_item;
pub(super) use payload::build_live_news_item;
pub(super) use replay::replay_recent_translations;
