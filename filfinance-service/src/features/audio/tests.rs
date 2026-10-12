//! Tests HTTP et runtime de la feature audio.

// Les extracteurs Axum permettent d'appeler les handlers runtime sans serveur HTTP.
use axum::{Json, extract::State};

use crate::features::{
    audio::{hls::runtime::hls_runtime, tts::runtime::tts_runtime},
    test_support::app_state,
};

#[tokio::test]
async fn tts_runtime_returns_a_serializable_snapshot() {
    let state = app_state().await;
    state.tts_runtime.record_received();
    state.tts_runtime.record_generated("audio-42", 222, 4096, 1);

    let Json(payload) = tts_runtime(State(state)).await;

    assert_eq!(payload["tts"]["provider"], "piper_local_session");
    assert_eq!(payload["tts"]["received_total"], 1);
    assert_eq!(payload["tts"]["generated_total"], 1);
    assert_eq!(payload["tts"]["pending_total"], 0);
    assert_eq!(payload["tts"]["generation_duration_p50_ms"], 222);
    assert_eq!(payload["tts"]["generation_duration_p95_ms"], 222);
    assert_eq!(payload["tts"]["last_stream_id"], "audio-42");
}

#[tokio::test]
async fn hls_runtime_returns_a_serializable_snapshot() {
    // Le snapshot HLS doit rester disponible meme quand l'origin est desactivee en test.
    let state = app_state().await;

    // Le handler est appele directement pour verifier le contrat JSON sans middleware HTTP.
    let Json(payload) = hls_runtime(State(state)).await;

    // Les compteurs a zero documentent l'etat initial attendu avant toute publication HLS.
    assert_eq!(payload["hls"]["enabled"], false);
    assert_eq!(payload["hls"]["clip_playlist_requests_total"], 0);
    assert_eq!(payload["hls"]["clip_segment_requests_total"], 0);
    assert_eq!(payload["hls"]["replay_playlist_requests_total"], 0);
    assert_eq!(payload["hls"]["replay_segment_requests_total"], 0);
    assert_eq!(payload["hls"]["ffmpeg_failed_total"], 0);
    assert_eq!(payload["hls"]["publish_duration_sample_count"], 0);
}

