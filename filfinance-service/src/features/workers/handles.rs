//! Handles et attente de shutdown des workers runtime.

// Les handles Tokio permettent au bootstrap d'attendre les workers critiques au shutdown.
use tokio::task::JoinHandle;
// `timeout_at` applique la deadline globale, sans repartir d'une nouvelle duree par worker.
use tokio::time::timeout_at;
// Les logs de shutdown restent ici, au plus pres des handles que ce module possede.
use tracing::{info, warn};

use crate::shutdown::ShutdownDeadline;

/// Handles des workers que le bootstrap attend explicitement au shutdown.
pub struct RuntimeWorkerHandles {
    // Les deux workers FinancialJuice sont separes pour isoler poll/backfill et SignalR live.
    pub financial_juice_poll_handle: JoinHandle<()>,
    pub financial_juice_live_handle: JoinHandle<()>,
    // Le collecteur X reste vivant meme quand son switch runtime est desactive.
    pub x_collector_handle: JoinHandle<()>,
    // Le pipeline s'arrete au signal partage; le buffer memoire n'est pas persiste sans accuse aval.
    pub ingestion_pipeline_handle: JoinHandle<()>,
    // Le worker de traduction est absent quand `NEWS_TRANSLATION_ENABLED=false`.
    pub translation_handle: Option<JoinHandle<()>>,
    // Le worker TTS est absent quand `NEWS_TTS_ENABLED=false`.
    pub tts_handle: Option<JoinHandle<()>>,
    // Le worker audio calendrier est separe du worker TTS live.
    pub calendar_day_audio_handle: Option<JoinHandle<()>>,
    // Le publisher edge est absent quand `NEWS_EDGE_PUBLISH_ENABLED=false`.
    pub edge_handle: Option<JoinHandle<()>>,
    /// Abonnement Redis Pub/Sub du pilote Bunny DEV.
    pub live_fanout_handle: Option<JoinHandle<()>>,
    // Le scheduler calendrier du jour est absent quand la feature est desactivee.
    pub calendar_day_scheduler_handle: Option<JoinHandle<()>>,
    // Le worker canonique reprend les intentions SQLite avant toute admission.
    pub calendar_day_native_handle: Option<JoinHandle<()>>,
    // Le scheduler d'alertes audio calendrier est absent quand la feature est desactivee.
    pub calendar_day_alert_scheduler_handle: Option<JoinHandle<()>>,
    // La retention tourne toujours pour borner la base news locale.
    pub retention_handle: JoinHandle<()>,
    // Le monitoring du fallback browser tourne toujours, mais ne logue que si le fallback est attendu.
    pub browser_fallback_monitor_handle: JoinHandle<()>,
    // Telegram est absent lorsque le downstream n'est pas configure ou desactive.
    pub telegram_bridge_handle: Option<JoinHandle<()>>,
    pub telegram_sender_handle: Option<JoinHandle<()>>,
}

