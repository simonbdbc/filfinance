//! Etat partage injecte dans les handlers et les workers du service.

// `Arc` est le pointeur de partage standard de Rust pour faire circuler un meme objet entre
// plusieurs taches sans se battre avec les lifetimes.
use std::sync::Arc;

// `broadcast`, `mpsc` et `watch` sont les canaux Tokio utilises partout dans l'architecture du
// projet.
use tokio::sync::{broadcast, mpsc, watch};

// On importe ici les types qui composent l'etat partage et les helpers de trace lies aux runs E2E.
use crate::{
    config::AppConfig,
    features::audio::{domain::CalendarDayAudioJob, hls::HlsOriginState, tts::TtsRuntimeState},
    features::calendar_day::{
        CalendarDayAdminBroadcastEvent, CalendarDayStore, native::runtime::CalendarNativeHandle,
    },
    features::ingestion::{
        runtime::{FinancialJuiceBrowserRuntimeState, FinancialJuiceRuntimeState, XRuntimeState},
        x_settings::XSettingsStore,
    },
    features::live::{
        dto::{EdgeLiveMessage, TestTraceEvent},
        fanout::LiveFanout,
    },
    features::news::NewsStore,
    features::news::domain::{NewsEvent, PersistedNewsEvent, extract_test_run_id},
    features::translation::TranslationRuntimeState,
    features::translation::domain::PersistedNewsTranslation,
    shared::time::now_rfc3339,
};

/// Regroupe les dependances longues durees partagees par l'application.
// `#[derive(Clone)]` permet de cloner legerement `AppState` pour le passer a plusieurs handlers.
// Ce clone reste peu couteux car les champs lourds sont eux-memes clonables ou partages.
#[derive(Clone)]
pub struct AppState {
    pub calendar_logs: crate::features::calendar_logs::CalendarLogs,
    // `Arc<AppConfig>` evite de recopier toute la configuration a chaque clone de l'etat.
    pub config: Arc<AppConfig>,
    // `NewsStore` encapsule le pool SQLx news et se clone par handle interne.
    pub news: NewsStore,
    // `CalendarDayStore` porte le calendrier economique journalier canonique.
    pub calendar_day: CalendarDayStore,
    // Le reveil du worker calendrier Rust est borne et best-effort; SQLite
    // reste l'unique source de verite lorsqu'il est plein ou indisponible.
    pub calendar_day_native: CalendarNativeHandle,
    // Etat mutable partage de la TTS pour changer la voix sans redemarrer le process.
    pub tts_runtime: Arc<TtsRuntimeState>,
    // Origin HLS partage entre le worker TTS, les clips `/audio/clips/*` et le replay du jour.
    pub hls_origin: Arc<HlsOriginState>,
    // Etat runtime partage de la traduction pour les logs et l'observabilite HTTP.
    pub translation_runtime: Arc<TranslationRuntimeState>,
    // Controle runtime du collecteur FinancialJuice, initialise depuis NEWS_FINANCIAL_JUICE_ENABLED.
    pub financial_juice_runtime: Arc<FinancialJuiceRuntimeState>,
    // Statut runtime du collecteur navigateur FinancialJuice externe au backend Rust.
    pub financial_juice_browser_runtime: Arc<FinancialJuiceBrowserRuntimeState>,
    // Controle runtime et listes dynamiques du collecteur X.
    pub x_runtime: Arc<XRuntimeState>,
    // Persistence SQLite des parametres admin X.
    pub x_settings_store: XSettingsStore,
    // Queue canonique d'ingestion des `NewsEvent`.
    pub ingest_tx: mpsc::Sender<NewsEvent>,
    // Signal de shutdown clonable pour les taches detachees depuis un handler ou une feature.
    // Il reste interne au processus et ne fait partie d'aucun contrat HTTP/admin.
    pub shutdown_rx: watch::Receiver<bool>,
    // Les fixtures gardent le sender vivant afin qu'une fermeture artificielle du `watch` ne soit
    // pas confondue avec SIGTERM par les watchdogs. Ce champ n'existe pas dans le binaire produit:
    // `main` y conserve explicitement le sender de production.
    #[cfg(test)]
    #[allow(dead_code)]
    pub(crate) shutdown_test_keepalive: watch::Sender<bool>,
    // Bus temps reel des evenements persistes bruts.
    pub broadcast_tx: broadcast::Sender<PersistedNewsEvent>,
    // Bus temps reel des traductions FR persistentes.
    pub translated_broadcast_tx: broadcast::Sender<PersistedNewsTranslation>,
    // Bus control plane live compatible avec le backend `/ws/news` pendant la transition edge.
    pub live_metadata_tx: broadcast::Sender<EdgeLiveMessage>,
    /// Relais Redis optionnel du pilote Bunny DEV; absent en local et en production Cloudflare.
    pub live_fanout: Option<LiveFanout>,
    // Bus reserve aux traces detaillees des runs de test.
    pub test_trace_tx: broadcast::Sender<TestTraceEvent>,
    // Bus reserve au suivi admin temps reel du calendrier du jour.
    pub calendar_day_admin_tx: broadcast::Sender<CalendarDayAdminBroadcastEvent>,
    // Queue dediee aux audios calendrier du jour a transformer en HLS Piper.
    pub calendar_day_audio_tx: mpsc::Sender<CalendarDayAudioJob>,
}

