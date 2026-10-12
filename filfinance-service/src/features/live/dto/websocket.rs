//! DTO des WebSockets backend texte.

mod payloads;
mod server;
mod test_trace;

// La facade conserve les exports historiques du contrat WebSocket backend.
pub use payloads::{
    TestRunAcceptedResponse, TestTraceEvent, WsErrorPayload, WsHeartbeatPayload, WsHelloPayload,
    WsLagPayload, WsTestTraceHelloPayload,
};
pub use server::WsServerMessage;
pub use test_trace::WsTestTraceMessage;
