//! Payloads edge pour les notices systeme non metier.

use serde::{Deserialize, Serialize};

use super::super::state::EdgeSystemNoticeSeverity;

/// Payload edge pour les notifications système légères.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeSystemNoticePayload {
    /// Identifiant opaque et monotone fourni par le backend publisher.
    pub event_id: String,
    /// Date serveur d'émission.
    pub server_time: String,
    /// Sévérité d'affichage.
    pub severity: EdgeSystemNoticeSeverity,
    /// Message court pour le client.
    pub message: String,
    /// Code stable optionnel pour filtrer ou grouper les notices.
    pub code: Option<String>,
    /// Détail optionnel réservé au debug lisible.
    pub detail: Option<String>,
}
