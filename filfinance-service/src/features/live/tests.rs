//! Tests de contrats live DTO.

// `json!` permet de comparer proprement des fragments JSON serialises.
use serde_json::json;

// Les DTO live se valident avec les lignes news persistantes qui les alimentent.
use crate::features::{
    calendar_day::PublicCalendarDaySummaryStatus,
    news::domain::{PersistedNewsEvent, PublicSourceStatus, build_news_narration_prefix},
    translation::domain::PersistedNewsTranslation,
};

use super::dto::{
    EdgeAudioFailedPayload, EdgeAudioFinalizedPayload, EdgeAudioReadyPayload, EdgeAudioState,
    EdgeAudioStatePayload, EdgeLiveAudioStatus, EdgeLiveMessage, EdgeNewsCard, EdgeNewsCardPayload,
    EdgeSourceStatusPayload, LiveNewsItem, WsServerMessage,
};

#[test]
fn heartbeat_websocket_message_serialization_uses_dedicated_tag() {
    // On serialise le heartbeat pour valider le contrat JSON vu par les clients WebSocket.
    let serialized = serde_json::to_value(WsServerMessage::heartbeat(
        sample_public_sources(),
        sample_calendar_day_summary_status(),
    ))
    .expect("heartbeat websocket payload should serialize");

    // Le tag `type` doit rester stable, sinon les clients cassent.
    assert_eq!(serialized["type"], json!("heartbeat"));
    assert!(serialized["payload"]["server_time"].is_string());
    assert_eq!(
        serialized["payload"]["sources"][0]["source"],
        json!("financial_juice")
    );
    assert_eq!(
        serialized["payload"]["calendar_day_summary"]["status"],
        json!("audio_ready")
    );
}

#[test]
fn edge_live_news_card_serialization_is_metadata_only() {
    // Ce test verrouille le premier contrat Durable Objects: metadata oui, chunks audio non.
    let message = EdgeLiveMessage::NewsCard {
        payload: EdgeNewsCardPayload {
            event_id: "evt-20260503-000001".to_owned(),
            server_time: "2026-05-03T08:00:00Z".to_owned(),
            card: EdgeNewsCard {
                news_event_id: 7,
                translation_id: Some(11),
                source: "financial_juice".to_owned(),
                event_kind: "headline".to_owned(),
                source_title: "Fed headline".to_owned(),
                translated_title: Some("Titre Fed".to_owned()),
                news_level: Some("active active-critical".to_owned()),
                url: Some("https://example.test/news".to_owned()),
                timestamp: "2026-05-03T07:59:58Z".to_owned(),
            },
        },
    };

    let serialized = serde_json::to_value(message).expect("edge news card should serialize");
    let serialized_text =
        serde_json::to_string(&serialized).expect("edge news card json should stringify");

    assert_eq!(serialized["type"], json!("news_card"));
    assert_eq!(
        serialized["payload"]["event_id"],
        json!("evt-20260503-000001")
    );
    assert_eq!(serialized["payload"]["card"]["news_event_id"], json!(7));
    assert!(serialized["payload"]["server_time"].is_string());
    assert!(!serialized_text.contains("audio_chunk"));
    assert!(!serialized_text.contains("bytes"));
}

#[test]
fn edge_source_status_serialization_is_public_metadata_only() {
    let message = EdgeLiveMessage::SourceStatus {
        payload: EdgeSourceStatusPayload {
            event_id: "source_status:2026-05-09T09:12:00Z".to_owned(),
            server_time: "2026-05-09T09:12:00Z".to_owned(),
            sources: sample_public_sources(),
            calendar_day_summary: Some(sample_calendar_day_summary_status()),
        },
    };
    let serialized = serde_json::to_value(message).expect("edge source status should serialize");

    assert_eq!(serialized["type"], json!("source_status"));
    assert_eq!(
        serialized["payload"]["sources"][0]["source"],
        json!("financial_juice")
    );
    assert!(serialized["payload"]["sources"][0].get("message").is_none());
    assert_eq!(
        serialized["payload"]["calendar_day_summary"],
        json!({
            "local_day": "2026-05-09",
            "status": "audio_ready",
            "audio_available": true,
            "updated_at": "2026-05-09T09:12:00Z"
        })
    );
}

