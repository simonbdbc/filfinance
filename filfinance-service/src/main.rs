//! Point d'entree du binaire `filfinance-service`.

// `app_state` expose l'etat partage injecte aux handlers et workers.
mod app_state;
// `config` centralise le chargement des variables d'environnement.
mod config;
// `errors` expose l'erreur applicative globale hors du socle `shared` feature-first.
mod errors;
// `features` devient la facade feature-first du backend.
mod features;
// `shared` garde le socle transversal limite entre features.
mod shared;
// `shutdown` normalise SIGTERM Docker et Ctrl-C puis reveille les workers via le watch partage.
mod shutdown;
// `telemetry` initialise les logs et futures metriques process.
mod telemetry;

// `Arc` apporte une possession partagee thread-safe entre plusieurs taches async sans copier
// toute la configuration ou l'etat lourd a chaque fois.
use std::sync::Arc;
// `PathBuf` est un chemin possede. On l'utilise ici car `load_dotenv_file` peut retourner un
// chemin qui doit vivre apres la fin de la fonction qui l'a cree.
use std::path::PathBuf;
// `Duration` represente une duree typee. Rust prefere ce type a un entier brut pour eviter les
// confusions entre secondes, millisecondes et autres unites.
use std::time::Duration;

// `Client` est le client HTTP mutualise pour les collecteurs et certains downstreams.
use reqwest::Client;
// `OffsetDateTime` lit l'horloge UTC pour les nettoyages de donnees au demarrage.
use time::OffsetDateTime;
// `TcpListener` ouvre la socket TCP du serveur.
// `broadcast`, `mpsc` et `watch` sont trois familles de channels Tokio avec des usages differents:
// - `broadcast` pour diffuser le meme evenement a plusieurs abonnes,
// - `mpsc` pour une queue multi-producteurs vers un seul consommateur,
// - `watch` pour partager un dernier etat simple comme un signal de shutdown.
use tokio::{
    net::TcpListener,
    sync::{broadcast, mpsc, watch},
    task::JoinHandle,
    time::timeout_at,
};
// `info!` et `warn!` sont des macros de log structure de `tracing`.
use tracing::{info, warn};

// Cette ligne importe des types definis dans d'autres modules de notre crate locale. Le prefixe
// `crate::` signifie "depuis la racine du binaire courant".
use crate::{
    app_state::AppState,
    config::{AppConfig, LoggingConfig},
    errors::AppError,
    features::audio::{hls::HlsOriginState, tts::TtsRuntimeState},
    features::calendar_day::CalendarDayStore,
    features::ingestion::{
        runtime::{FinancialJuiceBrowserRuntimeState, FinancialJuiceRuntimeState, XRuntimeState},
        x_settings::XSettingsStore,
    },
    features::live::fanout::LiveFanout,
    features::news::NewsStore,
    features::translation::TranslationRuntimeState,
};

// Docker laisse une marge supplementaire au conteneur, mais cette borne applicative couvre des
// la reception du signal le serveur HTTP et les workers; les WebSockets `on_upgrade` ne sont pas
// des taches supervisees et les clients doivent donc se reconnecter pendant le basculement.
// La queue live s'arrete avant toute persistence partielle qui ne pourrait plus atteindre ses
// consommateurs aval.
const SHUTDOWN_GRACE_PERIOD: Duration = Duration::from_secs(10);