mod service_tests {
    // Les tests ciblent surtout les helpers purs qui verrouillent le contrat audio sans dependre du systeme.
    use crate::features::audio::service::{build_live_clip_body_text, next_audio_stream_id};
    use crate::features::audio::tts::TtsRuntimeState;
    use crate::{
        config::TtsConfig,
        features::{
            audio::tts::{
                normalization::normalize_narration_text_for_tts,
                piper_session::{
                    canonicalize_piper_voice_selection, discover_piper_french_voice_selections,
                },
            },
            live::dto::LiveNewsItem,
        },
    };
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };
    use time::{OffsetDateTime, format_description::well_known::Rfc3339};

    #[test]
    fn next_audio_stream_id_uses_the_expected_prefix() {
        // Le prefixe stable aide le debug et evite les collisions avec d'autres ids applicatifs.
        let stream_id = next_audio_stream_id();

        assert!(stream_id.starts_with("audio-"));
    }

    #[test]
    fn live_clip_body_text_does_not_repeat_the_narration_prefix() {
        // Le clip progressif lit deja `narration_prefix` dans l'intro: le corps doit contenir seulement la news.
        let mut item = test_live_item(11, "2026-05-04T19:21:46Z");
        item.narration_text = "21:21. Information importante. Titre BCE.".to_owned();
        item.translated_title = "Titre BCE".to_owned();
        item.translated_body = Some("Detail traduit".to_owned());

        let body_text = build_live_clip_body_text(&item);

        assert_eq!(body_text, "Titre BCE. Detail traduit.");
        assert!(!body_text.contains("21:21"));
        assert!(!body_text.contains("Information importante"));
    }

    #[test]
    fn discover_piper_french_voices_filters_non_french_and_invalid_assets() {
        // On fabrique ici un petit catalogue local pour verrouiller la discovery sans dependre
        // d'un vrai modele Piper telecharge.
        let root = std::env::temp_dir().join(format!(
            "filfinance-piper-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock should be monotonic enough for tests")
                .as_nanos()
        ));
        fs::create_dir_all(root.join("fr_FR-upmc-medium"))
            .expect("fr voice directory should be created");
        fs::write(root.join("fr_FR-upmc-medium/model.onnx"), b"model")
            .expect("fr voice model should be created");
        fs::write(
            root.join("fr_FR-upmc-medium/model.onnx.json"),
            br#"{"num_speakers":2,"speaker_id_map":{"jessica":0,"pierre":1}}"#,
        )
        .expect("fr voice config should be created");
        fs::create_dir_all(root.join("fr_FR-tom-medium"))
            .expect("single speaker fr voice directory should be created");
        fs::write(root.join("fr_FR-tom-medium/model.onnx"), b"model")
            .expect("single speaker fr voice model should be created");
        fs::write(
            root.join("fr_FR-tom-medium/model.onnx.json"),
            br#"{"num_speakers":1,"speaker_id_map":{}}"#,
        )
        .expect("single speaker fr voice config should be created");
        fs::create_dir_all(root.join("en_US-lessac-medium"))
            .expect("en voice directory should be created");
        fs::write(root.join("en_US-lessac-medium/model.onnx"), b"model")
            .expect("en voice model should be created");
        fs::write(root.join("en_US-lessac-medium/model.onnx.json"), b"{}")
            .expect("en voice config should be created");
        fs::create_dir_all(root.join("fr_FR-broken"))
            .expect("broken fr voice directory should be created");
        fs::write(root.join("fr_FR-broken/model.onnx"), b"model")
            .expect("broken fr voice model should be created");

        let voices = discover_piper_french_voice_selections(root.to_string_lossy().as_ref())
            .expect("voice scan ok")
            .into_iter()
            .map(|selection| selection.selection_id)
            .collect::<Vec<_>>();

        assert_eq!(
            voices,
            vec![
                "fr_FR-tom-medium",
                "fr_FR-upmc-medium-jessica",
                "fr_FR-upmc-medium-pierre",
            ]
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn canonicalize_piper_voice_selection_maps_legacy_model_id_to_first_speaker() {
        let root = std::env::temp_dir().join(format!(
            "filfinance-piper-legacy-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock should be monotonic enough for tests")
                .as_nanos()
        ));
        fs::create_dir_all(root.join("fr_FR-upmc-medium"))
            .expect("legacy fr voice directory should be created");
        fs::write(root.join("fr_FR-upmc-medium/model.onnx"), b"model")
            .expect("legacy fr voice model should be created");
        fs::write(
            root.join("fr_FR-upmc-medium/model.onnx.json"),
            br#"{"num_speakers":2,"speaker_id_map":{"jessica":0,"pierre":1}}"#,
        )
        .expect("legacy fr voice config should be created");

        let voice = canonicalize_piper_voice_selection(
            root.to_string_lossy().as_ref(),
            "fr_FR-upmc-medium",
        )
        .expect("legacy voice should canonicalize");

        assert_eq!(voice, "fr_FR-upmc-medium-jessica");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn normalize_narration_text_for_tts_expands_finance_tokens() {
        // Cette fixture couvre les tokens usuels du live macro: heure, paire FX, variation et ticker.
        let normalized =
            normalize_narration_text_for_tts("09:30. EUR/USD monte de +3% après CPI pour NVDA.");

        // Les assertions verifient uniquement le texte parle: le titre affiche reste hors de ce helper.
        assert!(normalized.contains("neuf heures trente."));
        assert!(normalized.contains("euro dollar"));
        assert!(normalized.contains("plus 3 pour cent"));
        assert!(normalized.contains("C P I"));
        assert!(normalized.contains("N V D A"));
    }

    #[test]
    fn normalize_narration_text_for_tts_handles_macro_rates_fixtures() {
        // Cette phrase force les regles multi-tokens: `+25 bps` et `US 10Y`.
        let normalized = normalize_narration_text_for_tts(
            "14:30. Fed signale +25 bps; US 10Y recule avant ECB et BCE.",
        );

        // On verrouille ici les formes qui evitent les lectures literales de Piper.
        assert!(normalized.contains("quatorze heures trente."));
        assert!(normalized.contains("la Fed"));
        assert!(normalized.contains("plus 25 points de base;"));
        assert!(normalized.contains("Treasury américain à 10 ans"));
        assert!(normalized.contains("Banque centrale européenne et Banque centrale européenne."));
    }

    #[test]
    fn normalize_narration_text_for_tts_does_not_duplicate_fed_article() {
        // Une traduction peut deja contenir l'article francais avant `Fed`.
        let normalized = normalize_narration_text_for_tts("La Fed attend le CPI.");

        // Le normaliseur ajoute l'article seulement quand il manque, sans casser les sigles restants.
        assert!(normalized.contains("La Fed"));
        assert!(!normalized.contains("La la Fed"));
        assert!(normalized.contains("C P I."));
    }

    #[test]
    fn normalize_narration_text_for_tts_v2_handles_macro_fixtures() {
        // La v2 evite les lectures trop mecaniques sur les titres macro frequents.
        let normalized = normalize_narration_text_for_tts(
            "15:45. NFP surprend, EUR/USD gagne +1.5% et NVDA recule.",
        );

        // Les cas ci-dessous representent le contrat de diction sans changer la voix Piper.
        assert!(normalized.contains("quinze heures quarante cinq."));
        assert!(normalized.contains("rapport emploi américain"));
        assert!(normalized.contains("euro dollar"));
        assert!(normalized.contains("plus 1 virgule 5 pour cent"));
        assert!(normalized.contains("N V D A"));
    }

    #[test]
    fn tts_runtime_rejects_live_audio_items_older_than_watermark() {
        let runtime = TtsRuntimeState::new(&test_tts_config());
        let first = test_live_item(11, "2026-05-04T10:05:00Z");
        let older_timestamp = test_live_item(10, "2026-05-04T10:03:00Z");
        let older_id_same_timestamp = test_live_item(9, "2026-05-04T10:05:00Z");
        let newer_id_same_timestamp = test_live_item(12, "2026-05-04T10:05:00Z");
        let newer_timestamp = test_live_item(13, "2026-05-04T10:06:00Z");
        let older_with_offset = test_live_item(14, "2026-05-04T12:04:00+02:00");

        // Le live audio accepte la premiere news puis avance uniquement vers un repere plus recent.
        assert!(runtime.try_accept_live_audio_item(&first));
        assert!(!runtime.try_accept_live_audio_item(&older_timestamp));
        assert!(!runtime.try_accept_live_audio_item(&older_id_same_timestamp));
        assert!(!runtime.try_accept_live_audio_item(&older_with_offset));
        assert!(runtime.try_accept_live_audio_item(&newer_id_same_timestamp));
        assert!(runtime.try_accept_live_audio_item(&newer_timestamp));
    }

    #[test]
    fn tts_runtime_rejects_future_live_audio_items_without_poisoning_watermark() {
        let runtime = TtsRuntimeState::new(&test_tts_config());
        let now = OffsetDateTime::parse("2026-05-04T10:00:00Z", &Rfc3339)
            .expect("fixed timestamp should parse");
        let future = test_live_item(20, "2026-05-04T10:10:00Z");
        let current = test_live_item(21, "2026-05-04T10:01:00Z");

        assert!(!runtime.try_accept_live_audio_item_at(&future, now));
        assert!(runtime.try_accept_live_audio_item_at(&current, now));
    }

    #[test]
    fn tts_runtime_repairs_legacy_future_live_audio_watermark() {
        let runtime = TtsRuntimeState::new(&test_tts_config());
        let now = OffsetDateTime::parse("2026-05-04T10:00:00Z", &Rfc3339)
            .expect("fixed timestamp should parse");
        let current = test_live_item(21, "2026-05-04T10:01:00Z");
        let older_after_repair = test_live_item(22, "2026-05-04T09:59:00Z");

        runtime.force_live_audio_watermark_for_test("2026-05-04T10:10:00Z", 20);

        assert!(runtime.try_accept_live_audio_item_at(&current, now));
        assert!(!runtime.try_accept_live_audio_item_at(&older_after_repair, now));
    }

    #[test]
    fn tts_runtime_snapshot_tracks_voice_and_generation_metrics() {
        let runtime = TtsRuntimeState::new(&test_tts_config());

        runtime.record_received();
        runtime.record_generated("audio-1", 320, 48_000, 2);
        let snapshot = runtime.snapshot();

        assert_eq!(snapshot.provider, "piper_local_session");
        assert_eq!(snapshot.current_voice, "fr_FR-upmc-medium-jessica");
        assert_eq!(snapshot.received_total, 1);
        assert_eq!(snapshot.generated_total, 1);
        assert_eq!(snapshot.pending_total, 0);
        assert_eq!(snapshot.last_generation_duration_ms, Some(320));
        assert_eq!(snapshot.generation_duration_p50_ms, Some(320));
        assert_eq!(snapshot.generation_duration_p95_ms, Some(320));
        assert_eq!(snapshot.last_total_bytes, Some(48_000));
        assert_eq!(snapshot.last_chunk_count, Some(2));
        assert_eq!(snapshot.last_stream_id.as_deref(), Some("audio-1"));
        assert!(snapshot.voice_changed_at.is_none());
    }

    fn test_tts_config() -> TtsConfig {
        // La config de test reste minimale mais complete pour initialiser le runtime sans I/O.
        TtsConfig {
            enabled: true,
            provider: "piper_local_session".to_owned(),
            voice: "fr_FR-upmc-medium-jessica".to_owned(),
            local_command_program: "python3".to_owned(),
            local_command_args: vec!["scripts/piper_local_session.py".to_owned()],
            models_dir: "data/models/piper".to_owned(),
            worker_count: 1,
            rate_words_per_minute: 185,
            output_sample_rate_hz: 22_050,
            command_timeout_ms: 12_000,
            warmup_enabled: true,
            warmup_text: "Test audio.".to_owned(),
        }
    }

    fn test_live_item(id: i64, timestamp: &str) -> LiveNewsItem {
        // Seuls `id` et `timestamp` pilotent la watermark, les autres champs gardent le DTO valide.
        LiveNewsItem {
            id,
            news_event_id: 100 + id,
            language: "fr".to_owned(),
            status: "ready".to_owned(),
            source: "financial_juice".to_owned(),
            event_kind: "headline".to_owned(),
            source_title: "ECB headline".to_owned(),
            source_body: None,
            translated_title: "Titre BCE".to_owned(),
            translated_body: None,
            narration_text: "16:42. Information importante. Titre BCE.".to_owned(),
            translation_mode: "opus_mt_tc_big_local_session".to_owned(),
            error_message: None,
            news_level: Some("active".to_owned()),
            test_run_id: None,
            url: Some("https://example.test/news".to_owned()),
            timestamp: timestamp.to_owned(),
            created_at: "2026-05-04T10:00:00Z".to_owned(),
            updated_at: "2026-05-04T10:00:00Z".to_owned(),
        }
    }
}