#[test]
fn edge_live_audio_state_serialization_exposes_hls_metadata_only() {
    // L'etat audio edge annonce le replay HLS, pas des donnees audio binaires.
    let message = EdgeLiveMessage::AudioState {
        payload: EdgeAudioStatePayload {
            event_id: "evt-20260503-audio-001".to_owned(),
            server_time: "2026-05-03T08:00:10Z".to_owned(),
            state: EdgeAudioState {
                status: EdgeLiveAudioStatus::Playing,
                replay_today_playlist_url: Some("/audio/replay/today/playlist.m3u8".to_owned()),
                detail: None,
                updated_at: "2026-05-03T08:00:10Z".to_owned(),
            },
        },
    };

    let serialized = serde_json::to_value(message).expect("edge audio state should serialize");
    let serialized_text =
        serde_json::to_string(&serialized).expect("edge audio state json should stringify");

    assert_eq!(serialized["type"], json!("audio_state"));
    assert_eq!(serialized["payload"]["state"]["status"], json!("playing"));
    assert_eq!(
        serialized["payload"]["state"]["replay_today_playlist_url"],
        json!("/audio/replay/today/playlist.m3u8")
    );
    assert!(!serialized_text.contains("chunk"));
    assert!(!serialized_text.contains("bytes"));
}

#[test]
fn edge_live_audio_clip_events_are_metadata_only() {
    // `audio_ready` est le signal temps reel qui remplace le polling client.
    let ready = EdgeLiveMessage::AudioReady {
        payload: EdgeAudioReadyPayload {
            event_id: "audio_ready:00000000000000000011".to_owned(),
            server_time: "2026-05-04T08:00:00Z".to_owned(),
            translation_id: 11,
            news_event_id: 7,
            timestamp: "2026-05-04T07:59:58Z".to_owned(),
            stream_id: "clip-20260504-000011".to_owned(),
            playlist_url: "/audio/clips/clip-20260504-000011/playlist.m3u8".to_owned(),
            published_at: "2026-05-04T08:00:00Z".to_owned(),
            target_duration_secs: 2,
        },
    };
    // `audio_failed` transporte une erreur lisible, jamais le contenu audio echoue.
    let failed = EdgeLiveMessage::AudioFailed {
        payload: EdgeAudioFailedPayload {
            event_id: "audio_failed:00000000000000000011".to_owned(),
            server_time: "2026-05-04T08:00:01Z".to_owned(),
            translation_id: Some(11),
            stream_id: Some("clip-20260504-000011".to_owned()),
            error_message: "tts timeout".to_owned(),
            failed_at: "2026-05-04T08:00:01Z".to_owned(),
        },
    };
    // `audio_finalized` permet d'observer la fin de construction du clip.
    let finalized = EdgeLiveMessage::AudioFinalized {
        payload: EdgeAudioFinalizedPayload {
            event_id: "audio_finalized:00000000000000000011".to_owned(),
            server_time: "2026-05-04T08:00:02Z".to_owned(),
            translation_id: 11,
            stream_id: "clip-20260504-000011".to_owned(),
            playlist_url: "/audio/clips/clip-20260504-000011/playlist.m3u8".to_owned(),
            finalized_at: "2026-05-04T08:00:02Z".to_owned(),
        },
    };

    // Les trois messages doivent garder des tags stables pour le Worker et le client.
    let ready_json = serde_json::to_value(ready).expect("audio_ready should serialize");
    let failed_json = serde_json::to_value(failed).expect("audio_failed should serialize");
    let finalized_json = serde_json::to_value(finalized).expect("audio_finalized should serialize");
    // La forme JSON reste volontairement metadata-only.
    let serialized_text = serde_json::to_string(&json!([
        ready_json.clone(),
        failed_json.clone(),
        finalized_json.clone()
    ]))
    .expect("audio clip event json should stringify");

    assert_eq!(ready_json["type"], json!("audio_ready"));
    assert_eq!(ready_json["payload"]["translation_id"], json!(11));
    assert_eq!(ready_json["payload"]["news_event_id"], json!(7));
    assert_eq!(
        ready_json["payload"]["timestamp"],
        json!("2026-05-04T07:59:58Z")
    );
    assert_eq!(
        ready_json["payload"]["playlist_url"],
        json!("/audio/clips/clip-20260504-000011/playlist.m3u8")
    );
    assert_eq!(failed_json["type"], json!("audio_failed"));
    assert_eq!(
        failed_json["payload"]["error_message"],
        json!("tts timeout")
    );
    assert_eq!(finalized_json["type"], json!("audio_finalized"));
    assert!(!serialized_text.contains("audio_chunk"));
    assert!(!serialized_text.contains("audio_base64"));
    assert!(!serialized_text.contains("bytes"));
}