// `#[tokio::main]` transforme `main` en point d'entree async. Tokio cree le runtime a notre place
// pour que le reste du service puisse utiliser `async/await`.
#[tokio::main]
// Le `Result<(), AppError>` dit que `main` peut echouer proprement avec notre type d'erreur
// applicatif. L'operateur `?` plus bas remontera ces erreurs automatiquement.
async fn main() -> Result<(), AppError> {
    // On tente de charger `.env` tres tot pour que le reste du bootstrap lise deja l'environnement
    // effectif. Le `?` stoppe immediatement le process si ce chargement est invalide.
    let dotenv_path = load_dotenv_file()?;
    // La config logs est lue avant `AppConfig` pour pouvoir tracer le reste du bootstrap vers fichier.
    let logging = LoggingConfig::from_env()?;
    // Le logging est initialise juste apres pour que tout le bootstrap suivant soit trace.
    telemetry::init_tracing(&logging);
    // `if let Some(...)` est un idiome Rust pour traiter seulement le cas present d'un `Option`.
    if let Some(path) = dotenv_path {
        // `%path.display()` demande a `tracing` d'utiliser l'affichage texte du chemin plutot que
        // son rendu `Debug`.
        info!(dotenv_path = %path.display(), "loaded local .env configuration");
    }

    // Toute l'initialisation critique est centralisee ici avant de lancer les taches de fond.
    // `Arc::new(...)` place la configuration dans une allocation partagee. On pourra la cloner
    // legerement par pointeur dans plusieurs taches au lieu de recopier toute la structure.
    let config = Arc::new(AppConfig::from_env()?);
    let directory =
        std::env::var("NEWS_CALENDAR_LOG_DIR").unwrap_or_else(|_| "data/calendar-logs".into());
    let (calendar_logs, log_runtime) = features::calendar_logs::CalendarLogs::start(
        PathBuf::from(directory),
        features::calendar_logs::environment_secrets(),
        config.runtime.server_env.is_production(),
    )
    .await;
    let result = run(config, calendar_logs, &log_runtime).await;
    log_runtime.shutdown().await;
    result
}

