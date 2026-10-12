//! Signature HMAC des tickets courts edge.

use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::Serialize;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use crate::{
    config::EdgeConfig, errors::AppError, features::auth::CurrentUser, shared::time::now_rfc3339,
};

use super::claims::{EDGE_LIVE_METADATA_SCOPE, EdgeLiveTicketClaims};

/// Reponse HTTP renvoyee au client avant l'ouverture du WebSocket edge.
#[derive(Debug, Serialize)]
pub struct EdgeLiveTicketResponse {
    /// JWT HMAC court a passer en query string `ticket` a `/ws/live`.
    pub ticket: String,
    /// Date RFC3339 d'expiration pour debug client.
    pub expires_at: String,
    /// TTL effectif en secondes.
    pub ttl_secs: u64,
}

/// Genere un ticket court pour autoriser une connexion au WebSocket metadata edge.
pub fn create_live_ticket(
    config: &EdgeConfig,
    user: &CurrentUser,
) -> Result<EdgeLiveTicketResponse, AppError> {
    if !config.ticket_enabled {
        return Err(AppError::Config(
            "NEWS_EDGE_TICKET_ENABLED must be true to issue edge live tickets".to_owned(),
        ));
    }

    // La validation de config impose deja ce secret quand les tickets sont actifs.
    let ticket_secret = config
        .ticket_secret
        .as_deref()
        .ok_or_else(|| AppError::Config("NEWS_EDGE_TICKET_SECRET is missing".to_owned()))?;
    // On capture une seule horloge pour garder `iat`, `exp` et `expires_at` coherents.
    let issued_at = OffsetDateTime::now_utc();
    let expires_at_unix = issued_at.unix_timestamp() + config.ticket_ttl_secs as i64;
    let claims = EdgeLiveTicketClaims::new(user, issued_at, expires_at_unix);
    let header = Header::new(Algorithm::HS256);
    let ticket = encode(
        &header,
        &claims,
        &EncodingKey::from_secret(ticket_secret.as_bytes()),
    )
    .map_err(|error| AppError::downstream(format!("failed to sign edge live ticket: {error}")))?;

    Ok(EdgeLiveTicketResponse {
        ticket,
        expires_at: format_expires_at(expires_at_unix),
        ttl_secs: config.ticket_ttl_secs,
    })
}

/// Verifie localement le meme JWT HMAC que le Durable Object de production.
///
/// Bunny ne transporte que le WebSocket: l'autorisation reste donc identique, cote Rust,
/// lorsque `NEWS_LIVE_FANOUT_ENABLED=true` en DEV.
pub fn verify_live_ticket(config: &EdgeConfig, ticket: &str) -> Result<(), AppError> {
    if !config.ticket_enabled {
        return Err(AppError::unauthorized("edge live tickets are disabled"));
    }
    let secret = config
        .ticket_secret
        .as_deref()
        .ok_or_else(|| AppError::unauthorized("edge live ticket secret is not configured"))?;
    let validation = Validation::new(Algorithm::HS256);
    let decoded = decode::<EdgeLiveTicketClaims>(
        ticket,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map_err(|_| AppError::unauthorized("invalid edge live ticket"))?;
    let claims = decoded.claims;
    let now = OffsetDateTime::now_utc().unix_timestamp();
    if claims.scope != EDGE_LIVE_METADATA_SCOPE
        || claims.sub.trim().is_empty()
        || claims.role.trim().is_empty()
        || claims.iat > now + 30
        || claims.exp <= now
    {
        return Err(AppError::unauthorized("invalid edge live ticket claims"));
    }
    Ok(())
}

/// Formate l'expiration pour conserver le contrat de reponse historique.
fn format_expires_at(expires_at_unix: i64) -> String {
    OffsetDateTime::from_unix_timestamp(expires_at_unix)
        .ok()
        .and_then(|value| value.format(&Rfc3339).ok())
        .unwrap_or_else(now_rfc3339)
}

#[cfg(test)]
mod tests {
    use super::{EdgeLiveTicketClaims, verify_live_ticket};
    use crate::config::EdgeConfig;
    use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
    use time::OffsetDateTime;

    fn config() -> EdgeConfig {
        EdgeConfig {
            publish_enabled: false,
            publish_url: None,
            publish_secret: None,
            publish_timeout_ms: 1_500,
            publish_retry_attempts: 0,
            publish_retry_backoff_ms: 250,
            ticket_enabled: true,
            ticket_secret: Some("a-very-long-test-ticket-secret-over-32".to_owned()),
            ticket_ttl_secs: 60,
        }
    }

    #[test]
    fn verifies_the_same_hmac_ticket_contract_as_the_edge_worker() {
        let config = config();
        let now = OffsetDateTime::now_utc().unix_timestamp();
        let ticket = encode(
            &Header::new(Algorithm::HS256),
            &EdgeLiveTicketClaims {
                sub: "listener".to_owned(),
                email: None,
                role: "listener".to_owned(),
                scope: "live_metadata".to_owned(),
                iat: now,
                exp: now + 30,
            },
            &EncodingKey::from_secret(config.ticket_secret.as_deref().unwrap().as_bytes()),
        )
        .expect("ticket");
        assert!(verify_live_ticket(&config, &ticket).is_ok());
        assert!(verify_live_ticket(&config, "not-a-ticket").is_err());
    }
}
