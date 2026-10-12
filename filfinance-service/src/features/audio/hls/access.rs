//! Tickets courts d'acces aux playlists et segments HLS.

use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use url::form_urlencoded;

use crate::{config::HlsConfig, errors::AppError, shared::time::now_rfc3339};

const HLS_ACCESS_SCOPE: &str = "hls_audio";

/// Reponse HTTP renvoyee au client avant une lecture HLS.
#[derive(Debug, Serialize)]
pub struct HlsAccessTicketResponse {
    /// Indique si l'origin exige effectivement un ticket.
    pub enabled: bool,
    /// JWT HMAC court a passer en query string `hls_ticket`.
    pub ticket: Option<String>,
    /// Date RFC3339 d'expiration pour debug client.
    pub expires_at: Option<String>,
    /// TTL effectif restant au moment de l'emission.
    pub ttl_secs: Option<u64>,
}

#[derive(Debug, Deserialize, Serialize)]
struct HlsAccessClaims {
    scope: String,
    iat: i64,
    exp: i64,
}

/// Genere un ticket partage par bucket temporel pour eviter un cache CDN par utilisateur.
pub(in crate::features::audio) fn create_hls_access_ticket(
    config: &HlsConfig,
) -> Result<HlsAccessTicketResponse, AppError> {
    if !config.enabled || !config.access_control_enabled {
        return Ok(HlsAccessTicketResponse {
            enabled: false,
            ticket: None,
            expires_at: None,
            ttl_secs: None,
        });
    }

    let secret = hls_access_secret(config)?;
    let now = OffsetDateTime::now_utc();
    let (issued_at_unix, expires_at_unix) =
        ticket_bucket_window(now.unix_timestamp(), config.access_ticket_ttl_secs);
    let claims = HlsAccessClaims {
        scope: HLS_ACCESS_SCOPE.to_owned(),
        iat: issued_at_unix,
        exp: expires_at_unix,
    };
    let ticket = encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|error| AppError::downstream(format!("failed to sign hls access ticket: {error}")))?;

    Ok(HlsAccessTicketResponse {
        enabled: true,
        ticket: Some(ticket),
        expires_at: Some(format_expires_at(expires_at_unix)),
        ttl_secs: Some((expires_at_unix - now.unix_timestamp()).max(0) as u64),
    })
}

/// Verifie le ticket HLS fourni sur une route audio publique.
pub(in crate::features::audio) fn verify_hls_access_ticket(
    config: &HlsConfig,
    ticket: Option<&str>,
) -> Result<(), AppError> {
    if !config.enabled || !config.access_control_enabled {
        return Ok(());
    }

    let ticket = ticket
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| AppError::unauthorized("missing hls access ticket"))?;
    let secret = hls_access_secret(config)?;
    let mut validation = Validation::new(Algorithm::HS256);
    validation.set_required_spec_claims(&["exp", "iat"]);
    validation.validate_aud = false;

    let token = decode::<HlsAccessClaims>(
        ticket,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map_err(|_| AppError::unauthorized("invalid hls access ticket"))?;

    if token.claims.scope != HLS_ACCESS_SCOPE {
        return Err(AppError::unauthorized("invalid hls access ticket scope"));
    }

    Ok(())
}

