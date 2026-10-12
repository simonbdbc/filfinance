//! Payloads edge lies aux cartes et items de news.

use serde::{Deserialize, Serialize};

use super::super::state::EdgeNewsCard;
use crate::features::live::dto::LiveNewsItem;

/// Payload edge d'une carte de news.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeNewsCardPayload {
    /// Identifiant opaque et monotone fourni par le backend publisher.
    pub event_id: String,
    /// Date serveur d'émission.
    pub server_time: String,
    /// Carte compacte à diffuser.
    pub card: EdgeNewsCard,
}

/// Payload edge qui réutilise le contrat user-facing `LiveNewsItem`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeTranslatedNewsItemPayload {
    /// Identifiant opaque et monotone fourni par le backend publisher.
    pub event_id: String,
    /// Date serveur d'émission.
    pub server_time: String,
    /// Item EN+FR existant, sans audio binaire.
    pub item: LiveNewsItem,
}
