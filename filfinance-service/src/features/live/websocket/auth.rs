//! Facade du handshake d'authentification des WebSockets proteges.

mod close;
mod command;
mod handshake;
mod reader;

pub(in crate::features::live) use command::parse_websocket_auth_token;
pub(super) use handshake::authenticate_socket;
