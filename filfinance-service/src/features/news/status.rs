//! Projections des etats de sources pour l'admin et les WebSockets publics.

use crate::{
    app_state::AppState,
    features::{
        ingestion::runtime::browser_collector_status_stale_after_ms,
        news::domain::{PublicSourceStatus, SourceStatus},
    },
    shared::time::now_rfc3339,
};

/// Construit la vue detaillee reservee a `/api/sources`.
pub(crate) fn admin_source_statuses(state: &AppState) -> Vec<SourceStatus> {
    let mut statuses = state.config.source_statuses();
    if let Some(financial_juice) = statuses
        .iter_mut()
        .find(|source| source.name == state.config.sources.financial_juice.name)
    {
        // Le champ `enabled` doit montrer l'etat effectif courant, pas seulement le fallback env.
        financial_juice.enabled = state.financial_juice_runtime.is_enabled();
    }

    if let Some(financial_juice_browser) = statuses
        .iter_mut()
        .find(|source| source.name == state.config.sources.financial_juice_browser.source.name)
    {
        let effective_status = state.financial_juice_browser_runtime.effective_status(
            state.financial_juice_runtime.is_enabled(),
            browser_collector_status_stale_after_ms(
                state
                    .config
                    .sources
                    .financial_juice_browser
                    .source
                    .poll_interval_ms,
            ),
        );
        financial_juice_browser.enabled = effective_status.enabled;
        financial_juice_browser.set_runtime_status(
            effective_status.status,
            effective_status.message,
            effective_status.updated_at,
            effective_status.service_accessible,
            effective_status.main_feed_available,
            effective_status.stale,
        );
    }

    if let Some(x) = statuses
        .iter_mut()
        .find(|source| source.name == state.config.sources.x.source.name)
    {
        let x_settings = state.x_runtime.settings();
        x.enabled = x_settings.enabled;
        x.updated_at = x_settings.updated_at;
        x.status = Some(if x.enabled { "ok" } else { "disabled" }.to_owned());
        x.message = x_settings
            .disabled_reason
            .map(|reason| format!("disabled reason: {reason}"));
    }

    statuses
}

/// Construit le payload public compact diffuse dans `hello`, `heartbeat` et l'edge.
pub(crate) fn public_source_statuses(state: &AppState) -> Vec<PublicSourceStatus> {
    let snapshot_time = now_rfc3339();

    admin_source_statuses(state)
        .into_iter()
        .map(|source| public_source_status_from_admin(source, &snapshot_time))
        .collect()
}

fn public_source_status_from_admin(
    source: SourceStatus,
    snapshot_time: &str,
) -> PublicSourceStatus {
    let status = if source.enabled {
        "active"
    } else {
        match source.status.as_deref() {
            Some("blocked") => "blocked",
            Some("degraded") => "degraded",
            Some("stale") => "stale",
            _ => "disabled",
        }
    };

    PublicSourceStatus {
        source: source.name,
        enabled: source.enabled,
        status: status.to_owned(),
        updated_at: source
            .updated_at
            .filter(|updated_at| !updated_at.trim().is_empty())
            .unwrap_or_else(|| snapshot_time.to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_source_status_keeps_compact_contract() {
        let public = public_source_status_from_admin(
            SourceStatus {
                name: "financial_juice_browser".to_owned(),
                enabled: false,
                configured_enabled: true,
                url: "https://example.test".to_owned(),
                poll_interval_ms: 5_000,
                request_timeout_ms: 15_000,
                status: Some("blocked".to_owned()),
                message: Some("cloudflare challenge".to_owned()),
                updated_at: Some("2026-05-09T09:12:00Z".to_owned()),
                service_accessible: Some(true),
                main_feed_available: Some(false),
                stale: Some(false),
            },
            "2026-05-09T09:13:00Z",
        );
        let serialized =
            serde_json::to_value(public).expect("public source status should serialize");

        assert_eq!(serialized["source"], "financial_juice_browser");
        assert_eq!(serialized["enabled"], false);
        assert_eq!(serialized["status"], "blocked");
        assert_eq!(serialized["updated_at"], "2026-05-09T09:12:00Z");
        assert!(serialized.get("message").is_none());
        assert!(serialized.get("url").is_none());
    }

    #[test]
    fn active_public_source_status_uses_active_status() {
        let public = public_source_status_from_admin(
            SourceStatus {
                name: "financial_juice".to_owned(),
                enabled: true,
                configured_enabled: true,
                url: "https://example.test".to_owned(),
                poll_interval_ms: 5_000,
                request_timeout_ms: 5_000,
                status: None,
                message: None,
                updated_at: None,
                service_accessible: None,
                main_feed_available: None,
                stale: None,
            },
            "2026-05-09T09:13:00Z",
        );

        assert_eq!(public.status, "active");
        assert_eq!(public.updated_at, "2026-05-09T09:13:00Z");
    }
}
