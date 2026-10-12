//! Facade du traitement d'un evenement brut recu par le pipeline d'ingestion.

// `broadcast`, `dedup`, `persistence` et `trace` portent les etapes du hot path.
mod broadcast;
mod dedup;
mod persistence;
mod trace;

// `HashMap`, `Duration` et `Instant` portent l'etat de dedup memoire du hot path.
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use tracing::{info, warn};

use crate::{
    app_state::AppState,
    features::{
        blacklist::{
            domain::{
                BLACKLIST_ACTION_STRIP_CONTENT, NewNewsTruncationAudit, NewsBlacklistListParams,
            },
            matcher::find_news_blacklist_match,
        },
        news::domain::NewsEvent,
    },
    shared::text::trim_chars,
};

use super::super::dedup::evict_expired;
use dedup::{is_recent_duplicate, record_pending_duplicate, remember_signature};

/// Persiste un evenement si la dedup memoire et durable l'acceptent, puis le diffuse.
pub(super) async fn process_event(
    state: &AppState,
    mut event: NewsEvent,
    recently_seen: &mut HashMap<String, Instant>,
    pending_duplicate_counts: &mut HashMap<(String, String), u64>,
    ttl: Duration,
) {
    // Les traces de test E2E suivent le meme pipeline que la production, donc on marque
    // ici l'entree reelle de l'evenement dans le hot path.
    trace::emit_received(state, &event);

    audit_upstream_truncation_if_needed(state, &event).await;

    if apply_blacklist(state, &mut event).await {
        return;
    }

    // On purge d'abord les signatures trop anciennes pour garder la dedup memoire bornee.
    evict_expired(recently_seen, ttl);

    // On coupe ici les doublons recents pour proteger le hot path et les downstreams.
    if event.source != "financial_juice"
        && is_recent_duplicate(recently_seen, &event.signature, ttl)
    {
        trace::emit_memory_duplicate(state, &event);
        record_pending_duplicate(pending_duplicate_counts, &event);
        return;
    }

    // Si l'evenement est accepte, on memorise sa signature avec l'instant courant.
    remember_signature(recently_seen, &event.signature);

    // Le stockage fait foi pour l'idempotence durable; le broadcast ne part qu'apres persistence.
    // Cela evite de diffuser un item qui serait ensuite refuse par la base comme doublon.
    persistence::persist_then_broadcast(state, &event).await;
}

async fn apply_blacklist(state: &AppState, event: &mut NewsEvent) -> bool {
    let entries = match state
        .news
        .list_news_blacklist_entries(&NewsBlacklistListParams::active_for_pipeline())
        .await
    {
        Ok(entries) => entries,
        Err(error) => {
            warn!(%error, source = %event.source, "failed to load news blacklist; event continues");
            return false;
        }
    };

    let Some(matched) = find_news_blacklist_match(event, &entries) else {
        return false;
    };

    if let Err(error) = state.news.insert_news_blacklist_hit(&matched.hit).await {
        warn!(
            %error,
            blacklist_entry_id = matched.entry.id,
            source = %event.source,
            signature = %event.signature,
            "failed to persist news blacklist hit"
        );
    }

    info!(
        blacklist_entry_id = matched.entry.id,
        term = %matched.entry.term,
        match_mode = %matched.entry.match_mode,
        target_scope = %matched.entry.target_scope,
        action = %matched.hit.action,
        source = %event.source,
        signature = %event.signature,
        "news blacklist action applied"
    );

    if matched.hit.action == BLACKLIST_ACTION_STRIP_CONTENT {
        event.body = None;
        false
    } else {
        true
    }
}

async fn audit_upstream_truncation_if_needed(state: &AppState, event: &NewsEvent) {
    if event.source != "financial_juice" {
        return;
    }

    let Some(body) = event.body.as_deref() else {
        return;
    };
    let Some(marker) = detect_truncation_marker(body) else {
        return;
    };

    let audit = NewNewsTruncationAudit {
        source: event.source.clone(),
        field_name: "body".to_owned(),
        marker: marker.to_owned(),
        source_title: trim_chars(&event.title, 500),
        field_excerpt: trim_chars(body, 500),
        raw_payload_ref: event.raw_payload_ref.clone(),
        signature: event.signature.clone(),
    };

    if let Err(error) = state.news.insert_news_truncation_audit(&audit).await {
        warn!(
            %error,
            source = %event.source,
            signature = %event.signature,
            "failed to persist upstream truncation audit"
        );
    }
}

