//! Chargement et validation de toute la configuration applicative depuis l'environnement.

mod auth;
mod calendar_day;
mod edge;
mod environment;
mod hls;
mod live_fanout;
mod logging;
mod runtime;
mod sources;
mod telegram;
mod translation;
mod translation_profile;
mod tts;
mod tts_defaults;
mod types;

// On reexporte ici notre type d'erreur applicatif et la vue API d'une source.
use crate::{errors::AppError, features::news::domain::SourceStatus};

// Les helpers d'environnement restent prives au module de configuration.
use environment::{env_or, env_u32, env_u64, env_usize};
// La validation HLS depend de la configuration TTS construite dans le bootstrap global.
use hls::validate_hls_config;
use runtime::{runtime_config_from_env, validate_runtime_auth_policy};
// Le profil de traduction est reexporte pour les providers qui calculent leurs arguments effectifs.
pub(crate) use translation_profile::translation_profile_beam_size;

// Les types publics conservent le chemin historique `crate::config::*` malgre le decoupage interne.
pub use types::{
    AppConfig, AuthConfig, CalendarDayConfig, CalendarNativeConfig, EdgeConfig,
    FinancialJuiceBrowserConfig, FinancialJuiceSiteConfig, HlsConfig, HttpConfig, LiveFanoutConfig,
    LoggingConfig, RedactedSecret, RuntimeConfig, ServerEnvironment, SourceConfig, SourcesConfig,
    StorageConfig, TelegramConfig, TranslationConfig, TtsConfig, XConfig,
};

impl AppConfig {
    /// Construit toute la configuration depuis les variables d'environnement.
    pub fn from_env() -> Result<Self, AppError> {
        // Les sous-configurations feature-first gardent leurs variables NEWS_* et validations locales.
        let runtime = runtime_config_from_env()?;
        let translation = TranslationConfig::from_env()?;
        let tts = TtsConfig::from_env()?;
        // HLS depend de la TTS: sans clip source, l'origin ne peut rien publier.
        let hls = HlsConfig::from_env()?;
        validate_hls_config(&hls, &tts)?;
        // L'auth est construite avant les sources pour rendre les erreurs de securite visibles.
        let auth = AuthConfig::from_env()?;
        validate_runtime_auth_policy(&runtime, &auth)?;
        // Le publisher edge reste separe des WebSockets backend pour une activation progressive.
        let edge = EdgeConfig::from_env()?;
        // Le fan-out Redis est reserve au pilote Bunny DEV; il ne modifie pas le Worker production.
        let live_fanout = LiveFanoutConfig::from_env()?;
        // Les logs fichiers sont lus tot pour initialiser aussi la telemetry.
        let logging = LoggingConfig::from_env()?;
        let data_audit_retention_days = env_usize("NEWS_DATA_AUDIT_RETENTION_DAYS", 7)?;
        if data_audit_retention_days == 0 {
            return Err(AppError::Config(
                "NEWS_DATA_AUDIT_RETENTION_DAYS must be greater than 0".to_owned(),
            ));
        }
        // Les sources encapsulent FinancialJuice, son calendrier, le collecteur navigateur et X.
        let sources = SourcesConfig::from_env()?;
        // Telegram reste un downstream optionnel isole de la collecte.
        let telegram = TelegramConfig::from_env()?;
        let calendar_day = CalendarDayConfig::from_env()?;

        // Ok(Self { ... }) construit la configuration complete seulement si toutes les lectures
        // et validations precedentes ont reussi.
        Ok(Self {
            // Regroupe les parametres lies au process principal.
            runtime,
            // Regroupe la configuration du stockage durable.
            storage: StorageConfig {
                // La base par defaut reste un chemin SQLite local pour le MVP.
                database_url: env_or("NEWS_DATABASE_URL", "sqlite://data/news.db"),
                max_connections: env_u32("NEWS_STORAGE_MAX_CONNECTIONS", 5)?,
                data_audit_retention_days,
            },
            // Les logs fichiers et le viewer admin partagent le meme repertoire configure.
            logging,
            // Parametres HTTP communs a tous les fetchs.
            http: HttpConfig {
                // Le user-agent par defaut porte le nom runtime filfinance-service pour que les
                // logs amont et les proxys identifient le nouveau branding sans changer la variable.
                user_agent: env_or("NEWS_HTTP_USER_AGENT", "filfinance-service/0.1"),
                connect_timeout_ms: env_u64("NEWS_HTTP_CONNECT_TIMEOUT_MS", 1_000)?,
            },
            // Parametres par source pour garder une lecture simple et centralisee.
            sources,
            // Parametres du downstream de traduction texte.
            translation,
            // Parametres du downstream de synthese vocale serveur.
            tts,
            // Parametres de l'origin HLS audio.
            hls,
            // Parametres du downstream Telegram.
            telegram,
            // Parametres du calendrier du jour execute nativement par Rust.
            calendar_day,
            // Le publisher edge reste un downstream optionnel et isole du pipeline d'ingestion.
            edge,
            // Redis porte le snapshot et le relais inter-replicas du WebSocket Bunny DEV.
            live_fanout,
            // La configuration auth est stockee avec le reste du runtime pour que les middlewares
            // puissent la lire via AppState sans variable globale applicative.
            auth,
        })
    }

    /// Construit la vue API des sources pour /api/sources.
    pub fn source_statuses(&self) -> Vec<SourceStatus> {
        self.sources.statuses()
    }
}
