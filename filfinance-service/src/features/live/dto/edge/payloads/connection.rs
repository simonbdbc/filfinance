//! Payloads de connexion et supervision du transport edge live.

use serde::{Deserialize, Serialize};

use super::super::state::EdgeLiveSnapshot;

/// Message initial du WebSocket edge metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeLiveHelloPayload {
    /// Version du contrat edge pour permettre une migration progressive.
    pub protocol_version: u8,
    /// Date serveur d'émission du message.
    pub server_time: String,
    /// Snapshot minimal utile après reconnexion.
    pub snapshot: EdgeLiveSnapshot,
}

/// Heartbeat léger du WebSocket edge metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeLiveHeartbeatPayload {
    /// Date serveur d'émission du heartbeat.
    pub server_time: String,
}

/// Signal de lag quand un client edge a raté des événements.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeLiveLagPayload {
    /// Date serveur d'émission du signal.
    pub server_time: String,
    /// Nombre d'événements sautés.
    pub skipped: u64,
}
