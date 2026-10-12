//! Worker asynchrone de purge retention news.

// `AppState` donne acces au stockage partage sans recréer de connexion SQLite.
use crate::app_state::AppState;

// `JoinHandle` permet a `main` de garder la tache dans la liste des workers long-vivants.
use tokio::{sync::watch, task::JoinHandle, time::sleep};
// `OffsetDateTime` fournit le jour UTC courant de reference pour J/J-1.
use time::OffsetDateTime;
// Les logs structurés rendent visibles les suppressions effectives sans bruiter les cycles vides.
use tracing::{debug, info, warn};

use super::schedule::{current_local_time, duration_until_next_daily_run};

/// Lance la purge J/J-1 au demarrage puis chaque jour a 02:00 heure locale.
pub fn spawn(state: AppState, mut shutdown_rx: watch::Receiver<bool>) -> JoinHandle<()> {
    tokio::spawn(async move {
        prune_once(&state).await;

        loop {
            let sleep_duration = duration_until_next_daily_run(current_local_time());

            tokio::select! {
                changed = shutdown_rx.changed() => {
                    match changed {
                        Ok(()) if *shutdown_rx.borrow() => {
                            info!("news retention worker received shutdown signal");
                            break;
                        }
                        Ok(()) => {}
                        Err(_) => {
                            info!("news retention worker shutdown channel closed");
                            break;
                        }
                    }
                }
                _ = sleep(sleep_duration) => {
                    prune_once(&state).await;
                }
            }
        }
    })
}

/// Execute une passe de purge sans faire tomber le service si SQLite remonte une erreur ponctuelle.
async fn prune_once(state: &AppState) {
    match state
        .news
        .prune_news_retention(
            OffsetDateTime::now_utc(),
            state.config.storage.data_audit_retention_days,
        )
        .await
    {
        Ok(report) => {
            let deleted_calendar_hls_dirs = match state
                .hls_origin
                .prune_calendar_summary_manifests(&report.retained_calendar_days)
                .await
            {
                Ok(deleted) => deleted,
                Err(error) => {
                    warn!(%error, "calendar summary hls retention prune failed");
                    0
                }
            };

            if report.deleted_news_events > 0
                || report.deleted_translations > 0
                || report.deleted_daily_news_qualifications > 0
                || report.deleted_daily_summaries > 0
                || report.deleted_daily_summary_run_artifacts > 0
                || report.deleted_daily_summary_run_steps > 0
                || report.deleted_daily_summary_runs > 0
                || report.deleted_news_blacklist_hits > 0
                || report.deleted_news_truncation_audits > 0
                || report.deleted_calendar_rows > 0
                || deleted_calendar_hls_dirs > 0
            {
                info!(
                    oldest_retained_day = %report.oldest_retained_day,
                    oldest_audit_retained_day = %report.oldest_audit_retained_day,
                    data_audit_retention_days = report.data_audit_retention_days,
                    deleted_news_events = report.deleted_news_events,
                    deleted_translations = report.deleted_translations,
                    deleted_daily_news_qualifications = report.deleted_daily_news_qualifications,
                    deleted_daily_summaries = report.deleted_daily_summaries,
                    deleted_daily_summary_run_artifacts = report.deleted_daily_summary_run_artifacts,
                    deleted_daily_summary_run_steps = report.deleted_daily_summary_run_steps,
                    deleted_daily_summary_runs = report.deleted_daily_summary_runs,
                    deleted_news_blacklist_hits = report.deleted_news_blacklist_hits,
                    deleted_news_truncation_audits = report.deleted_news_truncation_audits,
                    deleted_calendar_rows = report.deleted_calendar_rows,
                    deleted_calendar_hls_dirs,
                    "news retention pruned old database rows and calendar hls files"
                );
            } else {
                debug!(
                    oldest_retained_day = %report.oldest_retained_day,
                    oldest_audit_retained_day = %report.oldest_audit_retained_day,
                    data_audit_retention_days = report.data_audit_retention_days,
                    "news retention checked database rows and calendar hls files"
                );
            }
        }
        Err(error) => {
            warn!(%error, "news retention prune failed");
        }
    }
}
