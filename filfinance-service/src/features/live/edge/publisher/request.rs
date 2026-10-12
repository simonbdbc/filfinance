//! Tentative HTTP unique de publication edge.

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use hmac::{Hmac, Mac};
use serde::Serialize;
use sha2::Sha256;

use super::{EdgePublisher, error::EdgePublishError, status::is_retriable_status};
use crate::shared::time::now_rfc3339;

impl EdgePublisher {
    /// Execute une seule tentative HTTP vers `/publish/live`.
    pub(super) async fn publish_once<TMessage>(
        &self,
        message: &TMessage,
    ) -> Result<(), EdgePublishError>
    where
        TMessage: Serialize + ?Sized,
    {
        // La validation de config impose deja ces champs quand le publisher est actif.
        let publish_url =
            self.config.publish_url.as_deref().ok_or_else(|| {
                EdgePublishError::Permanent("edge publish url is missing".to_owned())
            })?;
        let publish_secret = self.config.publish_secret.as_deref().ok_or_else(|| {
            EdgePublishError::Permanent("edge publish secret is missing".to_owned())
        })?;
        let body = serde_json::to_string(message).map_err(|error| {
            EdgePublishError::Permanent(format!(
                "edge publish message serialization failed: {error}"
            ))
        })?;
        crate::features::calendar_logs::payload("edge.request_body", &body);
        crate::features::calendar_logs::event(
            "edge.request",
            serde_json::json!({"url":publish_url,"method":"POST"}),
        );
        let timestamp = now_rfc3339();
        let signature = build_publish_signature(publish_secret, &timestamp, &body)?;

        let response = self
            .client
            .post(publish_url)
            .header("Content-Type", "application/json")
            .header("X-FilFinance-Timestamp", timestamp)
            .header("X-FilFinance-Signature", signature)
            .body(body)
            .send()
            .await
            .map_err(|error| EdgePublishError::Retriable(error.to_string()))?;

        let status = response.status();
        crate::features::calendar_logs::event(
            "edge.response",
            serde_json::json!({"status":status.as_u16(),"headers":format!("{:?}",response.headers())}),
        );
        if status.is_success() {
            return Ok(());
        }

        let body = response
            .text()
            .await
            .unwrap_or_else(|error| format!("failed to read edge error body: {error}"));
        crate::features::calendar_logs::payload("edge.response_error_body", &body);
        let message = format!("edge publish failed with HTTP {status}: {body}");

        if is_retriable_status(status) {
            Err(EdgePublishError::Retriable(message))
        } else {
            Err(EdgePublishError::Permanent(message))
        }
    }
}

pub(super) fn build_publish_signature(
    secret: &str,
    timestamp: &str,
    raw_body: &str,
) -> Result<String, EdgePublishError> {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).map_err(|error| {
        EdgePublishError::Permanent(format!(
            "edge publish secret cannot initialize HMAC: {error}"
        ))
    })?;
    mac.update(timestamp.as_bytes());
    mac.update(b".");
    mac.update(raw_body.as_bytes());
    let signature = mac.finalize().into_bytes();

    Ok(format!("sha256={}", URL_SAFE_NO_PAD.encode(signature)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_publish_signature_matches_worker_contract_fixture() {
        let signature = build_publish_signature(
            "publish-secret-01234567890123456789",
            "2026-05-09T09:12:00.000Z",
            r#"{"type":"source_status"}"#,
        )
        .expect("signature should build");

        assert_eq!(
            signature,
            "sha256=wrG8iK1U4o3SVMdqs3SKqRxCXMCPBpbNXhDiYxoNWUM"
        );
    }
}
