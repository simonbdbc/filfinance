//! Payloads edge pour l'etat public des sources.

use serde::{Deserialize, Serialize};

use crate::features::{
    calendar_day::PublicCalendarDaySummaryStatus, news::domain::PublicSourceStatus,
};

/// Payload edge de mise a jour des badges publics de sources.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeSourceStatusPayload {
    /// Identifiant opaque et monotone fourni par le backend publisher.
    pub event_id: String,
    /// Date serveur d'emission.
    pub server_time: String,
    /// Etat public compact des sources.
    pub sources: Vec<PublicSourceStatus>,
    /// Statut public compact du résumé calendrier du jour.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calendar_day_summary: Option<PublicCalendarDaySummaryStatus>,
}
