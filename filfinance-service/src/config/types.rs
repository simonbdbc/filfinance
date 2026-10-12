//! Types publics de configuration, reexportes par `crate::config`.

/// Point d'entree unique de la configuration chargee au demarrage.
// `Debug` facilite l'inspection en debug/log, et `Clone` permet de recopier la configuration
// lorsqu'un morceau de code a besoin d'une version possedee plutot qu'empruntee.
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// Parametres de runtime lies au process et aux buffers internes.
    pub runtime: RuntimeConfig,
    /// Parametres de stockage et de bootstrap de la base.
    pub storage: StorageConfig,
    /// Parametres de sortie fichier et de lecture admin des logs.
    pub logging: LoggingConfig,
    /// Parametres HTTP communs a l'ensemble des collecteurs.
    pub http: HttpConfig,
    /// Configuration des sources de collecte.
    pub sources: SourcesConfig,
    /// Configuration de la derivation texte francaise pour le flux temps reel.
    pub translation: TranslationConfig,
    /// Configuration de la synthese vocale serveur pour les clips HLS audio.
    pub tts: TtsConfig,
    /// Configuration de l'origin HLS pour clips temps reel et replay explicite.
    pub hls: HlsConfig,
    /// Configuration du downstream Telegram.
    pub telegram: TelegramConfig,
    /// Configuration du calendrier Rust natif et de ses alertes audio.
    pub calendar_day: CalendarDayConfig,
    /// Configuration du publisher metadata live vers Cloudflare Durable Objects.
    pub edge: EdgeConfig,
    /// Configuration du fan-out Redis utilise par le pilote Bunny DEV.
    pub live_fanout: LiveFanoutConfig,
    /// Configuration d'authentification appliquee aux surfaces HTTP protegees.
    pub auth: AuthConfig,
}

/// Configuration des logs applicatifs fichiers et du viewer admin.
#[derive(Debug, Clone)]
pub struct LoggingConfig {
    /// Active ou non l'ecriture fichier locale en plus de stdout.
    pub file_enabled: bool,
    /// Repertoire contenant les fichiers quotidiens `YYYY-MM-DD.log`.
    pub directory: String,
    /// Nombre de jours conserves, borne aujourd'hui a J et J-1.
    pub retention_days: usize,
    /// Taille maximale d'une page lue par l'endpoint admin.
    pub admin_max_lines: usize,
}

/// Configuration du processus principal et du pipeline en memoire.
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// Environnement serveur effectif, utilise pour activer les garde-fous production.
    pub server_env: ServerEnvironment,
    /// Adresse `ip:port` sur laquelle `axum` doit binder le serveur.
    pub listen_addr: String,
    /// Taille de la queue `mpsc` d'ingestion avant backpressure ou drop amont.
    pub ingest_buffer: usize,
    /// Taille des bus `broadcast` utilises pour les flux temps reel.
    pub broadcast_buffer: usize,
    /// Duree de vie de la dedup memoire en secondes.
    pub dedup_ttl_secs: u64,
    /// Seuil de log des requetes HTTP lentes, en millisecondes. Zero desactive le log.
    pub slow_request_threshold_ms: u64,
}

/// Environnement serveur supporte par les regles de securite au demarrage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerEnvironment {
    /// Environnement local, dev ou test.
    Local,
    /// Environnement de production, soumis aux garde-fous stricts.
    Production,
}

/// Configuration de la couche de persistence.
#[derive(Debug, Clone)]
pub struct StorageConfig {
    /// URL SQLx de la base SQLite.
    pub database_url: String,
    /// Nombre maximal de connexions ouvertes par le pool.
    pub max_connections: u32,
    /// Nombre de jours conserves pour les tables d'audit SQL append-only.
    pub data_audit_retention_days: usize,
}

/// Parametres HTTP partages par tous les collecteurs.
#[derive(Debug, Clone)]
pub struct HttpConfig {
    /// User-Agent commun a tous les fetchs sortants.
    pub user_agent: String,
    /// Timeout de connexion TCP/HTTP exprime en millisecondes.
    pub connect_timeout_ms: u64,
}

/// Regroupe les sources connues par le service.
#[derive(Debug, Clone)]
pub struct SourcesConfig {
    /// Configuration du nouveau collecteur Financial Juice via session web authentifiee.
    pub financial_juice: FinancialJuiceSiteConfig,
    /// Configuration du collecteur navigateur FinancialJuice externe au backend Rust.
    pub financial_juice_browser: FinancialJuiceBrowserConfig,
    /// Configuration specifique du collecteur X via API officielle.
    pub x: XConfig,
}