impl AppState {
    /// Point d'entree unique des producteurs live: sockets locales, Redis DEV et Worker prod
    /// reçoivent exactement le même DTO sans bloquer l'ingestion si Redis est indisponible.
    pub fn publish_live_metadata(&self, message: EdgeLiveMessage) {
        crate::features::live::fanout::publish_local(self, message);
    }

    /// Emet une trace de run de test si un identifiant de run est disponible.
    pub fn emit_test_trace(
        &self,
        run_id: &str,
        stage: &str,
        status: &str,
        message: impl Into<String>,
        detail: Option<String>,
    ) {
        // `let _ = ...` signifie que l'emission de trace est best-effort: on ne casse pas le hot
        // path si personne n'ecoute le canal de debug.
        let _ = self.test_trace_tx.send(TestTraceEvent {
            // `to_owned()` copie un `&str` en `String` possedee pour que l'evenement vive seul.
            run_id: run_id.to_owned(),
            stage: stage.to_owned(),
            status: status.to_owned(),
            // `Into<String>` garde l'API pratique pour des callers qui passent `&str` ou `String`.
            message: message.into(),
            detail,
            // Chaque trace est horodatee a l'emission pour faciliter la lecture chronologique.
            timestamp: now_rfc3339(),
        });
    }

    /// Emet une trace a partir d'une reference brute si elle correspond a un run de test.
    pub fn emit_test_trace_for_raw_ref(
        &self,
        raw_payload_ref: Option<&str>,
        stage: &str,
        status: &str,
        message: impl Into<String>,
        detail: Option<String>,
    ) {
        // `extract_test_run_id` retourne un `Option<&str>`; `if let Some(...)` traite seulement
        // le cas ou la reference brute correspond bien a un run E2E synthetique.
        if let Some(run_id) = extract_test_run_id(raw_payload_ref) {
            self.emit_test_trace(run_id, stage, status, message, detail);
        }
    }

    /// Emet une trace a partir d'un evenement persiste sans refaire la logique d'extraction.
    pub fn emit_test_trace_for_event(
        &self,
        event: &PersistedNewsEvent,
        stage: &str,
        status: &str,
        message: impl Into<String>,
        detail: Option<String>,
    ) {
        // `as_deref()` convertit un `Option<String>` ou `Option<&String>` en `Option<&str>`,
        // ce qui evite une allocation juste pour lire la valeur optionnelle.
        self.emit_test_trace_for_raw_ref(
            event.raw_payload_ref.as_deref(),
            stage,
            status,
            message,
            detail,
        );
    }
}
