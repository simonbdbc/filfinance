//! Erreurs internes de publication edge.

/// Erreur interne classifiee pour decider si un retry a du sens.
#[derive(Debug)]
pub(super) enum EdgePublishError {
    // Une erreur reseau ou 5xx/429 peut aboutir apres un retry court.
    Retriable(String),
    // Une erreur 4xx hors throttling signale plutot une config ou un contrat casse.
    Permanent(String),
}
