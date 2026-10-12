//! Groupes de routes Axum exposes par les features.

// Router compose les routes Axum exposees par les features.
use axum::Router;
// middleware applique les protections listener/moderator.
use axum::middleware;
// get/post montent les handlers HTTP dans le routeur.
use axum::routing::{get, patch, post};

use crate::{
    app_state::AppState,
    features::{
        audio, auth, blacklist, calendar_day, glossary, health, ingestion, live, logs, news,
        translation,
    },
};

/// Routes publiques sans authentification applicative.
pub(super) fn public_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/ws/admin/calendar-day/logs",
            get(crate::features::calendar_logs::reader::websocket),
        )
        .route("/health", get(health::http::health))
        .route(
            "/audio/clips/{stream_id}/playlist.m3u8",
            get(audio::hls::clip_playlist),
        )
        .route(
            "/audio/clips/{stream_id}/{segment_name}",
            get(audio::hls::clip_segment),
        )
        .route(
            "/audio/calendar-summary/{local_day}/current.m3u8",
            get(audio::hls::calendar_summary_current_playlist),
        )
        .route(
            "/audio/calendar-summary/{local_day}/{stream_id}/playlist.m3u8",
            get(audio::hls::calendar_summary_immutable_playlist),
        )
        .route(
            "/audio/calendar-summary/{local_day}/{stream_id}/{segment_name}",
            get(audio::hls::calendar_summary_segment),
        )
        .route(
            "/audio/replay/today/playlist.m3u8",
            get(audio::hls::replay_today_playlist),
        )
        .route(
            "/audio/replay/today/segments/{local_day}/{segment_name}",
            get(audio::hls::replay_today_segment),
        )
        .route("/ws/news", get(live::websocket::ws_news))
        .route("/ws/live", get(live::websocket::ws_live))
        .route("/ws/test-runs", get(live::websocket::ws_test_runs))
        .route("/ws/admin/logs", get(logs::websocket::ws_admin_logs))
        .route(
            "/ws/admin/calendar-day",
            get(calendar_day::ws_admin_calendar_day),
        )
        .route(
            "/api/ingest/financial-juice/browser/news",
            post(ingestion::http::ingest_financial_juice_browser_news),
        )
        .route(
            "/api/ingest/financial-juice/browser/status",
            post(ingestion::http::update_financial_juice_browser_status),
        )
        .route(
            "/api/ingest/financial-juice/browser/control",
            get(ingestion::http::financial_juice_browser_control),
        )
}

/// Routes moderateur: elles exposent l'etat runtime et exigent un role applicatif fort.
pub(super) fn moderator_routes(state: &AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/api/runtime/translation",
            get(translation::http::translation_runtime),
        )
        .route("/api/runtime/tts", get(audio::tts::runtime::tts_runtime))
        .route("/api/runtime/hls", get(audio::hls::runtime::hls_runtime))
        .route(
            "/api/runtime/financial-juice",
            get(ingestion::http::financial_juice_runtime)
                .patch(ingestion::http::update_financial_juice_runtime),
        )
        .route(
            "/api/runtime/financial-juice-browser",
            get(ingestion::http::financial_juice_browser_runtime),
        )
        .route(
            "/api/runtime/x",
            get(ingestion::http::x_runtime).patch(ingestion::http::update_x_runtime),
        )
        .route("/api/admin/logs/days", get(logs::http::list_log_days))
        .route("/api/admin/logs", get(logs::http::list_logs))
        .route(
            "/api/glossary",
            get(glossary::http::list_glossary_entries).post(glossary::http::create_glossary_entry),
        )
        .route(
            "/api/glossary/system-rules",
            get(glossary::http::list_glossary_system_rules),
        )
        .route(
            "/api/glossary/preview",
            post(glossary::http::preview_glossary_text),
        )
        .route(
            "/api/glossary/reapply-translations",
            post(glossary::http::reapply_glossary_translations),
        )
        .route(
            "/api/glossary/{id}",
            patch(glossary::http::update_glossary_entry)
                .delete(glossary::http::delete_glossary_entry),
        )
        .route(
            "/api/news-blacklist",
            get(blacklist::http::list_news_blacklist_entries)
                .post(blacklist::http::create_news_blacklist_entry),
        )
        .route(
            "/api/news-blacklist/hits",
            get(blacklist::http::list_news_blacklist_hits),
        )
        .route(
            "/api/news-blacklist/reapply",
            post(blacklist::http::reapply_news_blacklist),
        )
        .route(
            "/api/news-blacklist/{id}",
            patch(blacklist::http::update_news_blacklist_entry)
                .delete(blacklist::http::delete_news_blacklist_entry),
        )
        .route(
            "/api/news-truncation-audit",
            get(blacklist::http::list_news_truncation_audits),
        )
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth::require_moderator,
        ))
}

/// Routes admin: elles exposent les details complets et switches sensibles.
pub(super) fn admin_routes(state: &AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/api/admin/calendar-day/logs/{*path}",
            axum::routing::any(crate::features::calendar_logs::reader::endpoint),
        )
        .route("/api/sources", get(news::http::list_sources))
        .route("/api/admin/health", get(health::http::admin_health))
        .route(
            "/api/admin/calendar-day/today",
            get(calendar_day::get_admin_calendar_day_today)
                .post(calendar_day::run_admin_calendar_day_today),
        )
        .route(
            "/api/admin/calendar-day",
            get(calendar_day::get_admin_calendar_day).post(calendar_day::run_admin_calendar_day),
        )
        .route(
            "/api/admin/calendar-day/run",
            post(calendar_day::run_admin_calendar_day),
        )
        .route(
            "/api/admin/calendar-week/run",
            post(calendar_day::run_admin_calendar_week),
        )
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth::require_admin,
        ))
}

/// Routes listener: elles exigent une session valide sans privilege moderateur.
pub(super) fn listener_routes(state: &AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/api/edge/live-ticket",
            get(live::edge::create_edge_live_ticket),
        )
        .route(
            "/api/hls-ticket",
            get(audio::hls::http::create_hls_access_ticket_handler),
        )
        .route("/api/replay/today", get(audio::hls::http::get_replay_today))
        .route(
            "/api/calendar-day/today",
            get(calendar_day::get_calendar_day_today),
        )
        .route("/api/calendar-day", get(calendar_day::get_calendar_day))
        .route(
            "/api/calendar-day/today/summary",
            get(calendar_day::get_calendar_day_today_summary),
        )
        .route(
            "/api/calendar-day/summary",
            get(calendar_day::get_calendar_day_summary),
        )
        .route("/api/news", get(news::http::list_news))
        .route(
            "/api/translations",
            get(translation::http::list_translations_history),
        )
        .route(
            "/api/translations/{id}",
            get(translation::http::get_translation_detail),
        )
        .route("/api/news/{id}", get(news::http::get_news))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth::require_listener,
        ))
}
