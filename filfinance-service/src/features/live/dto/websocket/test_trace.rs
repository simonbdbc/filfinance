//! Messages du WebSocket backend dédié aux runs de test.

use serde::Serialize;

use crate::shared::time::now_rfc3339;

use super::payloads::{
    TestRunAcceptedResponse, TestTraceEvent, WsErrorPayload, WsHeartbeatPayload, WsLagPayload,
    WsTestTraceHelloPayload,
};

/// Contrat de sortie du flux WebSocket dedie aux traces de runs de test.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsTestTraceMessage {
    // Message initial du flux de debug.
    Hello { payload: WsTestTraceHelloPayload },
    // Heartbeat du flux de debug.
    Heartbeat { payload: WsHeartbeatPayload },
    // Accuse de reception d'un run synthetique.
    TestRunAccepted { payload: TestRunAcceptedResponse },
    // Trace detaillee de pipeline.
    TestTrace { payload: Box<TestTraceEvent> },
    // Signal de lag du canal de debug.
    Lagged { payload: WsLagPayload },
    // Erreur legere sans fermeture du socket.
    Error { payload: WsErrorPayload },
}

impl WsTestTraceMessage {
    /// Construit le message d'accueil du flux de trace pour annoncer le contrat disponible.
    pub fn hello(broadcast_buffer: usize) -> Self {
        Self::Hello {
            payload: WsTestTraceHelloPayload {
                protocol_version: 1,
                server_time: now_rfc3339(),
                broadcast_buffer,
            },
        }
    }

    /// Construit un heartbeat de trace pour garder le canal de debug actif.
    pub fn heartbeat() -> Self {
        Self::Heartbeat {
            payload: WsHeartbeatPayload {
                server_time: now_rfc3339(),
                sources: Vec::new(),
                calendar_day_summary: None,
            },
        }
    }

    /// Emballe un evenement de trace de test dans le contrat WS dedie.
    pub fn test_trace(payload: TestTraceEvent) -> Self {
        Self::TestTrace {
            // Le `Box` garde la taille de l'enum stable malgre un payload plus riche.
            payload: Box::new(payload),
        }
    }

    /// Confirme au client que le run est accepte avant la premiere trace detaillee.
    pub fn test_run_accepted(payload: TestRunAcceptedResponse) -> Self {
        Self::TestRunAccepted { payload }
    }

    /// Signale un lag sur le canal de trace sans bloquer les autres consommateurs.
    pub fn lagged(skipped: u64) -> Self {
        Self::Lagged {
            payload: WsLagPayload { skipped },
        }
    }

    /// Retourne une erreur fonctionnelle legere sans fermer le socket de debug.
    pub fn error(message: impl Into<String>) -> Self {
        Self::Error {
            payload: WsErrorPayload {
                message: message.into(),
            },
        }
    }
}
