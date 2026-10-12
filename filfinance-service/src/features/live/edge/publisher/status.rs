//! Classification des statuts HTTP retournes par le Worker edge.

use reqwest::StatusCode;

/// Classe les statuts HTTP pour savoir si un retry court peut resoudre l'echec.
pub(super) fn is_retriable_status(status: StatusCode) -> bool {
    // 408/429 et 5xx sont typiquement temporaires; les autres 4xx signalent une config ou auth invalide.
    status == StatusCode::REQUEST_TIMEOUT
        || status == StatusCode::TOO_MANY_REQUESTS
        || status.is_server_error()
}
