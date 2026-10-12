//! Tickets courts pour autoriser le WebSocket metadata edge.

mod claims;
mod handler;
mod signing;

pub use handler::create_edge_live_ticket;
pub use signing::{EdgeLiveTicketResponse, create_live_ticket, verify_live_ticket};