impl RuntimeWorkerHandles {
    /// Attend les workers critiques jusqu'a la deadline globale deja lancee par `main`.
    pub async fn wait_for_shutdown(self, shutdown_deadline: ShutdownDeadline) {
        // Les producteurs recoivent le signal simultanement avec le pipeline. On les rejoint
        // d'abord pour arreter les emissions source; le pipeline abandonne explicitement son
        // buffer memoire car les broadcasts aval ne fournissent aucun accuse de traitement.

        wait_for_required_worker(
            "financial juice periodic collector",
            self.financial_juice_poll_handle,
            shutdown_deadline,
        )
        .await;

        wait_for_required_worker(
            "financial juice SignalR collector",
            self.financial_juice_live_handle,
            shutdown_deadline,
        )
        .await;

        wait_for_required_worker("X collector", self.x_collector_handle, shutdown_deadline).await;

        // Ce handle arrive apres les producteurs afin de confirmer la fermeture de la queue live
        // sans persister un evenement brut qui ne pourrait plus traverser tous les downstreams.
        wait_for_required_worker(
            "ingestion pipeline",
            self.ingestion_pipeline_handle,
            shutdown_deadline,
        )
        .await;

        // Le worker de traduction n'existe que lorsque la traduction runtime est active.
        if let Some(handle) = self.translation_handle {
            wait_for_optional_worker("translation worker", handle, shutdown_deadline).await;
        }

        // Meme logique pour la TTS, qui peut etre desactivee par configuration.
        if let Some(handle) = self.tts_handle {
            wait_for_optional_worker("tts worker", handle, shutdown_deadline).await;
        }

        if let Some(handle) = self.calendar_day_audio_handle {
            wait_for_optional_worker("calendar day audio worker", handle, shutdown_deadline).await;
        }

        // Le bridge edge est optionnel et depend du mode de publication live.
        if let Some(handle) = self.edge_handle {
            wait_for_optional_worker("edge metadata publisher", handle, shutdown_deadline).await;
        }

        if let Some(handle) = self.live_fanout_handle {
            wait_for_optional_worker("live Redis fan-out subscriber", handle, shutdown_deadline)
                .await;
        }

        if let Some(handle) = self.calendar_day_scheduler_handle {
            wait_for_optional_worker("calendar day scheduler", handle, shutdown_deadline).await;
        }

        if let Some(handle) = self.calendar_day_native_handle {
            wait_for_optional_worker("calendar native worker", handle, shutdown_deadline).await;
        }

        if let Some(handle) = self.calendar_day_alert_scheduler_handle {
            wait_for_optional_worker("calendar day alert scheduler", handle, shutdown_deadline)
                .await;
        }

        // La retention tourne toujours pour borner la base SQLite locale.
        wait_for_required_worker(
            "news retention worker",
            self.retention_handle,
            shutdown_deadline,
        )
        .await;

        wait_for_required_worker(
            "financial juice browser fallback monitor",
            self.browser_fallback_monitor_handle,
            shutdown_deadline,
        )
        .await;

        // Telegram est un downstream non critique mais sa queue/retry doit quitter proprement.
        if let Some(handle) = self.telegram_bridge_handle {
            wait_for_optional_worker("telegram broadcast bridge", handle, shutdown_deadline).await;
        }

        if let Some(handle) = self.telegram_sender_handle {
            wait_for_optional_worker("telegram sender", handle, shutdown_deadline).await;
        }
    }
}

/// Attend un worker optionnel et garde un log homogene quel que soit son resultat.
async fn wait_for_optional_worker(
    name: &'static str,
    handle: JoinHandle<()>,
    shutdown_deadline: ShutdownDeadline,
) {
    // Cette fonction separe surtout la taxonomie de shutdown du bootstrap HTTP.
    wait_for_required_worker(name, handle, shutdown_deadline).await;
}

/// Attend un worker toujours present et journalise les trois issues possibles.
async fn wait_for_required_worker(
    name: &'static str,
    mut handle: JoinHandle<()>,
    shutdown_deadline: ShutdownDeadline,
) {
    // `timeout_at` evite qu'un worker long-vivant bloque la fin du process au-dela de la meme
    // deadline que le drain HTTP et les autres workers.
    match timeout_at(shutdown_deadline.instant(), &mut handle).await {
        // `Ok(Ok(()))` signifie que le timeout n'a pas expire et que la tache a fini normalement.
        Ok(Ok(())) => info!(worker = name, "{name} shut down cleanly"),
        // `JoinError` indique une terminaison anormale de la tache Tokio.
        Ok(Err(error)) => warn!(worker = name, %error, "{name} terminated with a join error"),
        // `Err(_)` indique que la deadline est expiree: on demande l'abort avant de relacher le
        // handle, afin de garder la sortie du process strictement bornee.
        Err(_) => {
            handle.abort();
            warn!(
                worker = name,
                "{name} did not stop before shutdown deadline and was aborted"
            );
        }
    }
}