/// Configuration du collecteur Financial Juice base sur la session web authentifiee.
#[derive(Debug, Clone)]
pub struct FinancialJuiceSiteConfig {
    /// Nom logique stable de la source.
    pub name: String,
    /// Active ou desactive le collecteur site.
    pub enabled: bool,
    /// URL du front `/home`, utilisee pour le bootstrap WebForms et la lecture des variables JS.
    pub home_url: String,
    /// Frequence de polling HTTP du backlog recent, en plus du live SignalR.
    pub poll_interval_ms: u64,
    /// Timeout des appels HTTP vers le site et l'API interne.
    pub request_timeout_ms: u64,
    /// Backoff applique apres un throttling explicite.
    pub rate_limit_backoff_ms: u64,
    /// Delai maximal avant de forcer un refresh complet de la session auth.
    pub session_refresh_interval_ms: u64,
    /// Backoff applique apres une deconnexion ou un echec du live SignalR.
    pub live_retry_backoff_ms: u64,
    /// Nombre maximal de pages `GetPreviousNews` a remonter au bootstrap.
    pub startup_backfill_page_limit: usize,
    /// Offset minute envoye a l'API interne FinancialJuice.
    pub time_offset_minutes: i32,
    /// Email du compte FinancialJuice.
    pub email: Option<String>,
    /// Mot de passe du compte FinancialJuice.
    pub password: Option<String>,
}

/// Configuration de l'entree backend reservee au collecteur navigateur FinancialJuice.
#[derive(Debug, Clone)]
pub struct FinancialJuiceBrowserConfig {
    /// Parametres publics de la source navigateur exposee dans `/api/sources`.
    pub source: SourceConfig,
    /// Token bearer partage entre le collecteur Playwright et l'entree d'ingestion backend.
    pub ingest_token: Option<String>,
    /// Taille maximale d'un lot poste par le collecteur navigateur.
    pub max_batch_size: usize,
}

/// Contrat commun de configuration d'une source de collecte.
#[derive(Debug, Clone)]
pub struct SourceConfig {
    /// Nom logique stable de la source, reutilise dans les logs et les erreurs.
    pub name: String,
    /// Active ou desactive le collecteur sans supprimer sa definition.
    pub enabled: bool,
    /// URL amont polluee par le collecteur.
    pub url: String,
    /// Frequence de polling de la source.
    pub poll_interval_ms: u64,
    /// Timeout de la requete vers cette source.
    pub request_timeout_ms: u64,
    /// Backoff applique apres un `429` ou un throttling cote source.
    pub rate_limit_backoff_ms: u64,
}

/// Configuration du collecteur X recent search.
#[derive(Debug, Clone)]
pub struct XConfig {
    /// Parametres communs de polling, dont l'URL `/2/tweets/search/recent`.
    pub source: SourceConfig,
    /// Bearer token serveur de l'application X Developer.
    pub bearer_token: Option<String>,
    /// Requete X brute optionnelle; prioritaire sur la liste de comptes.
    pub query: Option<String>,
    /// Liste de comptes publics surveilles quand aucune requete brute n'est fournie.
    pub account_usernames: Vec<String>,
    /// Liste de mots cles qui filtre les posts des comptes surveilles.
    pub keywords: Vec<String>,
    /// Langue X optionnelle ajoutee a la requete, par exemple `en`.
    pub language: Option<String>,
    /// Exclut les reposts/retweets via `-is:retweet`.
    pub exclude_reposts: bool,
    /// Nombre maximum de posts demandes par appel recent search.
    pub max_results: usize,
    /// Fenetre initiale lue au premier appel quand aucun `since_id` n'est encore connu.
    pub startup_lookback_minutes: u64,
}

/// Parametres du worker Telegram.
#[derive(Debug, Clone)]
pub struct TelegramConfig {
    /// Active ou non le downstream Telegram.
    pub enabled: bool,
    /// Base URL de l'API Telegram, utile pour tests ou proxys.
    pub api_base_url: String,
    /// Token du bot, optionnel pour permettre un mode desactive sans secret local.
    pub bot_token: Option<String>,
    /// Liste des chats cibles.
    pub chat_ids: Vec<String>,
    /// Taille de la queue interne Telegram.
    pub queue_buffer: usize,
    /// Delai minimal entre deux envois HTTP Telegram.
    pub rate_limit_ms: u64,
    /// Nombre de tentatives en cas d'erreur retriable.
    pub retry_attempts: usize,
    /// Delai d'attente entre deux tentatives.
    pub retry_backoff_ms: u64,
}

