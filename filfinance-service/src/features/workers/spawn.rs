//! Wiring centralise des workers exposes par les features.

// Client est partage par les collecteurs et Telegram.
use reqwest::Client;
// mpsc apporte la queue d'ingestion et watch le signal de shutdown partage.
use tokio::sync::{mpsc, watch};

use crate::{
    app_state::AppState,
    features::audio::domain::CalendarDayAudioJob,
    features::{
        audio, calendar_day, ingestion, live, news, news::domain::NewsEvent, notifications,
        translation,
    },
};

use super::handles::RuntimeWorkerHandles;

/// Lance les workers long-vivants depuis la facade feature-first.
// Les receivers restent des arguments distincts pour rendre visible l'isolation de chaque file runtime.
// L'exception locale au lint accepte cette facade de composition explicite; regrouper les arguments masquerait
// l'ownership consommateur-unique des channels et elargirait artificiellement une structure de configuration.
#[allow(clippy::too_many_arguments)]
pub fn spawn_workers(
    state: AppState,
    ingest_rx: mpsc::Receiver<NewsEvent>,
    calendar_day_audio_rx: mpsc::Receiver<CalendarDayAudioJob>,
    // Cette queue ne contient qu'un reveil best-effort; les travaux sont dans SQLite.
    calendar_day_native_rx: mpsc::Receiver<calendar_day::native::runtime::CalendarNativeWake>,
    http_client: Client,
    shutdown_tx: &watch::Sender<bool>,
    shutdown_rx: watch::Receiver<bool>,
) -> RuntimeWorkerHandles {
    // Le pipeline est supervise: il recoit le signal avant que le bootstrap ne termine le process.
    let ingestion_pipeline_handle = tokio::spawn(ingestion::pipeline::run(
        state.clone(),
        ingest_rx,
        shutdown_tx.subscribe(),
    ));
    // Le worker de traduction peut retourner `None` s'il est desactive dans la config.
    let translation_handle = translation::spawn(state.clone(), shutdown_rx);
    // Chaque worker recoit son propre receveur `watch` pour observer le meme signal d'arret.
    let tts_handle = audio::tts::spawn(state.clone(), shutdown_tx.subscribe());
    let calendar_day_audio_handle = audio::service::spawn_calendar_day_audio_worker(
        state.clone(),
        calendar_day_audio_rx,
        shutdown_tx.subscribe(),
    );
    // Le publisher edge recoit seulement les metadata traduites et reste isole du hot path.
    let edge_handle = live::edge::spawn(state.clone(), shutdown_tx.subscribe());
    // Chaque replica DEV s'abonne au canal Redis: Bunny ne garde aucun etat WebSocket.
    let live_fanout_handle = state.live_fanout.clone().map(|fanout| {
        fanout.spawn_subscriber(state.live_metadata_tx.clone(), shutdown_tx.subscribe())
    });
    // Le worker durable est lance avant le scheduler : il reprend d'abord les
    // leases expirees puis relit SQLite, avant toute admission periodique.
    let calendar_day_native_handle = calendar_day::native::runtime::spawn(
        state.clone(),
        calendar_day_native_rx,
        shutdown_tx.subscribe(),
    );
    let calendar_day_scheduler_handle =
        calendar_day::spawn_scheduler(state.clone(), shutdown_tx.subscribe());
    let calendar_day_alert_scheduler_handle =
        calendar_day::spawn_alert_scheduler(state.clone(), shutdown_tx.subscribe());
    // La retention purge immediatement puis periodiquement les lignes sorties de J/J-1.
    let retention_handle = news::retention::spawn(state.clone(), shutdown_tx.subscribe());
    // Le fallback navigateur doit produire un etat explicite meme quand le process externe est muet.
    let browser_fallback_monitor_handle =
        ingestion::monitor::spawn_browser_fallback_monitor(state.clone(), shutdown_tx.subscribe());
    // Les collecteurs injectent leurs `NewsEvent` dans la queue canonique et restent supervises
    // afin de fermer poll, SignalR et appels source pendant la fenetre d'arret bornee.
    let collector_handles =
        ingestion::collectors::spawn(state.clone(), &http_client, shutdown_tx.subscribe());
    // Telegram consomme le flux traduit via sa queue bornee dediee, avec des handles optionnels.
    let telegram_handles =
        notifications::telegram::spawn(state, http_client, shutdown_tx.subscribe());
    let (telegram_bridge_handle, telegram_sender_handle) = match telegram_handles {
        Some(handles) => (Some(handles.bridge_handle), Some(handles.sender_handle)),
        None => (None, None),
    };

    // Le bootstrap conserve seulement les handles utiles a un shutdown borne.
    RuntimeWorkerHandles {
        financial_juice_poll_handle: collector_handles.financial_juice_poll_handle,
        financial_juice_live_handle: collector_handles.financial_juice_live_handle,
        x_collector_handle: collector_handles.x_handle,
        ingestion_pipeline_handle,
        translation_handle,
        tts_handle,
        calendar_day_audio_handle,
        edge_handle,
        live_fanout_handle,
        calendar_day_scheduler_handle,
        calendar_day_native_handle,
        calendar_day_alert_scheduler_handle,
        retention_handle,
        browser_fallback_monitor_handle,
        telegram_bridge_handle,
        telegram_sender_handle,
    }
}