#[test]
fn live_news_item_copies_news_level_from_source_event() {
    // Ce test verrouille le pont entre la ligne source et le DTO expose par API/WS.
    let source_event = PersistedNewsEvent {
        id: 7,
        source: "financial_juice".to_owned(),
        event_kind: "headline".to_owned(),
        title: "Critical source".to_owned(),
        url: "https://example.test/item".to_owned(),
        published_at: Some("2026-04-21T10:00:00Z".to_owned()),
        event_time: None,
        ingested_at: "2026-04-21T10:00:01Z".to_owned(),
        body: None,
        // Le niveau est porte uniquement par l'event source pour eviter de denormaliser les traductions.
        news_level: Some("active active-critical".to_owned()),
        importance: None,
        country_code: None,
        currency: None,
        actual_value: None,
        forecast_value: None,
        previous_value: None,
        raw_payload_ref: None,
        signature: "critical-signature".to_owned(),
    };
    let translation = PersistedNewsTranslation {
        id: 11,
        news_event_id: source_event.id,
        language: "fr".to_owned(),
        status: "ready".to_owned(),
        source: "financial_juice".to_owned(),
        event_kind: "headline".to_owned(),
        source_title: source_event.title.clone(),
        source_body: None,
        translated_title: "Source critique".to_owned(),
        translated_body: None,
        narration_text: "10:00. Source critique.".to_owned(),
        translation_mode: "heuristic_fr".to_owned(),
        error_message: None,
        published_at: source_event.published_at.clone(),
        created_at: "2026-04-21T10:00:02Z".to_owned(),
        updated_at: "2026-04-21T10:00:03Z".to_owned(),
    };

    let item = LiveNewsItem::from_translation(translation, Some(&source_event), 280);
    let expected_prefix = build_news_narration_prefix(&source_event);

    assert_eq!(item.news_level.as_deref(), Some("active active-critical"));
    assert!(
        item.narration_text
            .starts_with(&format!("{expected_prefix}."))
    );
}

#[test]
fn translated_websocket_message_serialization_uses_dedicated_tag() {
    // On construit un payload live proche du reel pour valider la forme user-facing.
    let item = LiveNewsItem {
        id: 11,
        news_event_id: 7,
        language: "fr".to_owned(),
        status: "ready".to_owned(),
        source: "financial_juice".to_owned(),
        event_kind: "headline".to_owned(),
        source_title: "Powell says rate cuts stay data-dependent".to_owned(),
        source_body: None,
        translated_title: "Powell indique que les baisses de taux dependent des donnees".to_owned(),
        translated_body: None,
        narration_text: "10:00. Powell indique que les baisses de taux dependent des donnees."
            .to_owned(),
        translation_mode: "heuristic_fr".to_owned(),
        error_message: None,
        // Le payload user-facing doit transporter le niveau source quand il existe.
        news_level: Some("active active-critical".to_owned()),
        test_run_id: None,
        url: Some("https://example.test/item".to_owned()),
        timestamp: "2026-04-21T10:00:01Z".to_owned(),
        created_at: "2026-04-21T10:00:01Z".to_owned(),
        updated_at: "2026-04-21T10:00:01Z".to_owned(),
    };

    // La serialisation doit respecter le tag snake_case defini sur l'enum.
    let serialized = serde_json::to_value(WsServerMessage::translated_news_item(item))
        .expect("translated websocket payload should serialize");

    assert_eq!(serialized["type"], json!("translated_news_item"));
    assert_eq!(serialized["payload"]["language"], json!("fr"));
    assert_eq!(serialized["payload"]["news_event_id"], json!(7));
    assert_eq!(
        serialized["payload"]["news_level"],
        json!("active active-critical")
    );
}

mod edge_tests {
    // `json!` rend les assertions de contrat JSON plus lisibles.
    use serde_json::json;