async fn run(
    config: Arc<AppConfig>,
    calendar_logs: features::calendar_logs::CalendarLogs,
    _log_runtime: &features::calendar_logs::CalendarLogRuntime,
) -> Result<(), AppError> {
    // Le premier argument CLI permet de lancer une operation d'exploitation sans demarrer HTTP.
    let requested_command = std::env::args().nth(1);
    // Ce log resume les options les plus structurantes du runtime pour le debug des incidents.
    info!(
        server_env = %config.runtime.server_env.as_str(),
        translation_provider = %config.translation.provider,
        tts_enabled = config.tts.enabled,
        tts_provider = %config.tts.provider,
        tts_voice = %config.tts.voice,
        hls_enabled = config.hls.enabled,
        hls_access_control_enabled = config.hls.access_control_enabled,
        auth_enabled = config.auth.enabled,
        auth_provider = %config.auth.provider,
        edge_publish_enabled = config.edge.publish_enabled,
        live_fanout_enabled = config.live_fanout.enabled,
        calendar_day_enabled = config.calendar_day.enabled,
        "filfinance configuration loaded"
    );
    // La connexion SQLite est ouverte avant tout lancement de worker pour que les erreurs de
    // stockage fassent echouer le process au demarrage, pas en arriere-plan.
    let news = NewsStore::connect(&config.storage).await?;
    // Les migrations sont executees immediatement afin que les schemas soient coherents avant
    // toute lecture/ecriture concurrente.
    news.migrate().await?;
    let financial_juice_importance_backfilled = news
        .backfill_financial_juice_importance_from_news_level()
        .await?;
    if financial_juice_importance_backfilled > 0 {
        info!(
            updated_rows = financial_juice_importance_backfilled,
            "financial juice news importance backfilled from news_level"
        );
    }
    let browser_cleanup = news
        .cleanup_invalid_financial_juice_browser_events(OffsetDateTime::now_utc())
        .await?;
    if browser_cleanup.deleted_news_events > 0 || browser_cleanup.deleted_translations > 0 {
        info!(
            deleted_news_events = browser_cleanup.deleted_news_events,
            deleted_translations = browser_cleanup.deleted_translations,
            "invalid financial juice browser rows cleaned"
        );
    }
    // Toute commande explicite est refusee pour eviter de demarrer le serveur par erreur.
    if let Some(command) = requested_command {
        return Err(AppError::Config(format!(
            "unsupported filfinance-service command: {command}"
        )));
    }
    // Le calendrier du jour garde ses tables canoniques dans la meme base SQLite que les news,
    // mais dans un domaine separe du live et du legacy daily_summary.
    let calendar_day = CalendarDayStore::from_news_store(&news);
    calendar_day.migrate().await?;
    // L'etat runtime de la TTS est partage via `Arc` pour exposer l'observabilite serveur.
    let tts_runtime = Arc::new(TtsRuntimeState::new(&config.tts));
    // L'origin HLS est partage entre le worker TTS, qui publie, et les routes HTTP, qui servent.
    let hls_origin = Arc::new(HlsOriginState::from_config(&config.hls));
    let translation_runtime = Arc::new(TranslationRuntimeState::new(&config.translation));
    // Le switch admin part de la variable NEWS_FINANCIAL_JUICE_ENABLED puis vit en memoire.
    let financial_juice_runtime = Arc::new(FinancialJuiceRuntimeState::new(
        config.sources.financial_juice.enabled,
    ));
    // Le collecteur navigateur est un process externe: le backend ne garde que son dernier statut.
    let financial_juice_browser_runtime = Arc::new(FinancialJuiceBrowserRuntimeState::new(
        config.sources.financial_juice_browser.source.enabled,
    ));
    // Le runtime X part de l'env puis peut etre surcharge par SQLite et par l'admin.
    let x_runtime = Arc::new(XRuntimeState::new(&config.sources.x));
    let x_settings_store = XSettingsStore::from_news_store(&news);
    x_settings_store.migrate().await?;
    match x_settings_store.load().await {
        Ok(Some(settings)) => x_runtime.replace_settings(settings),
        Ok(None) => {
            let initial_settings = x_runtime.settings();
            match x_settings_store.save(&initial_settings).await {
                Ok(settings) => x_runtime.replace_settings(settings),
                Err(error) => warn!(%error, "failed to seed X settings in SQLite"),
            }
        }
        Err(error) => warn!(%error, "failed to load X settings from SQLite"),
    }

    // Les deux channels separent l'ingestion des consommateurs temps reel.
    // Chaque `broadcast::channel` cree un emetteur partage et des receveurs abonnes a la demande.
    let (broadcast_tx, _) = broadcast::channel(config.runtime.broadcast_buffer);
    // Ce bus dedie ne transporte que les traductions FR persistentes.
    let (translated_broadcast_tx, _) = broadcast::channel(config.runtime.broadcast_buffer);
    // Ce bus dedie transporte les metadata live edge-compatible quand le client utilise le fallback backend.
    let (live_metadata_tx, _) = broadcast::channel(config.runtime.broadcast_buffer);
    // Redis est volontairement optionnel: seul le profil Bunny DEV l'active.
    let live_fanout = LiveFanout::from_config(&config.live_fanout)?;
    // Ce bus dedie reserve les traces de runs de test E2E.
    let (test_trace_tx, _) = broadcast::channel(config.runtime.broadcast_buffer);
    // Ce bus dedie alimente la page admin du calendrier du jour.
    let (calendar_day_admin_tx, _) = broadcast::channel(config.runtime.broadcast_buffer);
    // `mpsc::channel` modele une vraie queue d'ingestion avec un seul consommateur principal.
    let (ingest_tx, ingest_rx) = mpsc::channel(config.runtime.ingest_buffer);
    let (calendar_day_audio_tx, calendar_day_audio_rx) =
        mpsc::channel(config.runtime.ingest_buffer);
    // La queue native ne transporte qu'un reveil. Les intentions calendaires
    // restent toujours dans SQLite, donc un `try_send` plein ne bloque jamais
    // l'ingestion live et sera repris par le polling du worker.
    let (calendar_day_native, calendar_day_native_rx) =
        features::calendar_day::native::runtime::CalendarNativeHandle::channel(
            config.calendar_day.active(),
            config.calendar_day.native.queue_capacity,
        );
    // `watch::channel(false)` partage un bool de shutdown. Les workers observent le dernier etat.
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    // `AppState` regroupe tout ce que les handlers et workers doivent partager a long terme.
    let state = AppState {
        calendar_logs,
        // `Arc::clone` clone seulement le pointeur partage, pas toute la configuration.
        config: Arc::clone(&config),
        // `news` est deja un handle clonable vers le pool SQLite news.
        news,
        // `calendar_day` porte le nouveau calendrier economique journalier canonique.
        calendar_day,
        calendar_day_native,
        // Le runtime TTS partage les compteurs et l'etat courant du worker audio.
        tts_runtime,
        // L'origin HLS reste inactif tant que `NEWS_HLS_ENABLED=false`.
        hls_origin,
        // Le runtime de traduction expose les compteurs et le dernier etat utile au debug.
        translation_runtime,
        // L'override FinancialJuice reste separe de la config immutable chargee depuis l'environnement.
        financial_juice_runtime,
        // Le statut navigateur reste separe du collecteur Rust historique.
        financial_juice_browser_runtime,
        // L'override X combine le switch admin, les comptes et les mots cles persistables.
        x_runtime,
        // Le store SQLite persiste les reglages admin X.
        x_settings_store,
        // On clone l'emetteur `mpsc` pour garder une copie dans l'etat et en laisser une locale.
        ingest_tx: ingest_tx.clone(),
        // Les taches detachees depuis une feature peuvent ainsi s'arreter sur le meme signal que
        // les workers supervises, sans ouvrir un contrat de shutdown dans les handlers HTTP.
        shutdown_rx: shutdown_rx.clone(),
        // Les tests compilent aussi ce bootstrap: leur sender est conserve uniquement sous
        // `cfg(test)` pour ne jamais confondre une fixture detruite avec un arret process.
        #[cfg(test)]
        shutdown_test_keepalive: shutdown_tx.clone(),
        // Ces emetteurs broadcast seront reutilises plus tard par les handlers WS.
        broadcast_tx,
        translated_broadcast_tx,
        live_metadata_tx,
        live_fanout,
        test_trace_tx,
        calendar_day_admin_tx,
        calendar_day_audio_tx,
    };

    // Les travaux calendrier deja durables passent avant le scheduler : une
    // reprise ne peut donc pas etre depassee par une nouvelle admission apres
    // redemarrage. Cette recuperation ne touche ni la file live news ni ses
    // consommateurs.
    features::calendar_day::native::runtime::recover_expired_leases_at_startup(&state).await?;

    // Le client HTTP partage ses parametres communs entre toutes les sources et downstreams.
    // Le builder rend la configuration plus lisible qu'un constructeur monolithique.
    let http_client = Client::builder()
        // On clone la `String` car `reqwest` a besoin d'une valeur possedee ici.
        .user_agent(config.http.user_agent.clone())
        // La session FinancialJuice repose sur des cookies ASP.NET et d'auth stockes dans le client.
        .cookie_store(true)
        // `Duration::from_millis` transforme un entier d'env en type de duree explicite.
        .connect_timeout(Duration::from_millis(config.http.connect_timeout_ms))
        // `build()?` valide la configuration et remonte une erreur si elle est incoherente.
        .build()?;

    // Les workers sont lances par la facade feature-first pour garder `main.rs` centre sur le bootstrap.
    let worker_handles = features::spawn_workers(
        state.clone(),
        ingest_rx,
        calendar_day_audio_rx,
        calendar_day_native_rx,
        http_client,
        &shutdown_tx,
        shutdown_rx,
    );

    // Le routeur est compose par les features, puis l'etat partage est injecte une seule fois ici.
    let app = features::router(state.clone()).with_state(state);

    // Le bind TCP echoue ici si le port est deja pris ou si l'adresse est invalide.
    let listener = TcpListener::bind(&config.runtime.listen_addr).await?;
    info!(listen_addr = %config.runtime.listen_addr, "filfinance service listening");
    // Le serveur observe le `watch` interne. `main` reste l'unique lecteur de SIGTERM/Ctrl-C et
    // demarre donc la meme deadline absolue pour HTTP et tous les workers avant ce reveil.
    let http_shutdown_rx = shutdown_tx.subscribe();
    let mut server_handle = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown::wait_for_shutdown_request(http_shutdown_rx))
            .await
    });
    // `Option` permet de deplacer les handles dans la seule branche gagnante du `select!` sans
    // recreer de tache ni de seconde politique d'arret.
    let mut worker_handles = Some(worker_handles);

    tokio::select! {
        signal = shutdown::wait_for_shutdown_signal() => {
            let shutdown_deadline = shutdown::ShutdownDeadline::after(SHUTDOWN_GRACE_PERIOD);
            if shutdown::request_worker_shutdown(&shutdown_tx) {
                info!(
                    signal = signal.as_str(),
                    shutdown_grace_secs = SHUTDOWN_GRACE_PERIOD.as_secs(),
                    "shutdown signal received; shared graceful shutdown deadline started"
                );
            }

            let workers_shutdown = worker_handles
                .take()
                .expect("workers must be available until shutdown begins")
                .wait_for_shutdown(shutdown_deadline);
            let http_shutdown = wait_for_http_server_shutdown(&mut server_handle, shutdown_deadline);
            let ((), http_result) = tokio::join!(workers_shutdown, http_shutdown);
            http_result?;
        }
        server_result = &mut server_handle => {
            // Un arret HTTP inattendu doit aussi couper les workers; il recoit la meme borne sans
            // attendre un second signal OS.
            let server_result = resolve_http_server_result(server_result);
            let shutdown_deadline = shutdown::ShutdownDeadline::after(SHUTDOWN_GRACE_PERIOD);
            if shutdown::request_worker_shutdown(&shutdown_tx) {
                info!(
                    shutdown_grace_secs = SHUTDOWN_GRACE_PERIOD.as_secs(),
                    "HTTP server stopped; shared graceful shutdown deadline started for workers"
                );
            }
            worker_handles
                .take()
                .expect("workers must be available until shutdown begins")
                .wait_for_shutdown(shutdown_deadline)
                .await;
            server_result?;
        }
    }

    // `Ok(())` marque une fin normale du programme.
    Ok(())
}