mod hls_origin_tests {
    // Les tests verifient les helpers purs sans exiger `ffmpeg`.
    use crate::features::audio::hls::origin::{
        HLS_REPLAY_TODAY_PLAYLIST_URL, HlsClip, HlsClipSegment, HlsOriginState, HlsSegment,
        current_local_day, estimate_wav_duration_secs, local_day_for_rfc3339, local_day_for_utc,
        now_unix_ms, parse_clip_segment_name, parse_segment_sequence,
    };
    // `header` fournit les noms standards verifies dans les assertions.
    use axum::http::header;
    // `AudioStartPayload` sert a verifier les methodes publiques de publication clip sans lancer Piper.
    use crate::features::{
        audio::domain::AudioStartPayload,
        audio::hls::http::{
            CLOUDFLARE_CDN_CACHE_CONTROL_HEADER, HLS_CLIP_PLAYLIST_FINAL_CACHE_CONTROL,
            HLS_CLIP_PLAYLIST_FINAL_CDN_CACHE_CONTROL, HLS_CLIP_PLAYLIST_MUTABLE_CACHE_CONTROL,
            HLS_CLIP_PLAYLIST_MUTABLE_CDN_CACHE_CONTROL, HLS_PLAYLIST_CACHE_CONTROL,
            HLS_PLAYLIST_CDN_CACHE_CONTROL, HLS_PLAYLIST_CONTENT_TYPE, HLS_SEGMENT_CACHE_CONTROL,
            HLS_SEGMENT_CDN_CACHE_CONTROL, HLS_SEGMENT_CONTENT_TYPE,
            clip_playlist_response_headers, playlist_response_headers, segment_response_headers,
        },
        audio::hls::playlist::{
            playlist_target_duration, render_calendar_summary_playlist, render_clip_playlist,
        },
    };
    // `VecDeque` permet d'injecter un replay sans passer par l'encodage.
    use std::{collections::VecDeque, path::PathBuf};
    use time::{OffsetDateTime, format_description::well_known::Rfc3339};

    #[test]
    fn hls_request_counters_are_exposed_in_runtime_snapshot() {
        // Les compteurs runtime permettent de verifier un idle sans parser les logs applicatifs.
        let origin = HlsOriginState::from_config(&crate::config::HlsConfig::disabled_for_tests());

        origin.record_clip_playlist_request();
        origin.record_clip_playlist_request();
        origin.record_clip_segment_request();
        origin.record_replay_playlist_request();
        origin.record_replay_segment_request();

        let snapshot = origin.snapshot();

        assert_eq!(snapshot.clip_playlist_requests_total, 2);
        assert_eq!(snapshot.clip_segment_requests_total, 1);
        assert_eq!(snapshot.replay_playlist_requests_total, 1);
        assert_eq!(snapshot.replay_segment_requests_total, 1);
    }