    // On teste les helpers publics du module sans ouvrir de connexion HTTP.
    use crate::features::live::edge::{
        build_translated_news_item_message, create_live_ticket, edge_event_id_for_audio_failed,
        edge_event_id_for_audio_finalized, edge_event_id_for_audio_ready,
        edge_event_id_for_translation_id, is_publishable_translation, publish_audio_failed,
        publish_audio_finalized, publish_audio_ready,
    };
    // `CurrentUser` donne une fixture realiste au ticket edge signe.
    use crate::features::auth::CurrentUser;
    // Les fixtures reconstituent les modeles reels exposes au Worker edge.
    use crate::features::live::dto::{
        EdgeAudioFailedPayload, EdgeAudioFinalizedPayload, EdgeAudioReadyPayload, LiveNewsItem,
    };
    use crate::features::translation::domain::PersistedNewsTranslation;

    #[test]
    fn edge_event_id_for_translation_id_is_stable_and_orderable() {
        // Le padding fixe garde l'ordre lexicographique coherent avec l'ordre numerique SQLite.
        assert_eq!(
            edge_event_id_for_translation_id(42),
            "translated_news_item:00000000000000000042"
        );
    }

    #[test]
    fn audio_edge_event_ids_are_stable_and_orderable() {
        // Chaque type audio garde son propre prefixe pour rester dedupliquable cote client.
        assert_eq!(
            edge_event_id_for_audio_ready(42),
            "audio_ready:00000000000000000042"
        );
        assert_eq!(
            edge_event_id_for_audio_failed(Some(42)),
            "audio_failed:00000000000000000042"
        );
        assert_eq!(
            edge_event_id_for_audio_finalized(42),
            "audio_finalized:00000000000000000042"
        );
    }

    #[test]
    fn audio_publish_helpers_are_noop_when_edge_is_disabled() {
        // Cette fixture verifie que le pipeline audio peut appeler le publisher sans Worker configure.
        let config = crate::config::EdgeConfig::disabled_for_tests();

        publish_audio_ready(
            &config,
            EdgeAudioReadyPayload {
                event_id: edge_event_id_for_audio_ready(11),
                server_time: "2026-05-04T08:00:00Z".to_owned(),
                translation_id: 11,
                news_event_id: 7,
                timestamp: "2026-05-04T07:59:58Z".to_owned(),
                stream_id: "clip-20260504-000011".to_owned(),
                playlist_url: "/audio/clips/clip-20260504-000011/playlist.m3u8".to_owned(),
                published_at: "2026-05-04T08:00:00Z".to_owned(),
                target_duration_secs: 2,
            },
        );
        publish_audio_failed(
            &config,
            EdgeAudioFailedPayload {
                event_id: edge_event_id_for_audio_failed(Some(11)),
                server_time: "2026-05-04T08:00:01Z".to_owned(),
                translation_id: Some(11),
                stream_id: Some("clip-20260504-000011".to_owned()),
                error_message: "tts timeout".to_owned(),
                failed_at: "2026-05-04T08:00:01Z".to_owned(),
            },
        );
        publish_audio_finalized(
            &config,
            EdgeAudioFinalizedPayload {
                event_id: edge_event_id_for_audio_finalized(11),
                server_time: "2026-05-04T08:00:02Z".to_owned(),
                translation_id: 11,
                stream_id: "clip-20260504-000011".to_owned(),
                playlist_url: "/audio/clips/clip-20260504-000011/playlist.m3u8".to_owned(),
                finalized_at: "2026-05-04T08:00:02Z".to_owned(),
            },
        );
    }

    #[test]
    fn translated_news_item_message_matches_edge_contract() {
        // Cette fixture couvre le payload riche envoye au Durable Object, sans aucun audio binaire.
        let item = test_live_item();
        let message = build_translated_news_item_message(
            "translated_news_item:00000000000000000011".to_owned(),
            item,
        );
        let serialized =
            serde_json::to_value(message).expect("edge message should serialize to json");
        let serialized_text =
            serde_json::to_string(&serialized).expect("edge message json should stringify");

        assert_eq!(serialized["type"], json!("translated_news_item"));
        assert_eq!(
            serialized["payload"]["event_id"],
            json!("translated_news_item:00000000000000000011")
        );
        assert_eq!(serialized["payload"]["item"]["id"], json!(11));
        assert_eq!(
            serialized["payload"]["item"]["news_level"],
            json!("active active-critical")
        );
        assert!(serialized["payload"]["server_time"].is_string());
        assert!(!serialized_text.contains("bytes"));
        assert!(!serialized_text.contains("audio_chunk"));
    }

