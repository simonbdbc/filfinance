//! Messages du WebSocket backend user-facing.

use serde::Serialize;

use crate::{
    features::{calendar_day::PublicCalendarDaySummaryStatus, news::domain::PublicSourceStatus},
    shared::time::now_rfc3339,
};

use super::super::LiveNewsItem;
use super::payloads::{WsHeartbeatPayload, WsHelloPayload, WsLagPayload};

/// Contrat de sortie du flux WebSocket serveur.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsServerMessage {
    // Message d'ouverture du flux texte.
    Hello { payload: WsHelloPayload },
    // Heartbeat applicatif pour garder la connexion vivante.
    Heartbeat { payload: WsHeartbeatPayload },
    // Item EN+FR user-facing pret a afficher/lire.
    TranslatedNewsItem { payload: Box<LiveNewsItem> },
    // Signal de lag quand un client a perdu des messages.
    Lagged { payload: WsLagPayload },
}

impl WsServerMessage {
    /// Construit le message d'accueil qui formalise le contrat du flux.
    pub fn hello(
        broadcast_buffer: usize,
        sources: Vec<PublicSourceStatus>,
        calendar_day_summary: PublicCalendarDaySummaryStatus,
    ) -> Self {
        Self::Hello {
            payload: WsHelloPayload {
                // La version de protocole permet de faire evoluer le contrat sans ambiguite.
                protocol_version: 1,
                // Le temps serveur aide au debug des decalages et reconnections.
                server_time: now_rfc3339(),
                broadcast_buffer,
                sources,
                calendar_day_summary: Some(calendar_day_summary),
            },
        }
    }

    /// Construit un heartbeat applicatif pour limiter les expirations de sockets inactives.
    pub fn heartbeat(
        sources: Vec<PublicSourceStatus>,
        calendar_day_summary: PublicCalendarDaySummaryStatus,
    ) -> Self {
        Self::Heartbeat {
            payload: WsHeartbeatPayload {
                server_time: now_rfc3339(),
                sources,
                calendar_day_summary: Some(calendar_day_summary),
            },
        }
    }

    /// Construit le message temps reel derive contenant le texte FR pret a lire.
    pub fn translated_news_item(payload: LiveNewsItem) -> Self {
        Self::TranslatedNewsItem {
            // `Box<T>` place l'objet sur le tas et garde l'enum plus compacte a manipuler.
            payload: Box::new(payload),
        }
    }

    /// Signale qu'un client lent a perdu une partie du flux temps reel.
    pub fn lagged(skipped: u64) -> Self {
        Self::Lagged {
            payload: WsLagPayload { skipped },
        }
    }
}