    #[test]
    fn hls_local_day_uses_europe_paris_product_day() {
        // Le resume du jour doit basculer a minuit Paris, meme si le conteneur tourne en UTC.
        let summer_after_paris_midnight = parse_rfc3339("2026-05-15T22:30:00Z");
        let winter_after_paris_midnight = parse_rfc3339("2026-01-15T23:30:00Z");
        let before_paris_midnight = parse_rfc3339("2026-05-15T21:30:00Z");

        assert_eq!(local_day_for_utc(summer_after_paris_midnight), "2026-05-16");
        assert_eq!(local_day_for_utc(winter_after_paris_midnight), "2026-01-16");
        assert_eq!(local_day_for_utc(before_paris_midnight), "2026-05-15");
        assert_eq!(
            local_day_for_rfc3339("2026-05-15T22:30:00Z"),
            Some("2026-05-16".to_owned())
        );
    }

    #[test]
    fn replay_today_playlist_exposes_all_current_day_segments() {
        // Le replay doit couvrir tout le jour courant, sans fenetre glissante.
        let current_day = current_local_day();
        // Le jour ancien est choisi differemment si le test tourne un 2 mai.
        let old_day = non_current_test_day(&current_day);
        let origin = HlsOriginState::from_config(&crate::config::HlsConfig {
            enabled: true,
            access_control_enabled: false,
            access_ticket_secret: None,
            access_ticket_ttl_secs: 3600,
            segments_dir: "data/hls-test".to_owned(),
            ffmpeg_command: "ffmpeg".to_owned(),
            replay_segment_target_duration_secs: 4,
            clip_segment_target_duration_secs: 2,
        });

        {
            let mut inner = origin.inner.lock().expect("hls test mutex poisoned");
            inner.next_sequence = 4;
            inner.segments = VecDeque::from(vec![
                test_segment_for_day(1, 4.0, old_day),
                test_segment_for_day(2, 4.0, &current_day),
                test_segment_for_day(3, 2.5, &current_day),
            ]);
        }

        let playlist = origin
            .replay_today_playlist(None)
            .expect("replay playlist should be generated");

        assert!(playlist.contains("#EXT-X-PLAYLIST-TYPE:EVENT"));
        assert!(playlist.contains("#EXT-X-START:TIME-OFFSET=0,PRECISE=YES"));
        assert!(!playlist.contains(&format!("/audio/replay/today/segments/{old_day}/1.ts")));
        assert!(playlist.contains(&format!("/audio/replay/today/segments/{current_day}/2.ts")));
        assert!(playlist.contains(&format!("/audio/replay/today/segments/{current_day}/3.ts")));
    }

    #[test]
    fn replay_today_playlist_can_start_at_first_requested_translation() {
        // Le resume audio du jour doit pouvoir se caler sur la premiere news affichee par le client.
        let current_day = current_local_day();
        let origin = HlsOriginState::from_config(&crate::config::HlsConfig {
            enabled: true,
            access_control_enabled: false,
            access_ticket_secret: None,
            access_ticket_ttl_secs: 3600,
            segments_dir: "data/hls-test".to_owned(),
            ffmpeg_command: "ffmpeg".to_owned(),
            replay_segment_target_duration_secs: 4,
            clip_segment_target_duration_secs: 2,
        });

        {
            let mut inner = origin.inner.lock().expect("hls test mutex poisoned");
            inner.next_sequence = 5;
            inner.segments = VecDeque::from(vec![
                test_segment_for_translation(1, 4.0, &current_day, 10),
                test_segment_for_translation(2, 4.0, &current_day, 20),
                test_segment_for_translation(3, 3.0, &current_day, 20),
                test_segment_for_translation(4, 2.0, &current_day, 30),
            ]);
        }

        let playlist = origin
            .replay_today_playlist(Some(20))
            .expect("replay playlist should be generated from requested translation");

        assert!(playlist.contains("#EXT-X-MEDIA-SEQUENCE:2"));
        assert!(!playlist.contains(&format!("/audio/replay/today/segments/{current_day}/1.ts")));
        assert!(playlist.contains(&format!("/audio/replay/today/segments/{current_day}/2.ts")));
        assert!(playlist.contains(&format!("/audio/replay/today/segments/{current_day}/3.ts")));
        assert!(playlist.contains(&format!("/audio/replay/today/segments/{current_day}/4.ts")));
    }

    #[test]
    fn replay_today_playlist_orders_backfilled_segments_by_news_timestamp() {
        // Un rattrapage audio tardif d'une news plus ancienne doit rester lu avant les news recentes.
        let current_day = current_local_day();
        let origin = HlsOriginState::from_config(&crate::config::HlsConfig {
            enabled: true,
            access_control_enabled: false,
            access_ticket_secret: None,
            access_ticket_ttl_secs: 3600,
            segments_dir: "data/hls-test".to_owned(),
            ffmpeg_command: "ffmpeg".to_owned(),
            replay_segment_target_duration_secs: 4,
            clip_segment_target_duration_secs: 2,
        });

        let mut newer = test_segment_for_translation(1, 4.0, &current_day, 20);
        newer.timestamp = Some(format!("{current_day}T02:08:00Z"));
        let mut older_backfill = test_segment_for_translation(2, 4.0, &current_day, 10);
        older_backfill.timestamp = Some(format!("{current_day}T00:31:00Z"));

        {
            let mut inner = origin.inner.lock().expect("hls test mutex poisoned");
            inner.next_sequence = 3;
            inner.segments = VecDeque::from(vec![newer, older_backfill]);
        }

        let playlist = origin
            .replay_today_playlist(Some(10))
            .expect("replay playlist should be ordered by news timestamp");

        let older_position = playlist
            .find(&format!("/audio/replay/today/segments/{current_day}/2.ts"))
            .expect("older backfill segment should be present");
        let newer_position = playlist
            .find(&format!("/audio/replay/today/segments/{current_day}/1.ts"))
            .expect("newer segment should be present");

        assert!(older_position < newer_position);
    }

