//! Boucle runtime du pipeline d'ingestion.

// `HashMap` sert ici de cache memoire pour la dedup recente et pour l'agregation de compteurs.
// `Duration` et `Instant` permettent de mesurer des delais de vie sans manipuler des entiers nus.
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

// `mpsc` est la queue Tokio du hot path: plusieurs producteurs, un consommateur pipeline; `watch`
// porte l'arret cooperatif du processus.
use tokio::sync::{mpsc, watch};
// `interval` cree un ticker asynchrone et `MissedTickBehavior` controle son comportement si la
// tache prend du retard entre deux ticks.
use tokio::time::{MissedTickBehavior, interval};
// Le log explicite distingue l'arret demande de la fermeture naturelle de la queue.
use tracing::{info, warn};

// On importe seulement l'evenement canonique brut et l'etat partage.
use crate::{app_state::AppState, features::news::domain::NewsEvent};

use super::duplicates::{DUPLICATE_LOG_INTERVAL_SECS, flush_duplicate_logs};

mod event;

/// Consomme la file d'ingestion et projette les evenements vers le stockage puis le broadcast.
pub(crate) async fn run(
    state: AppState,
    ingest_rx: mpsc::Receiver<NewsEvent>,
    shutdown_rx: watch::Receiver<bool>,
) {
    // `|| {}` est une fermeture sans effet; la production garde la meme boucle
    // et n'alloue aucun channel de synchronisation reserve aux tests.
    run_inner(state, ingest_rx, shutdown_rx, || {}).await;
}

/// Signale aux tests que le worker est initialise avant de mesurer le hot path.
/// `cfg(test)` exclut ce point d'entree et son channel du binaire de production.
#[cfg(test)]
pub(crate) async fn run_with_startup_signal(
    state: AppState,
    ingest_rx: mpsc::Receiver<NewsEvent>,
    shutdown_rx: watch::Receiver<bool>,
    startup_signal: tokio::sync::oneshot::Sender<()>,
) {
    // `move` transfere le Sender a la fermeture; `send` le consomme une seule
    // fois. Un test deja termine ne doit pas modifier le cycle de vie du worker.
    run_inner(state, ingest_rx, shutdown_rx, move || {
        let _ = startup_signal.send(());
    })
    .await;
}

// `FnOnce` permet de consommer le signal de test apres initialisation sans
// dupliquer le pipeline: les memes traitements et shutdown sont exerces.
async fn run_inner(
    state: AppState,
    mut ingest_rx: mpsc::Receiver<NewsEvent>,
    mut shutdown_rx: watch::Receiver<bool>,
    on_started: impl FnOnce(),
) {
    // Le TTL de dedup est converti une seule fois en `Duration` typee.
    let ttl = Duration::from_secs(state.config.runtime.dedup_ttl_secs);
    // Ce cache memoire evite de retraiter plusieurs fois le meme evenement dans une courte fenetre.
    let mut recently_seen: HashMap<String, Instant> = HashMap::new();
    // Les compteurs de doublons sont agreges par source et type pour reduire fortement le bruit.
    let mut pending_duplicate_counts: HashMap<(String, String), u64> = HashMap::new();
    // Ce ticker force un flush periodique des warnings agregees.
    let mut duplicate_log_ticker = interval(Duration::from_secs(DUPLICATE_LOG_INTERVAL_SECS));
    // `Delay` evite de "rattraper" artificiellement plusieurs ticks d'un coup si le pipeline a ete occupe.
    duplicate_log_ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
    // Le signal de test precede la premiere attente, apres toute initialisation.
    on_started();
    // La boucle vit tant que la queue d'ingestion ou le process reste ouvert.
    loop {
        // `tokio::select!` attend plusieurs sources d'evenements async en parallele:
        // ici la queue d'ingestion, le ticker de flush et le signal d'arret process.
        tokio::select! {
            // Quand une news et le shutdown sont prets au meme instant, fermer le receiver
            // d'abord empeche une persistence brute sans chaine aval complete.
            biased;
            changed = shutdown_rx.changed() => {
                if changed.is_err() || *shutdown_rx.borrow() {
                    // Les consommateurs aval reposent sur des `broadcast` sans accuse de
                    // traitement. Persister le buffer apres leur arret creerait des news brutes
                    // sans traduction/audio/edge; le buffer memoire est donc abandonne de facon
                    // explicite jusqu'a ce qu'une outbox durable permette un vrai handoff.
                    let discarded_events = ingest_rx.len();
                    ingest_rx.close();
                    flush_duplicate_logs(&mut pending_duplicate_counts);
                    if discarded_events > 0 {
                        warn!(
                            discarded_events,
                            "ingestion pipeline stopped at shutdown; buffered events were intentionally not persisted"
                        );
                    } else {
                        info!("ingestion pipeline received shutdown signal");
                    }
                    break;
                }
            }
            maybe_event = ingest_rx.recv() => {
                // `let Some(event) = ... else { ... }` est une forme Rust concise pour sortir proprement
                // si le channel est ferme et ne renverra plus d'evenement.
                let Some(event) = maybe_event else {
                    // Avant de quitter, on emet les compteurs de doublons encore accumules.
                    flush_duplicate_logs(&mut pending_duplicate_counts);
                    break;
                };

                event::process_event(
                    &state,
                    event,
                    &mut recently_seen,
                    &mut pending_duplicate_counts,
                    ttl,
                )
                .await;
            }
            _ = duplicate_log_ticker.tick() => {
                // Le ticker ne fait qu'un travail secondaire: flusher les warnings agregees.
                flush_duplicate_logs(&mut pending_duplicate_counts);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::{
        sync::{mpsc, watch},
        time::timeout,
    };

    use crate::features::{
        news::domain::{NewsEvent, NewsListParams},
        test_support::app_state,
    };

    use super::run;

    #[tokio::test]
    async fn pipeline_exits_when_process_shutdown_is_requested() {
        let state = app_state().await;
        let (_ingest_tx, ingest_rx) = mpsc::channel(state.config.runtime.ingest_buffer);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let pipeline = tokio::spawn(run(state, ingest_rx, shutdown_rx));

        shutdown_tx
            .send(true)
            .expect("pipeline receiver must still observe the shutdown signal");
        let joined = timeout(Duration::from_secs(1), pipeline)
            .await
            .expect("pipeline must stop within the bounded shutdown window");
        joined.expect("pipeline shutdown task must not panic");
    }

    #[tokio::test]
    async fn pipeline_does_not_persist_events_buffered_at_shutdown() {
        let state = app_state().await;
        let (ingest_tx, ingest_rx) = mpsc::channel(4);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        ingest_tx
            .send(NewsEvent::new_headline(
                "shutdown_test",
                "event buffered before graceful shutdown",
                "https://example.test/shutdown-drain",
                None,
                None,
            ))
            .await
            .expect("event must enter the bounded ingestion queue before shutdown");
        shutdown_tx
            .send(true)
            .expect("pipeline receiver must still observe the shutdown signal");

        let pipeline = tokio::spawn(run(state.clone(), ingest_rx, shutdown_rx));

        let joined = timeout(Duration::from_secs(1), pipeline)
            .await
            .expect("pipeline must stop within the bounded shutdown window");
        joined.expect("pipeline shutdown task must not panic");

        let persisted = state
            .news
            .list_news(&NewsListParams {
                limit: 10,
                source: Some("shutdown_test".to_owned()),
                event_kind: None,
                before_id: None,
            })
            .await
            .expect("canonical store must remain queryable after shutdown");
        assert!(persisted.items.is_empty());
    }
}
