//! Claims JWT du ticket court edge.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::features::auth::CurrentUser;

/// Scope stable attendu par le Worker pour autoriser le WebSocket live metadata.
pub(crate) const EDGE_LIVE_METADATA_SCOPE: &str = "live_metadata";

/// Claims signes dans le ticket court consomme par le Worker edge.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct EdgeLiveTicketClaims {
    /// Identifiant utilisateur Better Auth ou identifiant de dev.
    pub(crate) sub: String,
    /// Email optionnel, utile au debug edge sans etre requis pour l'autorisation.
    pub(crate) email: Option<String>,
    /// Role applicatif signe par Better Auth.
    pub(crate) role: String,
    /// Scope minimal du ticket; le Worker refuse tout autre usage.
    pub(crate) scope: String,
    /// Timestamp Unix d'emission.
    pub(crate) iat: i64,
    /// Timestamp Unix d'expiration.
    pub(crate) exp: i64,
}

impl EdgeLiveTicketClaims {
    /// Construit les claims avec une horloge deja capturee par le signataire.
    pub(super) fn new(user: &CurrentUser, issued_at: OffsetDateTime, expires_at_unix: i64) -> Self {
        Self {
            sub: user.id.clone(),
            email: user.email.clone(),
            role: user.role.clone(),
            scope: EDGE_LIVE_METADATA_SCOPE.to_owned(),
            iat: issued_at.unix_timestamp(),
            exp: expires_at_unix,
        }
    }
}