    #[test]
    fn replay_today_playlist_keeps_full_replay_when_legacy_segments_precede_match() {
        // Apres redemarrage, les segments reconstruits depuis disque n'ont pas de metadata de traduction.
        let current_day = current_local_day();
        let origin = HlsOriginState::from_config(&crate::config::HlsConfig {
            enabled: true,
            access_control_enabled: false,
            access_ticket_secret: None,
            access_ticket_ttl_secs: 3600,
            segments_dir: "data/hls-test".to_owned(),
            ffmpeg_command: "ffmpeg".to_owned(),
            replay_segment_target_duration_secs: 4,
            clip_segment_target_duration_secs: 2,
        });

        {
            let mut inner = origin.inner.lock().expect("hls test mutex poisoned");
            inner.next_sequence = 4;
            inner.segments = VecDeque::from(vec![
                test_segment_for_day(1, 4.0, &current_day),
                test_segment_for_translation(2, 4.0, &current_day, 20),
                test_segment_for_translation(3, 4.0, &current_day, 30),
            ]);
        }

        let playlist = origin
            .replay_today_playlist(Some(10))
            .expect("replay playlist should keep legacy prefix");

        assert!(playlist.contains("#EXT-X-MEDIA-SEQUENCE:1"));
        assert!(playlist.contains(&format!("/audio/replay/today/segments/{current_day}/1.ts")));
        assert!(playlist.contains(&format!("/audio/replay/today/segments/{current_day}/2.ts")));
        assert!(playlist.contains(&format!("/audio/replay/today/segments/{current_day}/3.ts")));
    }

    #[test]
    fn replay_today_metadata_reports_current_day_availability() {
        // La metadata API doit indiquer clairement si le replay du jour est disponible.
        let current_day = current_local_day();
        // Le jour ancien permet de verifier que la metadata ignore les segments hors jour courant.
        let old_day = non_current_test_day(&current_day);
        let origin = HlsOriginState::from_config(&crate::config::HlsConfig {
            enabled: true,
            access_control_enabled: false,
            access_ticket_secret: None,
            access_ticket_ttl_secs: 3600,
            segments_dir: "data/hls-test".to_owned(),
            ffmpeg_command: "ffmpeg".to_owned(),
            replay_segment_target_duration_secs: 4,
            clip_segment_target_duration_secs: 2,
        });

        {
            let mut inner = origin.inner.lock().expect("hls test mutex poisoned");
            inner.next_sequence = 4;
            inner.segments = VecDeque::from(vec![
                test_segment_for_day(1, 4.0, old_day),
                test_segment_for_day(2, 4.0, &current_day),
                test_segment_for_day(3, 2.5, &current_day),
            ]);
        }

        let metadata = origin.replay_today_metadata();

        assert!(metadata.enabled);
        assert!(metadata.available);
        assert_eq!(metadata.local_day, current_day);
        assert_eq!(metadata.playlist_url, HLS_REPLAY_TODAY_PLAYLIST_URL);
        assert_eq!(metadata.segment_count, 2);
        assert_eq!(metadata.first_sequence, Some(2));
        assert_eq!(metadata.latest_sequence, Some(3));
    }

    #[tokio::test]
    async fn replay_today_segment_rejects_non_current_day() {
        // Le replay ne doit pas servir un segment de la veille meme si l'URL est devinee.
        let current_day = current_local_day();
        // Le jour ancien reste distinct du jour courant quelle que soit la date de test.
        let old_day = non_current_test_day(&current_day);
        let origin = HlsOriginState::from_config(&crate::config::HlsConfig {
            enabled: true,
            access_control_enabled: false,
            access_ticket_secret: None,
            access_ticket_ttl_secs: 3600,
            segments_dir: "data/hls-test".to_owned(),
            ffmpeg_command: "ffmpeg".to_owned(),
            replay_segment_target_duration_secs: 4,
            clip_segment_target_duration_secs: 2,
        });

        let error = origin
            .replay_today_segment_bytes(old_day, "1.ts")
            .await
            .expect_err("old day replay segment should be rejected");

        assert!(error.to_string().contains("not current"));
    }

    #[tokio::test]
    async fn clip_publish_methods_are_noop_when_hls_is_disabled() {
        // NEWS-138 prepare l'API origin sans obliger tous les environnements a activer HLS.
        let origin = HlsOriginState::from_config(&crate::config::HlsConfig::disabled_for_tests());
        let start_payload = test_audio_start_payload();

        let initial = origin
            .publish_clip_intro(&start_payload, &[])
            .await
            .expect("disabled hls should not publish");
        let finalized = origin
            .append_clip_body_and_finalize(&start_payload.stream_id, &[])
            .await
            .expect("disabled hls should not append");

        assert!(!initial.hls_enabled);
        assert_eq!(initial.playlist_url, "/audio/clips/clip-test/playlist.m3u8");
        assert!(!finalized.hls_enabled);
        assert_eq!(
            finalized.playlist_url,
            "/audio/clips/clip-test/playlist.m3u8"
        );
    }