    #[test]
    fn publishable_translation_requires_ready_target_language() {
        // La publication edge ne doit pas exposer des brouillons ou une langue non cible.
        let mut translation = test_translation("fr", "ready");
        assert!(is_publishable_translation(&translation, "fr"));

        translation.status = "error".to_owned();
        assert!(!is_publishable_translation(&translation, "fr"));

        translation.status = "ready".to_owned();
        translation.language = "en".to_owned();
        assert!(!is_publishable_translation(&translation, "fr"));
    }

    #[test]
    fn create_live_ticket_signs_short_lived_listener_scope() {
        // Ce test verrouille le format minimal attendu ensuite par le Worker Cloudflare.
        let config = crate::config::EdgeConfig {
            publish_enabled: false,
            publish_url: None,
            publish_secret: None,
            publish_timeout_ms: 1_500,
            publish_retry_attempts: 0,
            publish_retry_backoff_ms: 250,
            ticket_enabled: true,
            ticket_secret: Some("01234567890123456789012345678901".to_owned()),
            ticket_ttl_secs: 60,
        };
        let user = CurrentUser {
            id: "user-1".to_owned(),
            email: Some("listener@example.test".to_owned()),
            role: "listener".to_owned(),
            access_token_expires_at: Some(1_800_000_000),
        };

        let response = create_live_ticket(&config, &user).expect("ticket should be signed");

        assert_eq!(response.ttl_secs, 60);
        assert!(response.expires_at.contains('T'));
        assert_eq!(response.ticket.split('.').count(), 3);
    }

    fn test_live_item() -> LiveNewsItem {
        // `LiveNewsItem` est le contrat user-facing reutilise tel quel par le message edge.
        LiveNewsItem {
            id: 11,
            news_event_id: 7,
            language: "fr".to_owned(),
            status: "ready".to_owned(),
            source: "financial_juice".to_owned(),
            event_kind: "headline".to_owned(),
            source_title: "Fed headline".to_owned(),
            source_body: Some("Source body".to_owned()),
            translated_title: "Titre Fed".to_owned(),
            translated_body: None,
            narration_text: "10:00. Information importante. Titre Fed.".to_owned(),
            translation_mode: "opus_mt_tc_big_local_session".to_owned(),
            error_message: None,
            news_level: Some("active active-critical".to_owned()),
            test_run_id: None,
            url: Some("https://example.test/news".to_owned()),
            timestamp: "2026-05-03T08:00:00Z".to_owned(),
            created_at: "2026-05-03T08:00:01Z".to_owned(),
            updated_at: "2026-05-03T08:00:01Z".to_owned(),
        }
    }

    fn test_translation(language: &str, status: &str) -> PersistedNewsTranslation {
        // La fixture minimale permet de tester le filtre de publication sans storage ni HTTP.
        PersistedNewsTranslation {
            id: 11,
            news_event_id: 7,
            language: language.to_owned(),
            status: status.to_owned(),
            source: "financial_juice".to_owned(),
            event_kind: "headline".to_owned(),
            source_title: "Fed headline".to_owned(),
            source_body: None,
            translated_title: "Titre Fed".to_owned(),
            translated_body: None,
            narration_text: "10:00. Titre Fed.".to_owned(),
            translation_mode: "heuristic_fr".to_owned(),
            error_message: None,
            published_at: Some("2026-05-03T08:00:00Z".to_owned()),
            created_at: "2026-05-03T08:00:01Z".to_owned(),
            updated_at: "2026-05-03T08:00:01Z".to_owned(),
        }
    }
}

mod websocket_tests {
    // `json!` construit facilement des valeurs JSON d'assertion sans declarations verbeuses.
    use serde_json::json;

    use super::{sample_calendar_day_summary_status, sample_public_sources};

    // Les tests verifient surtout la stabilite du contrat wire, pas le comportement reseau complet.
    use crate::features::live::dto::{
        EdgeAudioReadyPayload, EdgeLiveMessage, WsServerMessage, WsTestTraceMessage,
    };

