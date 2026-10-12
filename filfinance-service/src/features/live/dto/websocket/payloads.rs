//! Payloads JSON partagés par les WebSockets backend.

use serde::{Deserialize, Serialize};

use crate::features::{
    calendar_day::PublicCalendarDaySummaryStatus, news::domain::PublicSourceStatus,
};

/// Accuse de reception renvoye au client quand un run de test est accepte.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestRunAcceptedResponse {
    /// Identifiant du run E2E accepte.
    pub run_id: String,
    /// Date d'acceptation du run.
    pub accepted_at: String,
}

/// Evenement de trace dedie au suivi d'un run de test de bout en bout.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestTraceEvent {
    /// Identifiant du run auquel la trace appartient.
    pub run_id: String,
    /// Etape technique ou metier du pipeline.
    pub stage: String,
    /// Statut de l'etape (`info`, `ok`, `warning`, `error`...).
    pub status: String,
    /// Message humain principal de la trace.
    pub message: String,
    /// Detail optionnel utile au debug.
    pub detail: Option<String>,
    /// Date d'emission de la trace.
    pub timestamp: String,
}

/// Message initial envoye a un client WebSocket nouvellement connecte.
#[derive(Debug, Clone, Serialize)]
pub struct WsHelloPayload {
    /// Version de protocole du flux WS texte.
    pub protocol_version: u8,
    /// Heure serveur au moment du hello.
    pub server_time: String,
    /// Taille du buffer `broadcast` cote serveur.
    pub broadcast_buffer: usize,
    /// Etat public compact des sources affichable par tous les roles.
    pub sources: Vec<PublicSourceStatus>,
    /// Statut public compact du résumé calendrier du jour.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calendar_day_summary: Option<PublicCalendarDaySummaryStatus>,
}

/// Message de signalement lorsqu'un client WebSocket a rate des evenements.
#[derive(Debug, Clone, Serialize)]
pub struct WsLagPayload {
    /// Nombre d'evenements sautes par un client trop lent.
    pub skipped: u64,
}

/// Message de heartbeat envoye periodiquement pour garder les connexions mobiles actives.
#[derive(Debug, Clone, Serialize)]
pub struct WsHeartbeatPayload {
    /// Heure serveur d'emission du heartbeat.
    pub server_time: String,
    /// Etat public compact des sources affichable par tous les roles.
    pub sources: Vec<PublicSourceStatus>,
    /// Statut public compact du résumé calendrier du jour.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calendar_day_summary: Option<PublicCalendarDaySummaryStatus>,
}

/// Message initial envoye a un client WebSocket de trace nouvellement connecte.
#[derive(Debug, Clone, Serialize)]
pub struct WsTestTraceHelloPayload {
    /// Version de protocole du flux de trace.
    pub protocol_version: u8,
    /// Heure serveur a l'ouverture du socket de debug.
    pub server_time: String,
    /// Taille du bus `broadcast` des traces.
    pub broadcast_buffer: usize,
}

/// Message d'erreur non fatale renvoye au client de trace pour les commandes invalides.
#[derive(Debug, Clone, Serialize)]
pub struct WsErrorPayload {
    /// Message fonctionnel a afficher au client.
    pub message: String,
}