/// Parametres du publisher metadata live vers le Worker edge.
#[derive(Debug, Clone)]
pub struct EdgeConfig {
    /// Active ou non la publication des metadata live vers Cloudflare.
    pub publish_enabled: bool,
    /// Endpoint HTTP prive du Worker, typiquement `https://.../publish/live`.
    pub publish_url: Option<String>,
    /// Secret serveur utilise pour signer les publications HMAC vers le Worker edge.
    pub publish_secret: Option<String>,
    /// Timeout maximal d'un POST edge, pour ne jamais ralentir le pipeline principal.
    pub publish_timeout_ms: u64,
    /// Nombre de retries apres la premiere tentative en cas d'erreur temporaire.
    pub publish_retry_attempts: usize,
    /// Backoff lineaire de base entre deux tentatives de publication.
    pub publish_retry_backoff_ms: u64,
    /// Active l'endpoint backend qui genere les tickets courts pour `/ws/live`.
    pub ticket_enabled: bool,
    /// Secret HMAC partage avec le Worker pour signer les tickets edge.
    pub ticket_secret: Option<String>,
    /// Duree de vie maximale d'un ticket edge en secondes.
    pub ticket_ttl_secs: u64,
}

/// Parametres du relais Redis qui remplace le Durable Object uniquement en DEV Bunny.
#[derive(Debug, Clone)]
pub struct LiveFanoutConfig {
    /// Active le snapshot Redis et la diffusion Pub/Sub inter-replicas.
    pub enabled: bool,
    /// URL Redis interne Docker, jamais exposee au navigateur.
    pub redis_url: Option<String>,
    /// Canal Pub/Sub partage par les replicas Rust.
    pub redis_channel: String,
    /// Cle Redis qui contient le snapshot JSON normalise.
    pub redis_snapshot_key: String,
}

/// Secret de configuration dont `Debug` ne revele jamais la valeur.
// Le wrapper possede la chaine afin que la cle survive au chargement de l'environnement.
// `Clone` satisfait les derives transitifs des structs de configuration; tout clone effectivement appele
// dupliquerait donc la cle en memoire. `PartialEq`/`Eq` servent aux assertions sans ajouter de formatage.
#[derive(Clone, PartialEq, Eq)]
pub struct RedactedSecret(String);

// L'implementation manuelle remplace le derive `Debug`, qui exposerait la cle OpenRouter.
impl std::fmt::Debug for RedactedSecret {
    // Le formatter recoit seulement une preuve de presence, jamais la valeur encapsulee.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("RedactedSecret([configured])")
    }
}

impl RedactedSecret {
    /// Encapsule une valeur deja nettoyee par le lecteur d'environnement.
    pub(crate) fn new(value: String) -> Self {
        // Le tuple struct prend possession de la `String`; aucune seconde copie du secret n'est creee.
        Self(value)
    }

    /// Emprunte la cle pour la validation fermee et le client HTTP natif.
    pub(crate) fn expose(&self) -> &str {
        // L'emprunt lie sa duree de vie au wrapper et interdit au caller de liberer ou muter la cle.
        &self.0
    }
}

/// Parametres du chemin calendrier canonique execute par Rust.
///
/// Le worker natif persiste les resultats valides dans `calendar_day_*`.
#[derive(Debug, Clone)]
pub struct CalendarNativeConfig {
    /// Endpoint OpenRouter injectable uniquement vers loopback dans les tests.
    pub endpoint: String,
    /// Identifiant du modele OpenRouter autorise pour les deux etapes calendrier.
    pub model: String,
    /// Cle OpenRouter masquee dans tous les formats `Debug`.
    pub api_key: Option<RedactedSecret>,
    /// Timeout total d'une tentative OpenRouter.
    pub request_timeout_ms: u64,
    /// Nombre maximal de nouvelles tentatives OpenRouter apres la premiere.
    pub retry_attempts: usize,
    /// Backoff de base borne entre deux tentatives OpenRouter.
    pub retry_backoff_ms: u64,
    /// Une seule inference calendrier active par defaut, isolee du live news.
    pub max_concurrency: usize,
    /// Nombre maximal de notifications memoire du worker durable.
    pub queue_capacity: usize,
    /// Taille maximale d'une reponse OpenRouter lue en memoire.
    pub max_response_bytes: usize,
}

/// Parametres de la refonte calendrier du jour.
#[derive(Debug, Clone)]
pub struct CalendarDayConfig {
    /// Active ou non les workers du calendrier du jour.
    pub enabled: bool,
    /// Heure locale Europe/Paris du run principal, au format HH:MM.
    pub schedule_time: String,
    /// Heure locale Europe/Paris apres laquelle les retries automatiques s'arretent.
    pub retry_end_time: String,
    /// Timezone metier, volontairement bornee a Europe/Paris dans cette version.
    pub timezone: String,
    /// Nombre maximal de retries techniques apres un echec calendrier.
    pub max_retries: u32,
    /// Base de backoff en secondes entre deux retries.
    pub retry_base_secs: u64,
    /// Debut de la fenetre hebdomadaire de generation du samedi.
    pub week_generation_window_start: String,
    /// Fin de la fenetre hebdomadaire de generation du samedi.
    pub week_generation_window_end: String,
    /// Debut de la fenetre hebdomadaire de verification du dimanche.
    pub week_verification_window_start: String,
    /// Fin de la fenetre hebdomadaire de verification du dimanche.
    pub week_verification_window_end: String,
    /// Configuration du worker calendrier Rust canonique.
    pub native: CalendarNativeConfig,
    /// Offsets des alertes high en secondes avant l'evenement.
    pub alert_offsets_secs: Vec<u64>,
}