    #[tokio::test]
    async fn replay_publish_skips_items_outside_current_product_day() {
        // Une news hors jour courant rattrapee plus tard ne doit pas entrer dans le replay audio de J.
        let current_day = current_local_day();
        let non_current_day = non_current_test_day(&current_day);
        let origin = HlsOriginState::from_config(&crate::config::HlsConfig {
            enabled: true,
            access_control_enabled: false,
            access_ticket_secret: None,
            access_ticket_ttl_secs: 3600,
            segments_dir: "data/hls-test".to_owned(),
            ffmpeg_command: "ffmpeg-that-should-not-run".to_owned(),
            replay_segment_target_duration_secs: 4,
            clip_segment_target_duration_secs: 2,
        });
        let mut start_payload = test_audio_start_payload();
        start_payload.timestamp = format!("{non_current_day}T12:00:00Z");

        let metrics = origin
            .publish_replay_stream(&start_payload, b"not-a-real-wav")
            .await
            .expect("non current payload should be skipped before ffmpeg");

        assert!(metrics.hls_enabled);
        assert_eq!(metrics.segment_count, 0);
        let inner = origin.inner.lock().expect("hls test mutex poisoned");
        assert!(inner.segments.is_empty());
    }

    #[test]
    fn clip_playlist_exposes_intro_without_endlist() {
        // Le premier manifest doit etre lisible des l'intro, mais rester mutable.
        let clip = test_clip(false, vec![test_clip_segment("segment-000.ts", 1.8, false)]);

        let playlist = render_clip_playlist(&clip);

        assert!(playlist.body.contains("#EXT-X-PLAYLIST-TYPE:EVENT"));
        assert!(playlist.body.contains("#EXT-X-MEDIA-SEQUENCE:0"));
        assert!(
            playlist
                .body
                .contains("/audio/clips/clip-test/segment-000.ts")
        );
        assert!(!playlist.body.contains("#EXT-X-ENDLIST"));
        assert!(!playlist.finalized);
    }

    #[test]
    fn clip_playlist_appends_body_with_discontinuity_and_endlist() {
        // Intro et corps sont encodes separement, donc la playlist marque la transition.
        let clip = test_clip(
            true,
            vec![
                test_clip_segment("segment-000.ts", 1.8, false),
                test_clip_segment("segment-001.ts", 2.0, true),
                test_clip_segment("segment-002.ts", 1.2, false),
            ],
        );

        let playlist = render_clip_playlist(&clip);

        assert!(playlist.body.contains("#EXT-X-DISCONTINUITY"));
        assert!(
            playlist
                .body
                .contains("/audio/clips/clip-test/segment-001.ts")
        );
        assert!(playlist.body.ends_with("#EXT-X-ENDLIST\n"));
        assert!(playlist.finalized);
    }

    #[test]
    fn calendar_summary_playlist_uses_persistent_public_paths() {
        // Les resumes calendrier ont des chemins publics dedies, distincts des clips live.
        let clip = test_clip(
            true,
            vec![
                test_clip_segment("segment-000.ts", 1.8, false),
                test_clip_segment("segment-001.ts", 2.1, false),
            ],
        );

        let playlist = render_calendar_summary_playlist(&clip, "2026-06-15", "audio-abc");

        assert!(playlist.finalized);
        assert!(playlist.body.contains("#EXT-X-PLAYLIST-TYPE:VOD"));
        assert!(playlist.body.ends_with("#EXT-X-ENDLIST\n"));
        assert!(
            playlist
                .body
                .contains("/audio/calendar-summary/2026-06-15/audio-abc/segment-000.ts")
        );
        assert!(
            playlist
                .body
                .contains("/audio/calendar-summary/2026-06-15/audio-abc/segment-001.ts")
        );
    }