/// Cesse l'acceptation HTTP et attend Axum jusqu'a la deadline globale. Les callbacks WebSocket
/// `on_upgrade` ne sont pas des taches supervisees par ce handle: les clients doivent reconnecter
/// au futur endpoint blue/green. Les workers sont attendus en parallele par `main`.
async fn wait_for_http_server_shutdown(
    server_handle: &mut JoinHandle<std::io::Result<()>>,
    shutdown_deadline: shutdown::ShutdownDeadline,
) -> Result<(), AppError> {
    // `timeout_at` consomme sa future: on cree donc un reborrow court du handle afin de pouvoir
    // l'annuler explicitement dans la branche timeout.
    match timeout_at(shutdown_deadline.instant(), &mut *server_handle).await {
        Ok(server_result) => resolve_http_server_result(server_result),
        Err(_) => {
            server_handle.abort();
            warn!("HTTP server did not drain before shutdown deadline and was aborted");
            Ok(())
        }
    }
}

/// Convertit le resultat de la tache HTTP sans laisser un `JoinError` silencieux au bootstrap.
fn resolve_http_server_result(
    server_result: Result<std::io::Result<()>, tokio::task::JoinError>,
) -> Result<(), AppError> {
    match server_result {
        Ok(result) => result.map_err(AppError::from),
        Err(error) => Err(AppError::Storage(format!(
            "HTTP server task terminated unexpectedly: {error}"
        ))),
    }
}