/// Parametres de verification des JWT Better Auth cote backend.
#[derive(Debug, Clone)]
pub struct AuthConfig {
    /// Active ou non le middleware d'auth HTTP.
    pub enabled: bool,
    /// Fournisseur d'identite attendu par le backend.
    pub provider: String,
    /// URL JWKS exposee par Better Auth pour verifier les signatures asymetriques.
    pub jwks_url: Option<String>,
    /// Issuer attendu dans le claim `iss` des JWT Better Auth.
    pub jwt_issuer: Option<String>,
    /// Audience attendue dans le claim `aud` des JWT Better Auth.
    pub jwt_audience: String,
    /// Roles applicatifs autorises a acceder aux surfaces moderateur.
    pub moderator_roles: Vec<String>,
}

/// Parametres du worker de traduction texte.
#[derive(Debug, Clone)]
pub struct TranslationConfig {
    /// Active ou desactive la derivation texte FR.
    pub enabled: bool,
    /// Langue source attendue, aujourd'hui surtout `en`.
    pub source_language: String,
    /// Langue cible du flux derive, aujourd'hui `fr`.
    pub target_language: String,
    /// Nom du provider de traduction selectionne.
    pub provider: String,
    /// Profil qualite/latence applique aux arguments OPUS-MT effectifs.
    pub profile: String,
    /// Nombre de providers de traduction long-vivants consommant la queue interne.
    pub worker_count: usize,
    /// Programme local a executer pour un provider par commande.
    pub local_command_program: Option<String>,
    /// Arguments supplementaires de cette commande locale.
    pub local_command_args: Vec<String>,
    /// Timeout maximal d'un appel provider local.
    pub local_command_timeout_ms: u64,
    /// Active le warmup du provider au demarrage.
    pub warmup_enabled: bool,
    /// Phrase source de warmup, utile pour charger le runtime local a vide.
    pub warmup_source_text: String,
    /// Limite de caracteres pour les body copies dans les payloads live.
    pub max_body_chars: usize,
}

/// Parametres du worker TTS serveur.
#[derive(Debug, Clone)]
pub struct TtsConfig {
    /// Active ou non la synthese vocale serveur.
    pub enabled: bool,
    /// Provider TTS selectionne.
    pub provider: String,
    /// Voix active au demarrage.
    pub voice: String,
    /// Programme Python ou wrapper local qui pilote Piper.
    pub local_command_program: String,
    /// Arguments statiques passes a ce helper local.
    pub local_command_args: Vec<String>,
    /// Repertoire racine contenant les voix Piper installees localement.
    pub models_dir: String,
    /// Nombre de providers TTS long-vivants consommant la queue interne.
    pub worker_count: usize,
    /// Vitesse de diction cible du provider.
    pub rate_words_per_minute: u32,
    /// Frequence d'echantillonnage du clip audio produit.
    pub output_sample_rate_hz: u32,
    /// Timeout maximal d'un rendu audio.
    pub command_timeout_ms: u64,
    /// Active le warmup du moteur TTS au demarrage.
    pub warmup_enabled: bool,
    /// Texte de warmup utilise pour charger le provider.
    pub warmup_text: String,
}

/// Parametres de l'origin HLS clips/replay.
#[derive(Debug, Clone)]
pub struct HlsConfig {
    /// Active ou non la production HLS clips/replay.
    pub enabled: bool,
    /// Active ou non le controle d'acces par ticket court sur les playlists et segments.
    pub access_control_enabled: bool,
    /// Secret HMAC utilise pour signer les tickets d'acces HLS.
    pub access_ticket_secret: Option<String>,
    /// Duree du bucket de ticket HLS; le ticket reste partage par bucket pour rester cacheable CDN.
    pub access_ticket_ttl_secs: u64,
    /// Racine locale des segments `.ts` generes par le backend.
    pub segments_dir: String,
    /// Commande d'encodage AAC/MPEG-TS, generalement `ffmpeg`.
    pub ffmpeg_command: String,
    /// Duree cible des segments replay HLS.
    pub replay_segment_target_duration_secs: u32,
    /// Duree cible des segments de clips HLS finis par news.
    pub clip_segment_target_duration_secs: u32,
}