fn detect_truncation_marker(body: &str) -> Option<&'static str> {
    if body.contains('…') {
        Some("…")
    } else if body.contains("...") {
        Some("...")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, time::Duration};

    use super::*;
    use crate::features::{
        blacklist::domain::{
            BLACKLIST_ACTION_STRIP_CONTENT, BLACKLIST_MATCH_MODE_PARTIAL,
            BLACKLIST_TARGET_SCOPE_CONTENT, CreateNewsBlacklistEntryRequest, NewsAuditListParams,
        },
        news::domain::NewsListParams,
        test_support::app_state,
    };

    #[tokio::test]
    async fn process_event_rejects_blacklisted_news_before_persistence() {
        let state = app_state().await;
        let entry = CreateNewsBlacklistEntryRequest {
            term: "blocked phrase".to_owned(),
            match_mode: BLACKLIST_MATCH_MODE_PARTIAL.to_owned(),
            target_scope: None,
            enabled: true,
            notes: None,
        }
        .normalize()
        .expect("blacklist fixture should normalize");
        state
            .news
            .create_news_blacklist_entry(&entry)
            .await
            .expect("blacklist entry should be inserted");

        let event = NewsEvent::new_headline(
            "financial_juice",
            "Market update with blocked phrase",
            "https://example.test/blocked",
            Some("2026-06-10T10:00:00Z".to_owned()),
            Some("Regular body".to_owned()),
        );
        process_event(
            &state,
            event,
            &mut HashMap::new(),
            &mut HashMap::new(),
            Duration::from_secs(300),
        )
        .await;

        let news_page = state
            .news
            .list_news(&NewsListParams {
                limit: 10,
                source: None,
                event_kind: None,
                before_id: None,
            })
            .await
            .expect("news listing should succeed");
        let hits = state
            .news
            .list_news_blacklist_hits(&NewsAuditListParams { limit: 10 })
            .await
            .expect("blacklist hits should be listed");

        assert!(news_page.items.is_empty());
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].matched_field, "title");
    }

    #[tokio::test]
    async fn process_event_strips_body_when_content_scope_matches() {
        let state = app_state().await;
        let entry = CreateNewsBlacklistEntryRequest {
            term: "blocked detail".to_owned(),
            match_mode: BLACKLIST_MATCH_MODE_PARTIAL.to_owned(),
            target_scope: Some(BLACKLIST_TARGET_SCOPE_CONTENT.to_owned()),
            enabled: true,
            notes: None,
        }
        .normalize()
        .expect("blacklist fixture should normalize");
        state
            .news
            .create_news_blacklist_entry(&entry)
            .await
            .expect("blacklist entry should be inserted");

        let event = NewsEvent::new_headline(
            "financial_juice",
            "Market update remains visible",
            "https://example.test/stripped",
            Some("2026-06-10T10:00:00Z".to_owned()),
            Some("This contains blocked detail for body removal".to_owned()),
        );
        process_event(
            &state,
            event,
            &mut HashMap::new(),
            &mut HashMap::new(),
            Duration::from_secs(300),
        )
        .await;

        let news_page = state
            .news
            .list_news(&NewsListParams {
                limit: 10,
                source: Some("financial_juice".to_owned()),
                event_kind: None,
                before_id: None,
            })
            .await
            .expect("news listing should succeed");
        let hits = state
            .news
            .list_news_blacklist_hits(&NewsAuditListParams { limit: 10 })
            .await
            .expect("blacklist hits should be listed");

        assert_eq!(news_page.items.len(), 1);
        assert_eq!(news_page.items[0].title, "Market update remains visible");
        assert!(news_page.items[0].body.is_none());
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].matched_field, "body");
        assert_eq!(hits[0].target_scope, BLACKLIST_TARGET_SCOPE_CONTENT);
        assert_eq!(hits[0].action, BLACKLIST_ACTION_STRIP_CONTENT);
    }

    #[tokio::test]
    async fn process_event_audits_financial_juice_body_truncation_upstream() {
        let state = app_state().await;
        let event = NewsEvent::new_headline(
            "financial_juice",
            "Truncated body fixture",
            "https://example.test/truncated",
            Some("2026-06-10T10:00:00Z".to_owned()),
            Some("The volatility response is incomplete…".to_owned()),
        );

        process_event(
            &state,
            event,
            &mut HashMap::new(),
            &mut HashMap::new(),
            Duration::from_secs(300),
        )
        .await;

        let audits = state
            .news
            .list_news_truncation_audits(&NewsAuditListParams { limit: 10 })
            .await
            .expect("truncation audits should be listed");
        let news_page = state
            .news
            .list_news(&NewsListParams {
                limit: 10,
                source: Some("financial_juice".to_owned()),
                event_kind: None,
                before_id: None,
            })
            .await
            .expect("news listing should succeed");

        assert_eq!(audits.len(), 1);
        assert_eq!(audits[0].marker, "…");
        assert_eq!(news_page.items.len(), 1);
    }
}