    #[tokio::test]
    async fn calendar_summary_manifest_is_persisted_current_and_pruned_by_day() {
        // Une generation de resume doit conserver un manifest immutable et un pointeur current atomique.
        let root =
            std::env::temp_dir().join(format!("filfinance-calendar-summary-hls-{}", now_unix_ms()));
        let source_dir = root.join("source");
        tokio::fs::create_dir_all(&source_dir)
            .await
            .expect("source dir should be created");
        let segment_0 = source_dir.join("segment-000.ts");
        let segment_1 = source_dir.join("segment-001.ts");
        tokio::fs::write(&segment_0, b"segment-zero")
            .await
            .expect("segment 0 should be written");
        tokio::fs::write(&segment_1, b"segment-one")
            .await
            .expect("segment 1 should be written");
        let hls_config = crate::config::HlsConfig {
            enabled: true,
            access_control_enabled: false,
            access_ticket_secret: None,
            access_ticket_ttl_secs: 3600,
            segments_dir: root.to_string_lossy().to_string(),
            ffmpeg_command: "ffmpeg".to_owned(),
            replay_segment_target_duration_secs: 4,
            clip_segment_target_duration_secs: 2,
        };
        let origin = HlsOriginState::from_config(&hls_config);
        let stream_id = "calendar-summary-test-stream";
        {
            let mut inner = origin.inner.lock().expect("hls test mutex poisoned");
            inner.clips.insert(
                stream_id.to_owned(),
                HlsClip {
                    stream_id: stream_id.to_owned(),
                    target_duration_secs: 2,
                    segments: vec![
                        HlsClipSegment {
                            name: "segment-000.ts".to_owned(),
                            duration_secs: 1.8,
                            file_path: segment_0.clone(),
                            discontinuity_before: false,
                        },
                        HlsClipSegment {
                            name: "segment-001.ts".to_owned(),
                            duration_secs: 2.0,
                            file_path: segment_1.clone(),
                            discontinuity_before: false,
                        },
                    ],
                    finalized: true,
                    durable_recovery: true,
                    updated_at_ms: now_unix_ms(),
                },
            );
        }

        let manifest = origin
            .persist_calendar_summary_clip("2026-06-15", stream_id)
            .await
            .expect("calendar summary clip should be persisted");

        // Un crash ou une purge partielle peut laisser les segments du stream
        // reserve intacts mais enlever les deux manifests. La reparation doit
        // alors reutiliser ces segments et reconstruire current + immutable,
        // sans essayer de les recopier une seconde fois.
        let summary_dir = root
            .join("calendar-summary")
            .join("2026-06-15")
            .join(stream_id);
        let current_path = root
            .join("calendar-summary")
            .join("2026-06-15")
            .join("current.m3u8");
        tokio::fs::remove_file(&current_path)
            .await
            .expect("current manifest should be removable for repair");
        tokio::fs::remove_file(summary_dir.join("playlist.m3u8"))
            .await
            .expect("immutable manifest should be removable for repair");
        assert!(!origin.has_calendar_summary_current_manifest("2026-06-15"));
        assert!(!origin.has_calendar_summary_manifest("2026-06-15", stream_id));

        let repaired = origin
            .restore_calendar_summary_manifests_from_existing_clip("2026-06-15", stream_id)
            .await
            .expect("manifest reconstruction should not fail on existing segments")
            .expect("the finalized reserved clip should be reusable");

        assert_eq!(manifest.stream_id, stream_id);
        assert_eq!(repaired.stream_id, stream_id);
        assert_eq!(
            manifest.playlist_path,
            "calendar-summary/2026-06-15/calendar-summary-test-stream/playlist.m3u8"
        );
        assert_eq!(
            manifest.immutable_playlist_url,
            "/audio/calendar-summary/2026-06-15/calendar-summary-test-stream/playlist.m3u8"
        );
        assert_eq!(
            manifest.current_playlist_url,
            "/audio/calendar-summary/2026-06-15/current.m3u8"
        );
        assert!(origin.has_calendar_summary_current_manifest("2026-06-15"));
        assert!(origin.has_calendar_summary_manifest("2026-06-15", stream_id));
        let current = origin
            .calendar_summary_playlist("2026-06-15", None)
            .await
            .expect("current playlist should be readable");
        let immutable = origin
            .calendar_summary_playlist("2026-06-15", Some(stream_id))
            .await
            .expect("immutable playlist should be readable");
        assert_eq!(current, immutable);
        assert!(current.contains("#EXT-X-ENDLIST"));

        // Apres redemarrage, l'index memoire des clips est vide, mais
        // l'immuable suffit a recreer le pointeur stable attendu par le
        // client sans relancer Piper ni produire un second stream.
        tokio::fs::remove_file(&current_path)
            .await
            .expect("current manifest should be removable after restart");
        let restarted_origin = HlsOriginState::from_config(&hls_config);
        let restored_after_restart = restarted_origin
            .restore_calendar_summary_manifests_from_existing_clip("2026-06-15", stream_id)
            .await
            .expect("immutable manifest should restore current after restart")
            .expect("existing immutable calendar manifest should be reusable");
        assert_eq!(restored_after_restart.stream_id, stream_id);
        assert!(restarted_origin.has_calendar_summary_current_manifest("2026-06-15"));
        assert_eq!(
            restarted_origin
                .calendar_summary_playlist("2026-06-15", None)
                .await
                .expect("restored current playlist should be readable"),
            immutable
        );
        let segment_bytes = origin
            .calendar_summary_segment_bytes("2026-06-15", stream_id, "segment-000.ts")
            .await
            .expect("persisted segment should be readable");
        assert_eq!(segment_bytes, b"segment-zero");

        tokio::fs::create_dir_all(root.join("calendar-summary").join("2026-06-08"))
            .await
            .expect("old day dir should be created");
        let deleted = origin
            .prune_calendar_summary_manifests(&["2026-06-15".to_owned()])
            .await
            .expect("calendar summary hls retention should prune old day");

        assert_eq!(deleted, 1);
        assert!(origin.has_calendar_summary_current_manifest("2026-06-15"));
        assert!(!root.join("calendar-summary").join("2026-06-08").exists());

        let _ = tokio::fs::remove_dir_all(&root).await;
    }

    #[test]
    fn clip_segment_parser_rejects_path_traversal() {
        // Les segments clip ont un prefixe explicite et ne peuvent pas pointer hors dossier.
        assert_eq!(
            parse_clip_segment_name("segment-000.ts").expect("valid clip segment"),
            "segment-000.ts"
        );
        assert!(parse_clip_segment_name("../segment-000.ts").is_err());
        assert!(parse_clip_segment_name("000.ts").is_err());
    }

    #[test]
    fn segment_sequence_parser_rejects_path_traversal() {
        // Le handler ne doit accepter qu'un nom plat numerique.
        assert_eq!(parse_segment_sequence("42.ts").expect("valid sequence"), 42);
        assert!(parse_segment_sequence("../42.ts").is_err());
        assert!(parse_segment_sequence("42").is_err());
    }

    #[test]
    fn playlist_target_duration_covers_longest_segment() {
        // HLS impose une target duration au moins egale au plus long `EXTINF` arrondi.
        let target = playlist_target_duration(4, [3.2, 6.1, 4.0].into_iter());
        assert_eq!(target, 7);
    }

    #[test]
    fn wav_duration_estimator_reads_pcm_header() {
        // Ce WAV minimal annonce 1 seconde de PCM mono 16 bits a 22050 Hz.
        let bytes = minimal_wav_header(22_050, 44_100);
        let duration = estimate_wav_duration_secs(&bytes).expect("duration should be estimated");
        assert!((duration - 1.0).abs() < 0.001);
    }

    #[test]
    fn playlist_headers_keep_replay_manifest_fresh_at_browser_and_cdn() {
        // Le manifest replay ne doit pas etre garde longtemps par le navigateur ni par Cloudflare.
        let headers = playlist_response_headers();

        assert_eq!(
            headers
                .get(header::CONTENT_TYPE)
                .expect("content type should exist"),
            HLS_PLAYLIST_CONTENT_TYPE
        );
        assert_eq!(
            headers
                .get(header::CACHE_CONTROL)
                .expect("cache control should exist"),
            HLS_PLAYLIST_CACHE_CONTROL
        );
        assert_eq!(
            headers
                .get(CLOUDFLARE_CDN_CACHE_CONTROL_HEADER)
                .expect("cloudflare cache control should exist"),
            HLS_PLAYLIST_CDN_CACHE_CONTROL
        );
    }