    use crate::features::live::websocket::{
        parse_websocket_auth_token, serialize_live_metadata_message, serialize_message,
        serialize_test_trace_message,
    };

    #[test]
    fn parse_websocket_auth_token_reads_trimmed_access_token() {
        // Le handshake WebSocket attend un JSON simple envoye juste apres `onopen` cote client.
        let token =
            parse_websocket_auth_token(r#"{ "type": "auth", "access_token": "  bearer-token  " }"#)
                .expect("auth payload should be parsed");

        assert_eq!(token, "bearer-token");
    }

    #[test]
    fn parse_websocket_auth_token_rejects_empty_access_token() {
        // Un token vide doit fermer le socket avant tout `hello` ou replay metier.
        let error = parse_websocket_auth_token(r#"{ "type": "auth", "access_token": " " }"#)
            .expect_err("empty token should be rejected");

        assert_eq!(error, "empty auth token");
    }

    #[test]
    fn serialize_message_emits_hello_contract() {
        // Le `hello` user-facing est la premiere chose que voit un client; son schema doit rester stable.
        let payload = serialize_message(WsServerMessage::hello(
            2048,
            sample_public_sources(),
            sample_calendar_day_summary_status(),
        ))
        .expect("hello should serialize");
        let parsed: serde_json::Value =
            serde_json::from_str(&payload).expect("hello payload should be valid json");

        assert_eq!(parsed["type"], json!("hello"));
        assert_eq!(parsed["payload"]["protocol_version"], json!(1));
        assert_eq!(parsed["payload"]["broadcast_buffer"], json!(2048));
        assert_eq!(
            parsed["payload"]["sources"][0]["source"],
            json!("financial_juice")
        );
        assert_eq!(
            parsed["payload"]["calendar_day_summary"]["audio_available"],
            json!(true)
        );
    }

    #[test]
    fn serialize_test_trace_message_emits_hello_contract() {
        // Le canal debug a son propre `hello`, distinct du flux user-facing.
        let payload = serialize_test_trace_message(WsTestTraceMessage::hello(2048))
            .expect("test trace hello should serialize");
        let parsed: serde_json::Value =
            serde_json::from_str(&payload).expect("test trace hello payload should be valid json");

        assert_eq!(parsed["type"], json!("hello"));
        assert_eq!(parsed["payload"]["protocol_version"], json!(1));
        assert_eq!(parsed["payload"]["broadcast_buffer"], json!(2048));
    }

    #[test]
    fn serialize_live_metadata_message_emits_audio_ready_contract() {
        // Le fallback backend `/ws/news` doit transporter le meme contrat audio que l'edge.
        let payload = serialize_live_metadata_message(EdgeLiveMessage::AudioReady {
            payload: EdgeAudioReadyPayload {
                event_id: "audio_ready:00000000000000000011".to_owned(),
                server_time: "2026-05-04T16:23:03Z".to_owned(),
                translation_id: 11,
                news_event_id: 42,
                timestamp: "2026-05-04T16:23:02Z".to_owned(),
                stream_id: "clip-test".to_owned(),
                playlist_url: "/audio/clips/clip-test/playlist.m3u8".to_owned(),
                published_at: "2026-05-04T16:23:03Z".to_owned(),
                target_duration_secs: 2,
            },
        })
        .expect("audio_ready should serialize");
        let parsed: serde_json::Value =
            serde_json::from_str(&payload).expect("audio_ready payload should be valid json");

        assert_eq!(parsed["type"], json!("audio_ready"));
        assert_eq!(parsed["payload"]["stream_id"], json!("clip-test"));
        assert_eq!(
            parsed["payload"]["playlist_url"],
            json!("/audio/clips/clip-test/playlist.m3u8")
        );
    }
}

fn sample_public_sources() -> Vec<PublicSourceStatus> {
    vec![PublicSourceStatus {
        source: "financial_juice".to_owned(),
        enabled: true,
        status: "active".to_owned(),
        updated_at: "2026-05-09T09:12:00Z".to_owned(),
    }]
}

fn sample_calendar_day_summary_status() -> PublicCalendarDaySummaryStatus {
    PublicCalendarDaySummaryStatus {
        local_day: "2026-05-09".to_owned(),
        status: "audio_ready".to_owned(),
        audio_available: true,
        updated_at: "2026-05-09T09:12:00Z".to_owned(),
    }
}
