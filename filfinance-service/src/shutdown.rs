//! Reception et diffusion idempotente du signal d'arret du processus.
//!
//! Le point d'entree HTTP attend ce module afin de traiter `Ctrl+C` en local et `SIGTERM` dans
//! Docker. Les workers qui s'abonnent deja au canal `watch` recoivent ensuite un unique changement,
//! ce qui borne leur fin de vie sans pretendre garantir un handoff live sans outbox durable.

// `Duration` garde la grace d'arret typee avant de fabriquer une deadline Tokio.
use std::time::Duration;
// `watch` transporte l'etat d'arret partage entre le serveur HTTP et les workers abonnes.
use tokio::{sync::watch, time::Instant};
// Les warnings couvrent le cas rare ou le runtime ne peut pas installer un handler de signal.
use tracing::warn;

/// Origine systeme du premier arret gracieux demande au processus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShutdownSignal {
    /// Signal interactif recu pendant un lancement local ou supervise.
    CtrlC,
    /// Signal envoye par Docker ou un superviseur Unix lors d'un stop/redeploy.
    Terminate,
}

impl ShutdownSignal {
    /// Etiquette structuree stable pour les logs de cycle de vie.
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::CtrlC => "ctrl_c",
            Self::Terminate => "sigterm",
        }
    }
}

/// Deadline unique partagee par le serveur HTTP et les workers pendant un arret process.
// Le type evite de transmettre des durees independantes entre etapes: toutes les attentes se
// calent sur le meme instant Tokio initialise au premier SIGTERM/Ctrl-C.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ShutdownDeadline(Instant);

impl ShutdownDeadline {
    /// Construit la deadline globale une seule fois, au debut de l'arret gracieux.
    pub(crate) fn after(grace_period: Duration) -> Self {
        Self(Instant::now() + grace_period)
    }

    /// Expose l'instant absolu aux appels Tokio `timeout_at`.
    pub(crate) fn instant(self) -> Instant {
        self.0
    }
}

/// Attend un signal d'arret supporte par la plateforme courante.
pub(crate) async fn wait_for_shutdown_signal() -> ShutdownSignal {
    // Sous Unix, Docker et systemd envoient `SIGTERM`; le handler explicite empeche la terminaison
    // immediate afin que le serveur HTTP et les workers puissent se fermer proprement.
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};

        match signal(SignalKind::terminate()) {
            Ok(mut terminate) => {
                tokio::select! {
                    ctrl_c = tokio::signal::ctrl_c() => {
                        if let Err(error) = ctrl_c {
                            warn!(%error, "failed while waiting for Ctrl-C shutdown signal");
                        }
                        ShutdownSignal::CtrlC
                    }
                    _ = terminate.recv() => ShutdownSignal::Terminate,
                }
            }
            Err(error) => {
                // On garde Ctrl-C disponible si l'installation de SIGTERM a echoue, plutot que de
                // rendre le serveur incapable de s'arreter gracieusement dans tous les contextes.
                warn!(%error, "failed to install SIGTERM shutdown handler; waiting for Ctrl-C");
                wait_for_ctrl_c().await
            }
        }
    }

    // Les builds non Unix n'ont pas de SIGTERM; Ctrl-C reste le comportement portable precedent.
    #[cfg(not(unix))]
    {
        wait_for_ctrl_c().await
    }
}

/// Attend le `watch` interne utilise pour commencer le drain graceful du serveur HTTP.
// Cette attente ne lit jamais directement les signaux OS: `main` reste l'unique point qui
// transforme SIGTERM/Ctrl-C en deadline globale et en diffusion aux workers.
pub(crate) async fn wait_for_shutdown_request(mut shutdown_rx: watch::Receiver<bool>) {
    if *shutdown_rx.borrow() {
        return;
    }

    loop {
        match shutdown_rx.changed().await {
            Ok(()) if *shutdown_rx.borrow_and_update() => return,
            Ok(()) => {}
            // En production, `main` conserve le sender jusqu'a la fin du process. Une fermeture
            // inattendue reste toutefois un signal de retrait defensif pour ne pas retenir HTTP.
            Err(_) => return,
        }
    }
}

/// Attend Ctrl-C et conserve le comportement de secours commun aux plateformes.
async fn wait_for_ctrl_c() -> ShutdownSignal {
    if let Err(error) = tokio::signal::ctrl_c().await {
        warn!(%error, "failed while waiting for Ctrl-C shutdown signal");
    }
    ShutdownSignal::CtrlC
}

/// Diffuse une seule fois l'arret aux workers et retourne `true` pour le premier emetteur.
pub(crate) fn request_worker_shutdown(shutdown_tx: &watch::Sender<bool>) -> bool {
    // `send_if_modified` garantit qu'un second chemin de sortie HTTP ne reveille pas les workers
    // une seconde fois. Le passage a `true` reveille tous leurs receveurs `watch` existants.
    shutdown_tx.send_if_modified(|shutdown_requested| {
        if *shutdown_requested {
            false
        } else {
            *shutdown_requested = true;
            true
        }
    })
}

#[cfg(test)]
mod tests {
    use tokio::sync::watch;

    use super::{ShutdownSignal, request_worker_shutdown};

    #[test]
    fn shutdown_signal_labels_are_stable() {
        assert_eq!(ShutdownSignal::CtrlC.as_str(), "ctrl_c");
        assert_eq!(ShutdownSignal::Terminate.as_str(), "sigterm");
    }

    #[tokio::test]
    async fn worker_shutdown_is_broadcast_once() {
        let (shutdown_tx, mut first_worker_rx) = watch::channel(false);
        let mut second_worker_rx = shutdown_tx.subscribe();

        assert!(request_worker_shutdown(&shutdown_tx));
        first_worker_rx
            .changed()
            .await
            .expect("the first worker must receive the shutdown update");
        second_worker_rx
            .changed()
            .await
            .expect("the second worker must receive the shutdown update");
        assert!(*first_worker_rx.borrow_and_update());
        assert!(*second_worker_rx.borrow_and_update());

        assert!(!request_worker_shutdown(&shutdown_tx));
        assert!(
            !first_worker_rx
                .has_changed()
                .expect("watch channel must stay open")
        );
        assert!(
            !second_worker_rx
                .has_changed()
                .expect("watch channel must stay open")
        );
    }
}