/// Charge les fichiers `.env` locaux disponibles, sans masquer des variables deja exportees
/// explicitement dans l'environnement du process.
fn load_dotenv_file() -> Result<Option<PathBuf>, AppError> {
    // `FILFINANCE_ENV_FILE` permet de lancer le binaire directement avec le meme fichier que Docker.
    let explicit_env_file = std::env::var_os("FILFINANCE_ENV_FILE")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from);
    // Les fallbacks couvrent les deux lancements locaux courants:
    // - depuis `filfinance-service/` avec `.env` ou `../.env.local`;
    // - depuis la racine repo avec `.env.local`.
    let mut candidates = Vec::new();
    if let Some(path) = explicit_env_file {
        candidates.push((path, true));
    }
    candidates.push((PathBuf::from(".env"), false));
    candidates.push((PathBuf::from(".env.local"), false));
    candidates.push((PathBuf::from("../.env.local"), false));

    let mut last_loaded = None;
    for (candidate, required) in candidates {
        // `dotenvy` ne remplace pas les variables deja presentes: Docker et le shell restent donc
        // prioritaires sur les valeurs chargees depuis un fichier local.
        match dotenvy::from_path(&candidate) {
            Ok(()) => last_loaded = Some(candidate),
            Err(error) if error.not_found() && !required => {}
            Err(error) => {
                return Err(AppError::Config(format!(
                    "failed to load env file {}: {error}",
                    candidate.display()
                )));
            }
        }
    }

    Ok(last_loaded)
}
