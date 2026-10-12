//! Middleware d'audit des requetes HTTP lentes.

use std::time::Instant;

use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use tracing::warn;

use crate::app_state::AppState;

/// Journalise les requetes qui depassent le seuil runtime configure.
pub(super) async fn log_slow_requests(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let threshold_ms = state.config.runtime.slow_request_threshold_ms;
    if threshold_ms == 0 {
        return next.run(request).await;
    }

    let method = request.method().clone();
    let path = request.uri().path().to_owned();
    let started_at = Instant::now();
    let response = next.run(request).await;
    let duration_ms = started_at.elapsed().as_millis() as u64;

    if duration_ms >= threshold_ms {
        warn!(
            method = %method,
            path = %path,
            status = response.status().as_u16(),
            duration_ms,
            threshold_ms,
            "slow http request"
        );
    }

    response
}