/// Ajoute le ticket HLS aux URLs de segments contenues dans une playlist.
pub(in crate::features::audio) fn append_hls_ticket_to_playlist(
    playlist_body: &str,
    ticket: &str,
) -> String {
    let encoded_ticket: String = form_urlencoded::byte_serialize(ticket.as_bytes()).collect();
    playlist_body
        .lines()
        .map(|line| {
            if line.is_empty() || line.starts_with('#') {
                line.to_owned()
            } else {
                let separator = if line.contains('?') { '&' } else { '?' };
                format!("{line}{separator}hls_ticket={encoded_ticket}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

fn hls_access_secret(config: &HlsConfig) -> Result<&str, AppError> {
    config
        .access_ticket_secret
        .as_deref()
        .filter(|secret| !secret.trim().is_empty())
        .ok_or_else(|| AppError::Config("NEWS_HLS_ACCESS_TICKET_SECRET is missing".to_owned()))
}

fn ticket_bucket_window(now_unix: i64, bucket_secs: u64) -> (i64, i64) {
    let bucket_secs = bucket_secs.max(1) as i64;
    let bucket_start = now_unix - now_unix.rem_euclid(bucket_secs);
    (bucket_start, bucket_start + bucket_secs * 2)
}

fn format_expires_at(expires_at_unix: i64) -> String {
    OffsetDateTime::from_unix_timestamp(expires_at_unix)
        .ok()
        .and_then(|value| value.format(&Rfc3339).ok())
        .unwrap_or_else(now_rfc3339)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_hls_access_does_not_issue_a_ticket() {
        let config = test_config(false);

        let response = create_hls_access_ticket(&config).expect("disabled ticket should respond");

        assert!(!response.enabled);
        assert!(response.ticket.is_none());
    }

    #[test]
    fn hls_ticket_is_shared_within_the_same_time_bucket() {
        let (first_iat, first_exp) = ticket_bucket_window(1_000, 300);
        let (second_iat, second_exp) = ticket_bucket_window(1_199, 300);

        assert_eq!(first_iat, second_iat);
        assert_eq!(first_exp, second_exp);
    }

    #[test]
    fn issued_hls_ticket_verifies_with_same_config() {
        let config = test_config(true);
        let response = create_hls_access_ticket(&config).expect("ticket should be issued");

        verify_hls_access_ticket(&config, response.ticket.as_deref())
            .expect("fresh ticket should verify");
    }

    #[test]
    fn signing_secret_change_rejects_previous_tickets() {
        let previous_config = test_config(true);
        let previous = create_hls_access_ticket(&previous_config).expect("synthetic ticket");
        let mut renewed_config = test_config(true);
        renewed_config.access_ticket_secret = Some(
            "synthetic-renewed-signing-key-for-isolated-tests-only".to_owned(),
        );
        assert!(verify_hls_access_ticket(&renewed_config, previous.ticket.as_deref()).is_err());
        let renewed = create_hls_access_ticket(&renewed_config).expect("renewed synthetic ticket");
        verify_hls_access_ticket(&renewed_config, renewed.ticket.as_deref())
            .expect("renewed ticket should verify");
    }

    #[test]
    fn hls_ticket_rejects_missing_or_wrong_tokens() {
        let config = test_config(true);

        assert!(verify_hls_access_ticket(&config, None).is_err());
        assert!(verify_hls_access_ticket(&config, Some("not-a-jwt")).is_err());
    }

    #[test]
    fn hls_ticket_rejects_expired_tokens() {
        let config = test_config(true);
        let secret = hls_access_secret(&config).expect("test secret should exist");
        let expired_claims = HlsAccessClaims {
            scope: HLS_ACCESS_SCOPE.to_owned(),
            iat: 1,
            exp: 2,
        };
        let expired_ticket = encode(
            &Header::new(Algorithm::HS256),
            &expired_claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .expect("expired test token should sign");

        assert!(verify_hls_access_ticket(&config, Some(&expired_ticket)).is_err());
    }

    #[test]
    fn hls_playlist_rewrite_adds_ticket_to_media_uris_only() {
        let playlist = "#EXTM3U\n#EXTINF:2.0,\n/audio/clips/a/segment-000.ts\n";

        let rewritten = append_hls_ticket_to_playlist(playlist, "a token+with/special=");

        assert!(rewritten.contains("#EXTM3U\n#EXTINF:2.0,\n"));
        assert!(
            rewritten
                .contains("/audio/clips/a/segment-000.ts?hls_ticket=a+token%2Bwith%2Fspecial%3D\n")
        );
    }

    fn test_config(access_control_enabled: bool) -> HlsConfig {
        HlsConfig {
            enabled: true,
            access_control_enabled,
            access_ticket_secret: Some(
                "0123456789abcdef0123456789abcdef0123456789abcdef".to_owned(),
            ),
            access_ticket_ttl_secs: 300,
            segments_dir: "data/hls-test".to_owned(),
            ffmpeg_command: "ffmpeg".to_owned(),
            replay_segment_target_duration_secs: 4,
            clip_segment_target_duration_secs: 2,
        }
    }
}