    #[test]
    fn clip_playlist_headers_distinguish_mutable_and_final_manifests() {
        // Le manifest mutable doit etre revalide, le manifest final peut etre cache plus longtemps.
        let mutable_headers = clip_playlist_response_headers(false);
        let final_headers = clip_playlist_response_headers(true);

        assert_eq!(
            mutable_headers
                .get(header::CACHE_CONTROL)
                .expect("mutable cache control should exist"),
            HLS_CLIP_PLAYLIST_MUTABLE_CACHE_CONTROL
        );
        assert_eq!(
            final_headers
                .get(header::CACHE_CONTROL)
                .expect("final cache control should exist"),
            HLS_CLIP_PLAYLIST_FINAL_CACHE_CONTROL
        );
        assert_eq!(
            mutable_headers
                .get(CLOUDFLARE_CDN_CACHE_CONTROL_HEADER)
                .expect("mutable cdn cache control should exist"),
            HLS_CLIP_PLAYLIST_MUTABLE_CDN_CACHE_CONTROL
        );
        assert_eq!(
            final_headers
                .get(CLOUDFLARE_CDN_CACHE_CONTROL_HEADER)
                .expect("final cdn cache control should exist"),
            HLS_CLIP_PLAYLIST_FINAL_CDN_CACHE_CONTROL
        );
    }

    #[test]
    fn segment_headers_make_sequence_files_immutable_for_cdn() {
        // Les segments numerotes sont immuables et peuvent donc etre caches longtemps.
        let headers = segment_response_headers();

        assert_eq!(
            headers
                .get(header::CONTENT_TYPE)
                .expect("content type should exist"),
            HLS_SEGMENT_CONTENT_TYPE
        );
        assert_eq!(
            headers
                .get(header::CACHE_CONTROL)
                .expect("cache control should exist"),
            HLS_SEGMENT_CACHE_CONTROL
        );
        assert_eq!(
            headers
                .get(CLOUDFLARE_CDN_CACHE_CONTROL_HEADER)
                .expect("cloudflare cache control should exist"),
            HLS_SEGMENT_CDN_CACHE_CONTROL
        );
    }

    fn test_segment_for_day(sequence: u64, duration_secs: f64, local_day: &str) -> HlsSegment {
        // Le helper parametrable sert aux tests replay qui doivent isoler le jour courant.
        HlsSegment {
            sequence,
            duration_secs,
            file_path: PathBuf::from(format!("data/hls-test/{sequence}.ts")),
            local_day: local_day.to_owned(),
            translation_id: None,
            news_event_id: None,
            stream_id: None,
            timestamp: None,
        }
    }

    fn test_segment_for_translation(
        sequence: u64,
        duration_secs: f64,
        local_day: &str,
        translation_id: i64,
    ) -> HlsSegment {
        let mut segment = test_segment_for_day(sequence, duration_secs, local_day);
        segment.translation_id = Some(translation_id);
        segment.news_event_id = Some(translation_id + 1_000);
        segment.stream_id = Some(format!("audio-{translation_id}"));
        segment.timestamp = Some(format!("{local_day}T00:{translation_id:02}:00Z"));
        segment
    }

    fn test_clip(finalized: bool, segments: Vec<HlsClipSegment>) -> HlsClip {
        // Les tests playlist se concentrent sur le rendu HLS sans lancer `ffmpeg`.
        HlsClip {
            stream_id: "clip-test".to_owned(),
            target_duration_secs: 2,
            segments,
            finalized,
            durable_recovery: false,
            updated_at_ms: now_unix_ms(),
        }
    }

    fn test_clip_segment(
        name: &str,
        duration_secs: f64,
        discontinuity_before: bool,
    ) -> HlsClipSegment {
        // Le chemin disque est fictif: seul le nom public entre dans la playlist.
        HlsClipSegment {
            name: name.to_owned(),
            duration_secs,
            file_path: PathBuf::from(format!("data/hls-test/clips/clip-test/{name}")),
            discontinuity_before,
        }
    }

    fn test_audio_start_payload() -> AudioStartPayload {
        // Le payload minimal garde les correlations attendues par les methodes publiques clip.
        AudioStartPayload {
            stream_id: "clip-test".to_owned(),
            translation_id: 42,
            news_event_id: 24,
            language: "fr".to_owned(),
            source: "financial_juice".to_owned(),
            event_kind: "news".to_owned(),
            source_title: "ECB keeps rates unchanged".to_owned(),
            translated_title: "La BCE maintient ses taux inchanges".to_owned(),
            test_run_id: None,
            timestamp: "2026-05-04T14:42:00Z".to_owned(),
            voice: "fr_FR-upmc-medium-pierre".to_owned(),
            audio_format: "wav".to_owned(),
            mime_type: "audio/wav".to_owned(),
            total_bytes: 0,
            chunk_count: 0,
        }
    }

    fn non_current_test_day(current_day: &str) -> &'static str {
        // Le test garde une date stable mais evite une collision si l'horloge locale tombe dessus.
        if current_day == "2026-05-02" {
            "2026-05-01"
        } else {
            "2026-05-02"
        }
    }

    fn parse_rfc3339(value: &str) -> OffsetDateTime {
        OffsetDateTime::parse(value, &Rfc3339).expect("test timestamp should parse")
    }

    fn minimal_wav_header(sample_rate: u32, data_size: u32) -> Vec<u8> {
        let byte_rate = sample_rate * 2;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(36 + data_size).to_le_bytes());
        bytes.extend_from_slice(b"WAVE");
        bytes.extend_from_slice(b"fmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&sample_rate.to_le_bytes());
        bytes.extend_from_slice(&byte_rate.to_le_bytes());
        bytes.extend_from_slice(&2u16.to_le_bytes());
        bytes.extend_from_slice(&16u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&data_size.to_le_bytes());
        bytes.extend(std::iter::repeat_n(0, data_size as usize));
        bytes
    }
}
