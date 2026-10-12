//! Full payload diagnostics are emitted only through calendar_logs when scoped by a calendar operation.
//! Orchestrateur durable du calendrier Rust.
//!
//! Le runtime ne transporte jamais les pages source dans SQLite ni dans les
//! journaux applicatifs ordinaires. La queue durable contient seulement une
//! intention idempotente; le journal calendrier isole conserve les corps
//! source et sorties IA pour le diagnostic. Les notifications `mpsc` ne sont
//! qu'un reveil best-effort :
//! une reprise periodique et au demarrage relit toujours SQLite.

use std::time::Duration;

use reqwest::Client;
use time::{Duration as TimeDuration, OffsetDateTime};
use tokio::{
    sync::{mpsc, watch},
    task::JoinHandle,
    time::{self as tokio_time, MissedTickBehavior},
};
use tracing::{info, warn};
use url::Url;

use crate::{
    app_state::AppState,
    errors::AppError,
    features::{
        audio::domain::{CalendarDayAudioJob, CalendarDayAudioKind},
        calendar_day::{
            self, CalendarDayAdminBroadcastEvent, CalendarDayIngestTriggerOutcome, CalendarDayItem,
            CalendarDayRun,
            contracts::{
                self, CalendarCoveredSourceBinding, CalendarImportance, CalendarSourceSnapshot,
                CalendarSummaryInputItem,
            },
        },
    },
};

use super::{
    collectors::{
        CalendarCollectorAudit, CalendarCollectorOutcome, CalendarCollectorSet, CalendarLocalDay,
    },
    openrouter::{
        OpenRouterApiKey, OpenRouterCalendarClient, OpenRouterCalendarClientConfig,
        OpenRouterCalendarError, OpenRouterCalendarLimits, OpenRouterCalendarModel,
        OpenRouterCalendarRequest,
    },
    prompts::{build_ingestion_prompt, build_summary_prompt},
    work::{
        CALENDAR_SOURCE_REFUSAL_TERMINAL_ERROR, CalendarAudioWorkContext, CalendarWorkAdmission,
        CalendarWorkErrorCode, CalendarWorkKind, CalendarWorkLeaseMutation, CalendarWorkStore,
        ClaimedCalendarWork,
    },
};

#[cfg(test)]
mod diagnostic_probe;

/// Duree maximale d'une possession de travail natif.
///
/// Elle couvre les deux fetches strictement sequentiels (20 secondes chacun),
/// une inference OpenRouter bornee et son petit budget de retry. Une panne
/// reste recuperable a l'expiration sans jamais bloquer la file live news.
const CALENDAR_NATIVE_LEASE_DURATION: Duration = Duration::from_secs(10 * 60);
/// Le reveil periodique rend une notification perdue ou une file `try_send`
/// saturee inoffensive pour la persistance durable.
const CALENDAR_NATIVE_POLL_INTERVAL: Duration = Duration::from_secs(1);
/// Les retries de la file ne sont jamais immediats afin d'eviter une boucle
/// serree si un fournisseur ou Piper est indisponible.
const CALENDAR_NATIVE_RETRY_BACKOFF: TimeDuration = TimeDuration::seconds(5);
/// Toute intention durable a au plus trois executions. Au-dela, elle devient
/// un dead-letter redige plutot que de retenir indefiniment une capacite de la
/// file ou de solliciter un fournisseur en boucle.
const CALENDAR_NATIVE_MAX_WORK_ATTEMPTS: u32 = 3;
/// Le runtime traite un seul travail a la fois. C'est intentionnel : la
/// concurrence IA est donc strictement egale a un et ne peut pas concurrencer
/// le pipeline live news.
const CALENDAR_NATIVE_CLAIM_LIMIT: usize = 1;

/// Signal sans payload : SQLite est la source de verite et le receiver relit
/// donc toujours les intentions durables au reveil.
#[derive(Debug)]
pub(crate) struct CalendarNativeWake;

/// Resultat redacte d'une notification mpsc non bloquante.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CalendarNativeNotifyOutcome {
    Disabled,
    Enqueued,
    Full,
    Closed,
}

/// Facade clonable de la queue de reveil native.
///
/// Elle ne contient aucune donnee calendrier et n'est jamais attendue : un
/// handler admin ou le scheduler peut admettre une intention SQLite meme si
/// cette queue est pleine.
#[derive(Clone, Default)]
pub(crate) struct CalendarNativeHandle {
    tx: Option<mpsc::Sender<CalendarNativeWake>>,
}

impl CalendarNativeHandle {
    /// Cree une queue de reveil bornee seulement quand le calendrier est actif.
    pub(crate) fn channel(
        enabled: bool,
        capacity: usize,
    ) -> (Self, mpsc::Receiver<CalendarNativeWake>) {
        if !enabled {
            let (_tx, rx) = mpsc::channel(1);
            return (Self { tx: None }, rx);
        }
        let (tx, rx) = mpsc::channel(capacity.max(1));
        (Self { tx: Some(tx) }, rx)
    }

    /// Revele le worker sans faire attendre le caller.
    pub(crate) fn try_notify(&self) -> CalendarNativeNotifyOutcome {
        let Some(tx) = self.tx.as_ref() else {
            return CalendarNativeNotifyOutcome::Disabled;
        };
        match tx.try_send(CalendarNativeWake) {
            Ok(()) => CalendarNativeNotifyOutcome::Enqueued,
            Err(mpsc::error::TrySendError::Full(_)) => CalendarNativeNotifyOutcome::Full,
            Err(mpsc::error::TrySendError::Closed(_)) => CalendarNativeNotifyOutcome::Closed,
        }
    }
}

/// Admet une ingestion native sans effectuer de collecte dans le handler.
///
/// La reponse admin reste immediate : le travail durable est ecrit avant le
/// reveil best-effort. Le retour reprend le type historique interne pour que
/// les routes publiques/admin ne changent pas de contrat pendant la bascule.
pub(crate) async fn admit_ingestion(
    state: &AppState,
    local_day: &str,
    reason: &str,
    force: bool,
    attempt: u32,
) -> Result<CalendarDayIngestTriggerOutcome, AppError> {
    state.calendar_logs.scope(serde_json::json!({"local_day": local_day, "workflow_kind": "ingestion", "attempt": attempt}), async {
crate::features::calendar_logs::operation("admit_ingestion", serde_json::json!({"reason": reason, "force": force, "attempt": attempt}), async {

    calendar_day::ensure_business_local_day(local_day)?;
    let current = state.calendar_day.ensure_state(local_day).await?;
    let work_store = work_store(state);
    // Ce verrou durable couvre le cas exceptionnel ou la persistance de
    // l'audit redige a echoue apres l'observation d'un refus. Il reste
    // consulte meme si l'etat admin n'a pas pu etre mis a jour, afin qu'un
    // redemarrage ou une relance forcee ne recollecte jamais ce jour.
    if work_store
        .has_terminal_ingestion_source_refusal(local_day)
        .await?
    {
        return Ok(CalendarDayIngestTriggerOutcome::Skipped {
            reason: "sources_refused_terminal".to_owned(),
        });
    }
    if let Some(run) =
        calendar_day::active_calendar_day_pending_run(state, local_day, "ingestion").await?
    {
        return Ok(CalendarDayIngestTriggerOutcome::Skipped {
            reason: format!("pending:{}", run.run_id),
        });
    }
    if current.calendar_ingestion_status == "pending" {
        return Ok(CalendarDayIngestTriggerOutcome::Skipped {
            reason: "pending_state".to_owned(),
        });
    }
    // Un refus anti-bot ou d'acces est terminal pour ce jour. Le laisser
    // passer via `force`, le scheduler ou une reprise rejouerait une requete
    // que le fournisseur vient precisement de refuser.
    if current.last_error.as_deref() == Some(CALENDAR_SOURCE_REFUSAL_TERMINAL_ERROR) {
        return Ok(CalendarDayIngestTriggerOutcome::Skipped {
            reason: "sources_refused_terminal".to_owned(),
        });
    }
    if !force && current.calendar_ingestion_status == "ready" {
        return Ok(CalendarDayIngestTriggerOutcome::Skipped {
            reason: "ready".to_owned(),
        });
    }
    if !force && attempt >= state.config.calendar_day.max_retries {
        return Ok(CalendarDayIngestTriggerOutcome::Skipped {
            reason: "max_retries".to_owned(),
        });
    }
    if !force
        && !calendar_day::calendar_retry_backoff_elapsed(state, local_day, "ingestion").await?
    {
        return Ok(CalendarDayIngestTriggerOutcome::Skipped {
            reason: "retry_backoff".to_owned(),
        });
    }

    match state
        .calendar_day
        .admit_native_ingestion(
            &work_store,
            local_day,
            reason,
            attempt,
            OffsetDateTime::now_utc(),
        )
        .await?
    {
        calendar_day::CalendarNativeIngestionAdmission::Started {
            state: state_row,
            run,
            admission,
        } => {
            calendar_day::emit_admin_event(
                state,
                CalendarDayAdminBroadcastEvent::RunStarted(run.clone()),
            );
            calendar_day::emit_admin_event(
                state,
                CalendarDayAdminBroadcastEvent::StateUpdated(state_row),
            );
            record_native_admission(state, &run, "ingestion", &admission).await;
            record_native_notify(state, &run, state.calendar_day_native.try_notify()).await;
            Ok(CalendarDayIngestTriggerOutcome::Started(run))
        }
        calendar_day::CalendarNativeIngestionAdmission::Existing(run) => {
            Ok(CalendarDayIngestTriggerOutcome::Skipped {
                reason: format!("pending:{}", run.run_id),
            })
        }
    }

}).await
}).await
}

/// Admet un resume Rust de reparation sans appeler de service externe.
///
/// Le chemin nominal est cree dans le commit atomique d'ingestion. Cette
/// fonction est reservee au scheduler de verification et aux reprises admin
/// lorsque les items canoniques existent deja mais que le resume est absent ou
/// en erreur.
pub(crate) async fn admit_summary(
    state: &AppState,
    local_day: &str,
    reason: &str,
    force: bool,
    attempt: u32,
) -> Result<Option<CalendarDayRun>, AppError> {
    state.calendar_logs.scope(serde_json::json!({"local_day": local_day, "workflow_kind": "summary", "attempt": attempt}), async {
crate::features::calendar_logs::operation("admit_summary", serde_json::json!({"reason": reason, "force": force, "attempt": attempt}), async {

    calendar_day::ensure_business_local_day(local_day)?;
    let current = state.calendar_day.ensure_state(local_day).await?;
    if current.calendar_ingestion_status != "ready" {
        return Ok(None);
    }
    if let Some(run) =
        calendar_day::active_calendar_day_pending_run(state, local_day, "summary").await?
    {
        return Ok(Some(run));
    }
    if !force && current.calendar_summary_status == "audio_ready" {
        return Ok(None);
    }
    if !force && attempt >= state.config.calendar_day.max_retries {
        return Ok(None);
    }
    let items = state.calendar_day.list_items(local_day).await?;
    if items.is_empty() {
        return Ok(None);
    }

    let work = work_store(state);
    match state
        .calendar_day
        .admit_native_summary(&work, local_day, reason, attempt, OffsetDateTime::now_utc())
        .await?
    {
        calendar_day::CalendarNativeSummaryAdmission::Started {
            state: state_row,
            run,
            admission,
        } => {
            calendar_day::emit_admin_event(
                state,
                CalendarDayAdminBroadcastEvent::RunStarted(run.clone()),
            );
            calendar_day::emit_admin_event(
                state,
                CalendarDayAdminBroadcastEvent::StateUpdated(state_row),
            );
            record_native_admission(state, &run, "summary", &admission).await;
            record_native_notify(state, &run, state.calendar_day_native.try_notify()).await;
            Ok(Some(run))
        }
        calendar_day::CalendarNativeSummaryAdmission::Existing(run) => Ok(Some(run)),
    }

}).await
}).await
}

/// Recupere les leases calendrier expirees avant le demarrage des workers.
///
/// Cette etape est executee par le bootstrap, avant le scheduler et avant
/// toute nouvelle admission. Le worker la repete ensuite periodiquement : un
/// echec transitoire n'empeche donc pas une reprise ulterieure, mais il ne
/// permet jamais a une admission neuve de passer devant un travail deja
/// durable.
pub(crate) async fn recover_expired_leases_at_startup(state: &AppState) -> Result<(), AppError> {
    state
        .calendar_logs
        .scope(serde_json::json!({"step": "startup_recovery"}), async {
            crate::features::calendar_logs::operation(
                "recover_expired_leases_at_startup",
                serde_json::json!({}),
                async {
                    if !state.config.calendar_day.active() {
                        return Ok(());
                    }
                    work_store(state)
                        .recover_expired_leases(OffsetDateTime::now_utc())
                        .await
                        .map(|_| ())
                },
            )
            .await
        })
        .await
}

/// Lance le worker durable et recupere les leases expirees avant toute nouvelle
/// admission. Le handle reste optionnel seulement lorsque le calendrier est
/// explicitement desactive.
pub(crate) fn spawn(
    state: AppState,
    mut wake_rx: mpsc::Receiver<CalendarNativeWake>,
    mut shutdown_rx: watch::Receiver<bool>,
) -> Option<JoinHandle<()>> {
    if !state.config.calendar_day.active() {
        info!("calendar native worker disabled");
        return None;
    }

    Some(tokio::spawn(async move {
        let runtime = match NativeCalendarRuntime::from_state(&state) {
            Ok(runtime) => runtime,
            Err(_error) => {
                warn!("calendar native worker could not initialize its bounded dependencies");
                return;
            }
        };
        let worker_id = format!("calendar-native-{}", std::process::id());
        if let Err(_error) = runtime
            .work
            .recover_expired_leases(OffsetDateTime::now_utc())
            .await
        {
            warn!("calendar native worker could not recover expired leases at startup");
        }
        if let Err(_error) = drain_ready_work(&state, &runtime, &worker_id).await {
            warn!("calendar native worker startup drain failed");
        }

        let mut poll = tokio_time::interval(CALENDAR_NATIVE_POLL_INTERVAL);
        poll.set_missed_tick_behavior(MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                changed = shutdown_rx.changed() => {
                    match changed {
                        Ok(()) if *shutdown_rx.borrow() => {
                            info!("calendar native worker received shutdown signal");
                            break;
                        }
                        Ok(()) => {}
                        Err(_) => break,
                    }
                }
                received = wake_rx.recv() => {
                    if received.is_none() {
                        info!("calendar native wake queue closed");
                        break;
                    }
                    if let Err(_error) = drain_ready_work(&state, &runtime, &worker_id).await {
                        warn!("calendar native worker wake drain failed");
                    }
                }
                _ = poll.tick() => {
                    if let Err(_error) = runtime.work.recover_expired_leases(OffsetDateTime::now_utc()).await {
                        warn!("calendar native worker lease recovery failed");
                    }
                    if let Err(_error) = drain_ready_work(&state, &runtime, &worker_id).await {
                        warn!("calendar native worker poll drain failed");
                    }
                }
            }
        }
    }))
}

struct NativeCalendarRuntime {
    collectors: CalendarCollectorSet,
    openrouter: OpenRouterCalendarClient,
    work: CalendarWorkStore,
}

impl NativeCalendarRuntime {
    fn from_state(state: &AppState) -> Result<Self, AppError> {
        let collectors = CalendarCollectorSet::production()
            .map_err(|_| AppError::Config("calendar_native_collectors_unavailable".to_owned()))?;
        let config = &state.config.calendar_day.native;
        let endpoint = Url::parse(&config.endpoint).map_err(|_| {
            AppError::Config("calendar_native_openrouter_endpoint_invalid".to_owned())
        })?;
        let model = OpenRouterCalendarModel::new(config.model.clone())
            .map_err(|_| AppError::Config("calendar_native_openrouter_model_invalid".to_owned()))?;
        let limits = OpenRouterCalendarLimits::new(
            Duration::from_millis(config.request_timeout_ms),
            512 * 1024,
            config.max_response_bytes,
            config.retry_attempts,
            Duration::from_millis(config.retry_backoff_ms),
            1,
        )
        .map_err(|_| AppError::Config("calendar_native_openrouter_limits_invalid".to_owned()))?;
        let client_config =
            OpenRouterCalendarClientConfig::new(endpoint, model, limits).map_err(|_| {
                AppError::Config("calendar_native_openrouter_endpoint_invalid".to_owned())
            })?;
        let api_key = config.api_key.as_ref().ok_or_else(|| {
            AppError::Config(
                "OPENROUTER_API_KEY is required when FILFINANCE_CALENDAR_DAY_ENABLED=true"
                    .to_owned(),
            )
        })?;
        let api_key = OpenRouterApiKey::new(api_key.expose().to_owned()).map_err(|_| {
            AppError::Config("calendar_native_openrouter_api_key_invalid".to_owned())
        })?;
        // Ce client est volontairement distinct du client global FinancialJuice :
        // il ne partage donc jamais cookies ou session source avec le calendrier.
        let http = Client::builder()
            .cookie_store(false)
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| {
                AppError::Config("calendar_native_openrouter_http_client_invalid".to_owned())
            })?;
        Ok(Self {
            collectors,
            openrouter: OpenRouterCalendarClient::new(http, client_config, api_key),
            work: work_store(state),
        })
    }
}

fn work_store(state: &AppState) -> CalendarWorkStore {
    CalendarWorkStore::new(state.calendar_day.pool.clone())
}

async fn drain_ready_work(
    state: &AppState,
    runtime: &NativeCalendarRuntime,
    worker_id: &str,
) -> Result<(), AppError> {
    loop {
        let claimed = runtime
            .work
            .claim(
                worker_id,
                OffsetDateTime::now_utc(),
                CALENDAR_NATIVE_LEASE_DURATION,
                CALENDAR_NATIVE_CLAIM_LIMIT,
            )
            .await?;
        let Some(claimed) = claimed.into_iter().next() else {
            return Ok(());
        };
        state
            .calendar_logs
            .scope(
                serde_json::json!({
                    "local_day": claimed.work.local_day, "run_id": claimed.work.idempotency_key,
                    "work_id": claimed.work.id, "attempt": claimed.work.attempt_count,
                    "workflow_kind": format!("{:?}", claimed.work.kind)
                }),
                async {
                    let attempt_started = std::time::Instant::now();
                    crate::features::calendar_logs::event(
                        "workflow.attempt.started",
                        serde_json::json!({
                            "kind": format!("{:?}", claimed.work.kind),
                            "work_id": claimed.work.id,
                            "attempt": claimed.work.attempt_count,
                        }),
                    );
                    crate::features::calendar_logs::event(
                        "work.claimed",
                        serde_json::json!({"work_id": claimed.work.id, "kind": format!("{:?}", claimed.work.kind)}),
                    );
                    let execution = execute_claimed_work(state, runtime, &claimed).await;
                    let settled = settle_work(state, runtime, &claimed, execution).await;
                    crate::features::calendar_logs::event(
                        "workflow.attempt.finished",
                        serde_json::json!({
                            "kind": format!("{:?}", claimed.work.kind),
                            "work_id": claimed.work.id,
                            "attempt": claimed.work.attempt_count,
                            "outcome": format!("{execution:?}"),
                            "settled": settled.is_ok(),
                            "duration_ms": attempt_started.elapsed().as_millis(),
                        }),
                    );
                    settled
                },
            )
            .await?;
    }
}

#[derive(Debug, Clone, Copy)]
enum NativeWorkExecution {
    /// Le changement metier et l'acquittement de la lease ont ete commits dans
    /// une transaction SQLite unique. Il ne faut surtout pas tenter un second
    /// ack apres ce resultat, meme si le worker est reveille une seconde fois.
    Committed,
    Complete,
    Retry(&'static str),
    DeadLetter(&'static str),
    /// Le job audio a ete remis a Piper/HLS. Le worker audio acquittera la
    /// lease avec son contexte fence apres la persistence HLS.
    HandedOffToAudio,
}

async fn execute_claimed_work(
    state: &AppState,
    runtime: &NativeCalendarRuntime,
    claimed: &ClaimedCalendarWork,
) -> NativeWorkExecution {
    crate::features::calendar_logs::observe("execute_claimed_work", serde_json::json!({}), async {
        match claimed.work.kind {
            CalendarWorkKind::Ingestion => execute_ingestion(state, runtime, claimed).await,
            CalendarWorkKind::Summary => execute_summary(state, runtime, claimed).await,
            CalendarWorkKind::ItemAudio
            | CalendarWorkKind::AlertAudio
            | CalendarWorkKind::SummaryAudio => dispatch_audio(state, runtime, claimed).await,
        }
    })
    .await
}

async fn settle_work(
    state: &AppState,
    runtime: &NativeCalendarRuntime,
    claimed: &ClaimedCalendarWork,
    execution: NativeWorkExecution,
) -> Result<(), AppError> {
    crate::features::calendar_logs::operation("settle_work", serde_json::json!({}), async {
        let now = OffsetDateTime::now_utc();
        match execution {
            NativeWorkExecution::Committed => {}
            NativeWorkExecution::Complete => {
                let _ = runtime.work.complete(&claimed.lease, now).await?;
            }
            NativeWorkExecution::Retry(code) => {
                if claimed.work.attempt_count >= CALENDAR_NATIVE_MAX_WORK_ATTEMPTS {
                    terminalize_claimed_work(
                        state,
                        runtime,
                        claimed,
                        "calendar_native_retry_budget_exhausted",
                        now,
                    )
                    .await?;
                } else {
                    let code = CalendarWorkErrorCode::try_new(code)?;
                    let retry_at = now + CALENDAR_NATIVE_RETRY_BACKOFF;
                    let _ = runtime
                        .work
                        .retry(&claimed.lease, &code, retry_at, now)
                        .await?;
                }
            }
            NativeWorkExecution::DeadLetter(code) => {
                terminalize_claimed_work(state, runtime, claimed, code, now).await?;
            }
            NativeWorkExecution::HandedOffToAudio => {
                // La lease reste volontairement active : seul le worker Piper/HLS
                // peut l'acquitter apres avoir ecrit l'etat audio final.
            }
        }
        Ok(())
    })
    .await
}

/// Termine une intention dont l'execution n'a pas deja commite son etat.
///
/// Les deux etapes qui ont un etat visible (ingestion et resume) passent par
/// le commit transactionnel, y compris lorsqu'un retry epuise son budget. Les
/// travaux audio n'ont pas de changement de statut calendar a ce stade et
/// peuvent etre dead-letteres directement sous fencing.
async fn terminalize_claimed_work(
    state: &AppState,
    runtime: &NativeCalendarRuntime,
    claimed: &ClaimedCalendarWork,
    code: &'static str,
    now: OffsetDateTime,
) -> Result<(), AppError> {
    crate::features::calendar_logs::operation("terminalize_claimed_work", serde_json::json!({}), async {

    let error_code = CalendarWorkErrorCode::try_new(code)?;
    let workflow_kind = match claimed.work.kind {
        CalendarWorkKind::Ingestion => Some("ingestion"),
        CalendarWorkKind::Summary => Some("summary"),
        CalendarWorkKind::ItemAudio
        | CalendarWorkKind::AlertAudio
        | CalendarWorkKind::SummaryAudio => None,
    };
    let Some(workflow_kind) = workflow_kind else {
        let target_id = match work_target_id(&claimed.work.idempotency_key) {
            Ok(target_id) => target_id,
            // Une cle corrompue ne permet pas de retrouver une ligne audio
            // visible. Elle ne peut provenir que d'une ancienne base invalide;
            // on dead-lettere alors l'intention sans toucher un autre target.
            Err(_) => {
                let _ = runtime.work.fail(&claimed.lease, &error_code, now).await?;
                return Ok(());
            }
        };
        match state
            .calendar_day
            .commit_native_audio_terminal_error(
                &runtime.work,
                &claimed.lease,
                claimed.work.kind,
                target_id,
                &error_code,
                now,
            )
            .await
        {
            Ok(commit) => {
                if let Some(state_row) = commit.state {
                    calendar_day::emit_admin_event(
                        state,
                        CalendarDayAdminBroadcastEvent::StateUpdated(state_row),
                    );
                }
                if let Some(summary) = commit.summary {
                    calendar_day::emit_admin_event(
                        state,
                        CalendarDayAdminBroadcastEvent::SummaryUpdated(summary),
                    );
                }
                if let Some(run) = commit.run {
                    calendar_day::emit_admin_event(
                        state,
                        CalendarDayAdminBroadcastEvent::RunUpdated(run),
                    );
                }
            }
            Err(AppError::Conflict(message)) if message == "calendar_native_work_fenced_out" => {}
            Err(error) => return Err(error),
        }
        return Ok(());
    };

    match state
        .calendar_day
        .commit_native_terminal_error(
            &runtime.work,
            &claimed.lease,
            &claimed.work.idempotency_key,
            &claimed.work.local_day,
            workflow_kind,
            &[],
            &error_code,
            now,
        )
        .await
    {
        Ok((state_row, run)) => {
            calendar_day::emit_admin_event(
                state,
                CalendarDayAdminBroadcastEvent::StateUpdated(state_row),
            );
            calendar_day::emit_admin_event(state, CalendarDayAdminBroadcastEvent::RunUpdated(run));
            let _ = state.calendar_day_native.try_notify();
        }
        // Une lease perdue a deja ete reprise ou acquittee par un autre
        // worker. Ne jamais la muter une seconde fois.
        Err(error) => {
            // Le seul cas qui ne doit jamais redevenir retryable est un
            // refus source deja observe. Si le commit atomique a echoue, on
            // conserve au moins le verrou terminal de la file; l'admission
            // le relira avant toute nouvelle requete HTTP.
            if code == CALENDAR_SOURCE_REFUSAL_TERMINAL_ERROR {
                warn!(
                    %error,
                    local_day = %claimed.work.local_day,
                    "calendar native refusal terminal commit failed; attempting durable queue lock"
                );
                match runtime.work.fail(&claimed.lease, &error_code, now).await {
                    Ok(CalendarWorkLeaseMutation::Applied(_)) => {
                        match state
                            .calendar_day
                            .mark_native_ingestion_refusal_after_audit_persist_failure(
                                &claimed.work.idempotency_key,
                                &claimed.work.local_day,
                                now,
                            )
                            .await
                        {
                            Ok((state_row, run)) => {
                                calendar_day::emit_admin_event(
                                    state,
                                    CalendarDayAdminBroadcastEvent::StateUpdated(state_row),
                                );
                                calendar_day::emit_admin_event(
                                    state,
                                    CalendarDayAdminBroadcastEvent::RunUpdated(run),
                                );
                            }
                            Err(projection_error) => warn!(
                                %projection_error,
                                local_day = %claimed.work.local_day,
                                "calendar native refusal queue lock retained without admin state projection"
                            ),
                        }
                    }
                    Ok(CalendarWorkLeaseMutation::FencedOut) => {}
                    Err(lock_error) => warn!(
                        %lock_error,
                        local_day = %claimed.work.local_day,
                        "calendar native refusal queue lock could not be written"
                    ),
                }
            }
        }
    }
    Ok(())

}).await
}

/// Commit terminal avec les audits de collecte encore en memoire.
///
/// C'est indispensable pour que le refus anti-bot, le dead-letter et l'etat
/// visible soient atomiques. Le texte/HTML source ne sort jamais du collecteur.
async fn commit_terminal_calendar_work(
    state: &AppState,
    runtime: &NativeCalendarRuntime,
    claimed: &ClaimedCalendarWork,
    workflow_kind: &'static str,
    audits: &[CalendarCollectorAudit],
    code: &'static str,
) -> NativeWorkExecution {
    crate::features::calendar_logs::observe(
        "commit_terminal_calendar_work",
        serde_json::json!({}),
        async {
            commit_terminal_calendar_work_diagnostic(
                state,
                runtime,
                claimed,
                workflow_kind,
                audits,
                code,
                None,
            )
            .await
        },
    )
    .await
}

async fn commit_terminal_calendar_work_diagnostic(
    state: &AppState,
    runtime: &NativeCalendarRuntime,
    claimed: &ClaimedCalendarWork,
    workflow_kind: &'static str,
    audits: &[CalendarCollectorAudit],
    code: &'static str,
    diagnostic: Option<&super::diagnostics::CalendarIngestionDiagnostic>,
) -> NativeWorkExecution {
    crate::features::calendar_logs::observe("commit_terminal_calendar_work_diagnostic", serde_json::json!({}), async {

    let error_code = match CalendarWorkErrorCode::try_new(code) {
        Ok(code) => code,
        Err(_) => return NativeWorkExecution::Retry("calendar_native_terminal_error_code"),
    };
    match state
        .calendar_day
        .commit_native_terminal_error_with_diagnostic(
            &runtime.work,
            &claimed.lease,
            &claimed.work.idempotency_key,
            &claimed.work.local_day,
            workflow_kind,
            audits,
            &error_code,
            OffsetDateTime::now_utc(),
            diagnostic,
        )
        .await
    {
        Ok((state_row, run)) => {
            if diagnostic.is_some() {
                if let Ok(Some(artifact)) = sqlx::query_as::<_, calendar_day::CalendarDayRunArtifact>(
                    "SELECT * FROM calendar_day_run_artifacts WHERE run_id=?1 AND label='calendar_native_ingestion_diagnostic' LIMIT 1"
                ).bind(&run.run_id).fetch_optional(&state.calendar_day.pool).await {
                    calendar_day::emit_admin_event(state, CalendarDayAdminBroadcastEvent::Artifact(artifact));
                }
            }
            calendar_day::emit_admin_event(
                state,
                CalendarDayAdminBroadcastEvent::StateUpdated(state_row),
            );
            calendar_day::emit_admin_event(state, CalendarDayAdminBroadcastEvent::RunUpdated(run));
            if workflow_kind == "ingestion" && !audits.is_empty() {
                if let Ok(Some(artifact)) = state
                    .calendar_day
                    .latest_native_source_audit_artifact(&claimed.work.local_day)
                    .await
                {
                    calendar_day::emit_admin_event(
                        state,
                        CalendarDayAdminBroadcastEvent::Artifact(artifact),
                    );
                }
            }
            calendar_day::record_calendar_step(
                state,
                &claimed.work.idempotency_key,
                &claimed.work.local_day,
                "calendar_native_terminal",
                "error",
                "Le travail calendrier natif a ete termine avec une erreur redigee.",
                Some(code),
            )
            .await;
            let _ = state.calendar_day_native.try_notify();
            NativeWorkExecution::Committed
        }
        Err(_) => NativeWorkExecution::Retry("calendar_native_terminal_storage"),
    }

}).await
}

/// Verrouille un jour apres un refus source dont l'audit n'a pas pu etre
/// persiste. Le dead-letter de la file est volontairement ecrit avant la
/// projection d'etat admin : la prochaine admission relira ce verrou meme si
/// la table d'audit ou l'ecriture de l'etat reste indisponible.
async fn fail_closed_after_refused_source_audit_persist_failure(
    state: &AppState,
    work: &CalendarWorkStore,
    claimed: &ClaimedCalendarWork,
) -> NativeWorkExecution {
    crate::features::calendar_logs::observe("fail_closed_after_refused_source_audit_persist_failure", serde_json::json!({}), async {

    let error_code = match CalendarWorkErrorCode::try_new(CALENDAR_SOURCE_REFUSAL_TERMINAL_ERROR) {
        Ok(code) => code,
        Err(_) => return NativeWorkExecution::DeadLetter("calendar_native_terminal_error_code"),
    };
    let now = OffsetDateTime::now_utc();
    match work.fail(&claimed.lease, &error_code, now).await {
        Ok(CalendarWorkLeaseMutation::Applied(_)) => {
            match state
                .calendar_day
                .mark_native_ingestion_refusal_after_audit_persist_failure(
                    &claimed.work.idempotency_key,
                    &claimed.work.local_day,
                    now,
                )
                .await
            {
                Ok((state_row, run)) => {
                    calendar_day::emit_admin_event(
                        state,
                        CalendarDayAdminBroadcastEvent::StateUpdated(state_row),
                    );
                    calendar_day::emit_admin_event(
                        state,
                        CalendarDayAdminBroadcastEvent::RunUpdated(run),
                    );
                    calendar_day::record_calendar_step(
                        state,
                        &claimed.work.idempotency_key,
                        &claimed.work.local_day,
                        "calendar_native_source_audit",
                        "error",
                        "Un refus de source a ete observe; le jour est termine sans nouvelle collecte.",
                        Some(CALENDAR_SOURCE_REFUSAL_TERMINAL_ERROR),
                    )
                    .await;
                    let _ = state.calendar_day_native.try_notify();
                }
                Err(error) => {
                    // Le dead-letter durable a deja ete commite. Ne jamais
                    // reessayer la collecte pour tenter de reparer une vue
                    // admin; elle sera rederivee a la prochaine lecture.
                    warn!(
                        %error,
                        local_day = %claimed.work.local_day,
                        "calendar native refusal terminal state projection failed after durable queue lock"
                    );
                }
            }
            NativeWorkExecution::Committed
        }
        // L'ancien worker a perdu son fencing token. Il ne doit ni ecrire un
        // etat global ni replanifier une nouvelle collecte.
        Ok(CalendarWorkLeaseMutation::FencedOut) => NativeWorkExecution::Complete,
        Err(error) => {
            // `DeadLetter` tente encore le commit transactionnel existant,
            // mais ne passe jamais par `Retry`: un refus observe est
            // terminal meme si le premier verrou SQLite a echoue.
            warn!(
                %error,
                local_day = %claimed.work.local_day,
                "calendar native source audit persistence failed and fail-closed queue lock was unavailable"
            );
            NativeWorkExecution::DeadLetter(CALENDAR_SOURCE_REFUSAL_TERMINAL_ERROR)
        }
    }

}).await
}

fn audits_contain_terminal_refusal(audits: &[CalendarCollectorAudit]) -> bool {
    audits
        .iter()
        .any(|audit| matches!(audit.outcome, CalendarCollectorOutcome::Refused(_)))
}

async fn execute_ingestion(
    state: &AppState,
    runtime: &NativeCalendarRuntime,
    claimed: &ClaimedCalendarWork,
) -> NativeWorkExecution {
    crate::features::calendar_logs::observe("execute_ingestion", serde_json::json!({}), async {

    let local_day = match CalendarLocalDay::parse(&claimed.work.local_day) {
        Ok(day) => day,
        Err(_) => return NativeWorkExecution::DeadLetter("calendar_native_invalid_local_day"),
    };
    let run_id = claimed.work.idempotency_key.as_str();
    match state.calendar_day.get_run(run_id).await {
        Ok(Some(run)) if matches!(run.status.as_str(), "ready" | "error") => {
            return NativeWorkExecution::Complete;
        }
        Ok(_) => {}
        Err(_) => return NativeWorkExecution::Retry("calendar_native_run_read_failed"),
    }
    let quarantines = match state
        .calendar_day
        .native_collector_quarantines(&claimed.work.local_day)
        .await
    {
        Ok(quarantines) => quarantines,
        Err(_) => {
            return NativeWorkExecution::Retry("calendar_native_source_quarantine_read_failed");
        }
    };
    let collected = runtime
        .collectors
        .collect_with_quarantines(local_day, &quarantines)
        .await;
    let audits = collected.audits().to_vec();
    // Cette ecriture precede OpenRouter: un refus terminal est ainsi visible
    // pour le prochain claim meme si l'IA demande ensuite un retry. Sans elle,
    // une source 403/challenge pourrait etre appelee une seconde fois le meme
    // jour, ce que le chemin natif ne doit jamais faire.
    let source_audit_artifact = match state
        .calendar_day
        .persist_native_source_audits(
            &runtime.work,
            &claimed.lease,
            run_id,
            &claimed.work.local_day,
            &audits,
            OffsetDateTime::now_utc(),
        )
        .await
    {
        Ok(artifact) => artifact,
        Err(_) if audits_contain_terminal_refusal(&audits) => {
            return fail_closed_after_refused_source_audit_persist_failure(
                state,
                &runtime.work,
                claimed,
            )
            .await;
        }
        Err(_) => return NativeWorkExecution::Retry("calendar_native_source_audit_persist_failed"),
    };
    calendar_day::emit_admin_event(
        state,
        CalendarDayAdminBroadcastEvent::Artifact(source_audit_artifact),
    );
    if let Some(reason) = collected.required_source_failure() {
        return commit_terminal_calendar_work_diagnostic(
            state,
            runtime,
            claimed,
            "ingestion",
            &audits,
            "calendar_native_required_source_unavailable",
            Some(&super::diagnostics::CalendarIngestionDiagnostic::new(
                reason,
            )),
        )
        .await;
    }
    let covered_sources = match collected {
        super::collectors::CalendarCollectorSetResult::Covered { sources, .. } => sources,
        super::collectors::CalendarCollectorSetResult::NoCoverage { .. } => {
            return commit_terminal_calendar_work(
                state,
                runtime,
                claimed,
                "ingestion",
                &audits,
                no_coverage_error_code(&audits),
            )
            .await;
        }
    };

    let mut snapshots = Vec::with_capacity(covered_sources.len());
    let mut source_bindings = Vec::with_capacity(covered_sources.len());
    for covered in &covered_sources {
        let snapshot = match snapshot_from_covered_source(&claimed.work.local_day, covered) {
            Ok(snapshot) => snapshot,
            Err(_) => {
                return commit_terminal_calendar_work(
                    state,
                    runtime,
                    claimed,
                    "ingestion",
                    &audits,
                    "calendar_native_source_contract_invalid",
                )
                .await;
            }
        };
        let binding = match source_binding_from_covered_source(&claimed.work.local_day, covered) {
            Ok(binding) => binding,
            Err(_) => {
                return commit_terminal_calendar_work(
                    state,
                    runtime,
                    claimed,
                    "ingestion",
                    &audits,
                    "calendar_native_source_contract_invalid",
                )
                .await;
            }
        };
        snapshots.push(snapshot);
        source_bindings.push(binding);
    }
    if snapshots.is_empty() {
        return commit_terminal_calendar_work(
            state,
            runtime,
            claimed,
            "ingestion",
            &audits,
            "calendar_native_source_contract_invalid",
        )
        .await;
    }

    let prompt = match build_ingestion_prompt(&claimed.work.local_day, &snapshots) {
        Ok(prompt) => prompt,
        Err(_) => {
            return commit_terminal_calendar_work(
                state,
                runtime,
                claimed,
                "ingestion",
                &audits,
                "calendar_native_prompt_invalid",
            )
            .await;
        }
    };
    crate::features::calendar_logs::event("ingestion.prompt_built", serde_json::json!({
        "bytes": prompt.content().len(),
        "source_count": snapshots.len(),
        "full_request_action": "openrouter.request",
    }));
    let request = match OpenRouterCalendarRequest::ingestion(prompt.content()) {
        Ok(request) => request,
        Err(_) => return NativeWorkExecution::DeadLetter("calendar_native_prompt_invalid"),
    };
    let completion = match runtime.openrouter.complete_json(&request).await {
        Ok(completion) => completion,
        Err(error) if is_retryable_openrouter_error(&error) => {
            if claimed.work.attempt_count >= CALENDAR_NATIVE_MAX_WORK_ATTEMPTS {
                let diagnostic = super::diagnostics::CalendarIngestionDiagnostic::openrouter(error);
                return commit_terminal_calendar_work_diagnostic(
                    state,
                    runtime,
                    claimed,
                    "ingestion",
                    &audits,
                    "calendar_native_openrouter_retries_exhausted",
                    Some(&diagnostic),
                )
                .await;
            }
            return NativeWorkExecution::Retry("calendar_native_openrouter_retry");
        }
        Err(error) => {
            let diagnostic = super::diagnostics::CalendarIngestionDiagnostic::openrouter(error);
            return commit_terminal_calendar_work_diagnostic(
                state,
                runtime,
                claimed,
                "ingestion",
                &audits,
                "calendar_native_openrouter_failed",
                Some(&diagnostic),
            )
            .await;
        }
    };
    let mut draft = match contracts::canonicalize_ingestion_output(
        &claimed.work.local_day,
        &snapshots,
        completion.content(),
    ) {
        Ok(draft) => draft,
        Err(error) => {
            let mut diagnostic = super::diagnostics::CalendarIngestionDiagnostic::contract(&error);
            diagnostic.finish_reason = Some(super::openrouter::CalendarFinishReason::Stop);
            return commit_terminal_calendar_work_diagnostic(
                state,
                runtime,
                claimed,
                "ingestion",
                &audits,
                "calendar_native_ingestion_rejected",
                Some(&diagnostic),
            )
            .await;
        }
    };
    crate::features::calendar_logs::event("ingestion.rows.normalized", serde_json::json!({
        "row_count": draft.items.len(), "high_count": draft.high_count(),
    }));
    let documents = covered_sources
        .iter()
        .filter_map(|source| source.document())
        .collect::<Vec<_>>();
    let output_valid = super::feeds::validate_output(&draft, &documents);
    crate::features::calendar_logs::event(
        "ingestion.output_validation",
        serde_json::json!({ "valid": output_valid, "document_count": documents.len() }),
    );
    if !output_valid {
        return commit_terminal_calendar_work(
            state,
            runtime,
            claimed,
            "ingestion",
            &audits,
            "calendar_native_ingestion_provenance_rejected",
        )
        .await;
    }
    let provenance_valid = contracts::project_native_ingestion_provenance(&mut draft, &source_bindings).is_ok();
    crate::features::calendar_logs::event(
        "ingestion.provenance_validation",
        serde_json::json!({ "valid": provenance_valid, "binding_count": source_bindings.len() }),
    );
    if !provenance_valid {
        return commit_terminal_calendar_work(
            state,
            runtime,
            claimed,
            "ingestion",
            &audits,
            "calendar_native_ingestion_provenance_rejected",
        )
        .await;
    }
    for (index, item) in draft.items.iter().enumerate() {
        crate::features::calendar_logs::event("ingestion.row.processed", serde_json::json!({
            "row_index": index + 1,
            "event_time": item.event_time,
            "source_title": item.source_title,
            "final_importance": item.final_importance,
            "status": "validated",
        }));
    }
    match state
        .calendar_day
        .commit_native_ingestion(
            &runtime.work,
            &claimed.lease,
            run_id,
            &draft,
            &audits,
            &state.config.calendar_day.alert_offsets_secs,
            OffsetDateTime::now_utc(),
        )
        .await
    {
        Ok(committed) => {
            crate::features::calendar_logs::event("ingestion.rows.persisted", serde_json::json!({
                "row_count": committed.items.len(),
                "high_count": committed.items.iter().filter(|item| item.final_importance == "high").count(),
            }));
            calendar_day::emit_admin_event(
                state,
                CalendarDayAdminBroadcastEvent::RunStarted(committed.summary_run.clone()),
            );
            calendar_day::emit_admin_event(
                state,
                CalendarDayAdminBroadcastEvent::StateUpdated(committed.state),
            );
            calendar_day::emit_admin_event(
                state,
                CalendarDayAdminBroadcastEvent::ItemsUpdated(committed.items.clone()),
            );
            calendar_day::emit_admin_event(
                state,
                CalendarDayAdminBroadcastEvent::Artifact(committed.source_audit_artifact.clone()),
            );
            calendar_day::emit_admin_event(
                state,
                CalendarDayAdminBroadcastEvent::RunUpdated(committed.ingestion_run),
            );
            let high_count = committed
                .items
                .iter()
                .filter(|item| item.final_importance == "high")
                .count();
            calendar_day::record_calendar_step(
                state,
                run_id,
                &claimed.work.local_day,
                "calendar_native_ingestion_committed",
                "ok",
                "Les evenements calendrier natifs et les travaux aval ont ete persistés.",
                Some(&format!(
                    "item_count={} high_count={} summary_run={}",
                    committed.items.len(),
                    high_count,
                    committed.summary_run.run_id
                )),
            )
            .await;
            let _ = state.calendar_day_native.try_notify();
            NativeWorkExecution::Committed
        }
        Err(_) => NativeWorkExecution::Retry("calendar_native_ingestion_storage"),
    }

}).await
}

async fn execute_summary(
    state: &AppState,
    runtime: &NativeCalendarRuntime,
    claimed: &ClaimedCalendarWork,
) -> NativeWorkExecution {
    crate::features::calendar_logs::observe("execute_summary", serde_json::json!({}), async {
        let local_day = claimed.work.local_day.as_str();
        let run_id = claimed.work.idempotency_key.as_str();
        match state.calendar_day.get_run(run_id).await {
            Ok(Some(run))
                if matches!(run.status.as_str(), "audio_generating" | "ready" | "error") =>
            {
                return NativeWorkExecution::Complete;
            }
            Ok(_) => {}
            Err(_) => return NativeWorkExecution::Retry("calendar_native_run_read_failed"),
        }
        let items = match state.calendar_day.list_items(local_day).await {
            Ok(items) if !items.is_empty() => items,
            Ok(_) => {
                return commit_terminal_calendar_work(
                    state,
                    runtime,
                    claimed,
                    "summary",
                    &[],
                    "calendar_native_summary_no_items",
                )
                .await;
            }
            Err(_) => return NativeWorkExecution::Retry("calendar_native_summary_storage"),
        };
        let input_items = match items
            .iter()
            .map(summary_input_item)
            .collect::<Result<Vec<_>, _>>()
        {
            Ok(items) => items,
            Err(_) => {
                return NativeWorkExecution::DeadLetter("calendar_native_summary_input_invalid");
            }
        };
        crate::features::calendar_logs::event("summary.input_rows", serde_json::json!({
            "row_count": input_items.len(),
            "high_count": input_items.iter().filter(|item| item.final_importance == CalendarImportance::High).count(),
        }));
        let prompt = match build_summary_prompt(local_day, &input_items) {
            Ok(prompt) => prompt,
            Err(_) => {
                return NativeWorkExecution::DeadLetter("calendar_native_summary_prompt_invalid");
            }
        };
        let request = match OpenRouterCalendarRequest::summary(prompt.content()) {
            Ok(request) => request,
            Err(_) => {
                return NativeWorkExecution::DeadLetter("calendar_native_summary_prompt_invalid");
            }
        };
        crate::features::calendar_logs::event("summary.prompt_built", serde_json::json!({
            "bytes": prompt.content().len(),
            "full_request_action": "openrouter.request",
        }));
        let completion = match runtime.openrouter.complete_json(&request).await {
            Ok(completion) => completion,
            Err(error) if is_retryable_openrouter_error(&error) => {
                if claimed.work.attempt_count >= CALENDAR_NATIVE_MAX_WORK_ATTEMPTS {
                    return commit_terminal_calendar_work(
                        state,
                        runtime,
                        claimed,
                        "summary",
                        &[],
                        "calendar_native_openrouter_retries_exhausted",
                    )
                    .await;
                }
                return NativeWorkExecution::Retry("calendar_native_openrouter_retry");
            }
            Err(_) => {
                return commit_terminal_calendar_work(
                    state,
                    runtime,
                    claimed,
                    "summary",
                    &[],
                    "calendar_native_openrouter_failed",
                )
                .await;
            }
        };
        let draft = match contracts::canonicalize_summary_output(
            local_day,
            &input_items,
            completion.content(),
        ) {
            Ok(draft) => draft,
            Err(_) => {
                return commit_terminal_calendar_work(
                    state,
                    runtime,
                    claimed,
                    "summary",
                    &[],
                    "calendar_native_summary_rejected",
                )
                .await;
            }
        };
        crate::features::calendar_logs::event("summary.validated", serde_json::json!({
            "input_item_count": draft.input_item_count,
            "input_high_item_count": draft.input_high_item_count,
        }));
        match state
            .calendar_day
            .commit_native_summary(
                &runtime.work,
                &claimed.lease,
                run_id,
                &draft,
                &state.config.calendar_day.native.model,
                OffsetDateTime::now_utc(),
            )
            .await
        {
            Ok(committed) => {
                crate::features::calendar_logs::event("summary.persisted", serde_json::json!({
                    "summary_id": committed.summary.id,
                    "input_item_count": draft.input_item_count,
                }));
                calendar_day::emit_admin_event(
                    state,
                    CalendarDayAdminBroadcastEvent::StateUpdated(committed.state),
                );
                calendar_day::emit_admin_event(
                    state,
                    CalendarDayAdminBroadcastEvent::SummaryUpdated(committed.summary.clone()),
                );
                calendar_day::emit_admin_event(
                    state,
                    CalendarDayAdminBroadcastEvent::RunUpdated(committed.run),
                );
                calendar_day::record_calendar_step(
                    state,
                    run_id,
                    local_day,
                    "calendar_native_summary_committed",
                    "ok",
                    "Le resume calendrier natif et son travail audio ont ete persistés.",
                    Some(&format!("summary_id={}", committed.summary.id)),
                )
                .await;
                let _ = state.calendar_day_native.try_notify();
                NativeWorkExecution::Committed
            }
            Err(_) => NativeWorkExecution::Retry("calendar_native_summary_storage"),
        }
    })
    .await
}

async fn dispatch_audio(
    state: &AppState,
    runtime: &NativeCalendarRuntime,
    claimed: &ClaimedCalendarWork,
) -> NativeWorkExecution {
    crate::features::calendar_logs::observe("dispatch_audio", serde_json::json!({}), async {
        let candidate = match audio_stream_candidate(state, claimed).await {
            Ok(candidate) => candidate,
            Err(_) => {
                return NativeWorkExecution::Retry("calendar_native_stream_candidate_storage");
            }
        };
        let reserved = match runtime
            .work
            .reserve_stream_id(&claimed.lease, candidate, OffsetDateTime::now_utc())
            .await
        {
            Ok(CalendarWorkLeaseMutation::Applied(work)) => match work.reserved_stream_id {
                Some(stream_id) => stream_id,
                None => {
                    return NativeWorkExecution::Retry(
                        "calendar_native_stream_reservation_missing",
                    );
                }
            },
            Ok(CalendarWorkLeaseMutation::FencedOut) => return NativeWorkExecution::Complete,
            Err(_) => {
                return NativeWorkExecution::Retry("calendar_native_stream_reservation_storage");
            }
        };
        let job = match audio_job_for_work(state, claimed, reserved).await {
            Ok(Some(job)) => job,
            Ok(None) => return NativeWorkExecution::Complete,
            Err(_) => return NativeWorkExecution::Retry("calendar_native_audio_input_storage"),
        };
        match state.calendar_day_audio_tx.try_send(job) {
            Ok(()) => NativeWorkExecution::HandedOffToAudio,
            Err(mpsc::error::TrySendError::Full(_)) => {
                NativeWorkExecution::Retry("calendar_native_audio_queue_full")
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                NativeWorkExecution::Retry("calendar_native_audio_queue_closed")
            }
        }
    })
    .await
}

/// Reutilise d'abord le stream expose par un resume historique dont le
/// manifeste a disparu. S'il n'existe aucune reservation fiable, le candidat
/// depend seulement de l'identifiant durable du travail et reste donc stable
/// entre redemarrages.
async fn audio_stream_candidate(
    state: &AppState,
    claimed: &ClaimedCalendarWork,
) -> Result<String, AppError> {
    crate::features::calendar_logs::operation(
        "audio_stream_candidate",
        serde_json::json!({}),
        async {
            let fallback = || {
                format!(
                    "calendar-native-{}-{}",
                    claimed.work.kind.as_str(),
                    claimed.work.id
                )
            };
            if claimed.work.kind != CalendarWorkKind::SummaryAudio {
                return Ok(fallback());
            }
            let summary_id = work_target_id(&claimed.work.idempotency_key)?;
            let Some(summary) = state.calendar_day.get_summary_by_id(summary_id).await? else {
                return Ok(fallback());
            };
            Ok(
                super::work::valid_reserved_stream_id(summary.stream_id.as_deref())
                    .unwrap_or_else(fallback),
            )
        },
    )
    .await
}

async fn audio_job_for_work(
    state: &AppState,
    claimed: &ClaimedCalendarWork,
    reserved_stream_id: String,
) -> Result<Option<CalendarDayAudioJob>, AppError> {
    crate::features::calendar_logs::operation("audio_job_for_work", serde_json::json!({}), async {
        let local_day = claimed.work.local_day.clone();
        let id = work_target_id(&claimed.work.idempotency_key)?;
        let native_work = CalendarAudioWorkContext::from_claimed(claimed);
        let job = match claimed.work.kind {
            CalendarWorkKind::ItemAudio => {
                let Some(item) = state.calendar_day.get_item_by_id(id).await? else {
                    return Ok(None);
                };
                if item.final_importance != "high" {
                    return Ok(None);
                }
                CalendarDayAudioJob {
                    native_work: native_work.clone(),
                    reserved_stream_id,
                    local_day,
                    kind: CalendarDayAudioKind::Item,
                    item_id: Some(item.id),
                    summary_id: None,
                    alert_id: None,
                    title: item.translated_title,
                    narration_text: item.tts_text,
                    timestamp: item
                        .event_time
                        .unwrap_or_else(crate::shared::time::now_rfc3339),
                }
            }
            CalendarWorkKind::AlertAudio => {
                let Some(alert) = state.calendar_day.get_alert_by_id(id).await? else {
                    return Ok(None);
                };
                let Some(item) = state
                    .calendar_day
                    .get_item_by_id(alert.calendar_item_id)
                    .await?
                else {
                    return Ok(None);
                };
                CalendarDayAudioJob {
                    native_work: native_work.clone(),
                    reserved_stream_id,
                    local_day,
                    kind: CalendarDayAudioKind::Alert,
                    item_id: Some(item.id),
                    summary_id: None,
                    alert_id: Some(alert.id),
                    title: format!(
                        "Alerte calendrier dans {} minute{}",
                        alert.alert_offset_secs / 60,
                        if alert.alert_offset_secs > 60 {
                            "s"
                        } else {
                            ""
                        }
                    ),
                    narration_text: format!(
                        "Alerte calendrier. Dans {} minute{}, {}.",
                        alert.alert_offset_secs / 60,
                        if alert.alert_offset_secs > 60 {
                            "s"
                        } else {
                            ""
                        },
                        item.tts_text
                    ),
                    timestamp: alert.scheduled_at,
                }
            }
            CalendarWorkKind::SummaryAudio => {
                let Some(summary) = state.calendar_day.get_summary_by_id(id).await? else {
                    return Ok(None);
                };
                CalendarDayAudioJob {
                    native_work,
                    reserved_stream_id,
                    local_day,
                    kind: CalendarDayAudioKind::Summary,
                    item_id: None,
                    summary_id: Some(summary.id),
                    alert_id: None,
                    title: summary.title,
                    narration_text: summary.narration_text,
                    timestamp: crate::shared::time::now_rfc3339(),
                }
            }
            CalendarWorkKind::Ingestion | CalendarWorkKind::Summary => return Ok(None),
        };
        Ok(Some(job))
    })
    .await
}

fn work_target_id(key: &str) -> Result<i64, AppError> {
    let (_, raw_id) = key
        .rsplit_once('-')
        .ok_or_else(|| AppError::storage("calendar_native_work_target_invalid"))?;
    raw_id
        .parse::<i64>()
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| AppError::storage("calendar_native_work_target_invalid"))
}

fn snapshot_from_covered_source(
    local_day: &str,
    covered: &super::collectors::CalendarCoveredSource,
) -> Result<CalendarSourceSnapshot, AppError> {
    let source = covered.source().source_kind();
    let url = covered.source().production_url_for_day(covered.local_day());
    CalendarSourceSnapshot::from_response_text(
        local_day,
        source,
        url.as_str(),
        200,
        covered.raw_text(),
    )
    .map_err(|_| AppError::storage("calendar_native_source_snapshot_invalid"))
}

/// Lie la provenance persistable a l'URL de production du collecteur qui a
/// effectivement couvert ce jour. L'IA ne fournit jamais cette URL.
fn source_binding_from_covered_source(
    local_day: &str,
    covered: &super::collectors::CalendarCoveredSource,
) -> Result<CalendarCoveredSourceBinding, AppError> {
    let source = covered.source().source_kind();
    let url = covered.source().production_url_for_day(covered.local_day());
    CalendarCoveredSourceBinding::for_local_day(local_day, source, url)
        .map_err(|_| AppError::storage("calendar_native_source_binding_invalid"))
}

fn summary_input_item(item: &CalendarDayItem) -> Result<CalendarSummaryInputItem, AppError> {
    let final_importance = match item.final_importance.as_str() {
        "low" => CalendarImportance::Low,
        "medium" => CalendarImportance::Medium,
        "high" => CalendarImportance::High,
        _ => {
            return Err(AppError::storage(
                "calendar_native_summary_importance_invalid",
            ));
        }
    };
    Ok(CalendarSummaryInputItem {
        id: item.id,
        local_day: item.local_day.clone(),
        event_time: item.event_time.clone(),
        source_title: item.source_title.clone(),
        translated_title: item.translated_title.clone(),
        tts_text: item.tts_text.clone(),
        country: item.country.clone(),
        currency: item.currency.clone(),
        forecast_value: item.forecast_value.clone(),
        previous_value: item.previous_value.clone(),
        actual_value: item.actual_value.clone(),
        source_importance: item.source_importance.clone(),
        ai_importance: item.ai_importance.clone(),
        final_importance,
        category: item.category.clone(),
        reasoning: item.reasoning.clone(),
        confidence: item.confidence,
        content_hash: item.content_hash.clone(),
        sources_json: item.sources_json.clone(),
        created_at: item.created_at.clone(),
        updated_at: item.updated_at.clone(),
    })
}

fn is_retryable_openrouter_error(error: &OpenRouterCalendarError) -> bool {
    matches!(
        error,
        OpenRouterCalendarError::Busy | OpenRouterCalendarError::RetriesExhausted { .. }
    )
}

fn no_coverage_error_code(audits: &[CalendarCollectorAudit]) -> &'static str {
    if audits
        .iter()
        .any(|audit| matches!(audit.outcome, CalendarCollectorOutcome::Refused(_)))
    {
        "calendar_native_sources_refused"
    } else {
        "calendar_native_sources_unavailable"
    }
}

async fn record_native_admission(
    state: &AppState,
    run: &CalendarDayRun,
    stage: &str,
    admission: &CalendarWorkAdmission,
) {
    let outcome = if admission.inserted() {
        "inserted"
    } else {
        "existing"
    };
    calendar_day::record_calendar_step(
        state,
        &run.run_id,
        &run.local_day,
        "calendar_native_work_admitted",
        "info",
        "Le travail calendrier Rust est durablement admis.",
        Some(&format!("stage={stage} outcome={outcome}")),
    )
    .await;
}

async fn record_native_notify(
    state: &AppState,
    run: &CalendarDayRun,
    outcome: CalendarNativeNotifyOutcome,
) {
    let (status, detail) = match outcome {
        CalendarNativeNotifyOutcome::Disabled => ("warning", "disabled"),
        CalendarNativeNotifyOutcome::Enqueued => return,
        CalendarNativeNotifyOutcome::Full => ("warning", "full"),
        CalendarNativeNotifyOutcome::Closed => ("warning", "closed"),
    };
    calendar_day::record_calendar_step(
        state,
        &run.run_id,
        &run.local_day,
        "calendar_native_worker_notification",
        status,
        "Le reveil memoire du calendrier est indisponible; SQLite sera repris sans bloquer le run.",
        Some(&format!("outcome={detail}")),
    )
    .await;
}

// Le scenario de reprise vit dans un sous-module de test afin de pouvoir
// exercer les commits canoniques prives sans les transformer en API runtime.
#[cfg(test)]
mod recovery_e2e;

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::features::calendar_day::native::collectors::{
        CalendarCollectorAudit, CalendarCollectorOutcome, CalendarCollectorRefusalReason,
        CalendarCollectorSource, CalendarCollectorUnavailableReason,
    };
    use crate::features::calendar_day::native::work::CalendarWorkRequest;

    #[test]
    fn one_refused_source_makes_the_day_terminal_even_if_the_fallback_is_unavailable() {
        let audits = [
            CalendarCollectorAudit {
                source: CalendarCollectorSource::Feed(crate::features::calendar_day::native::feeds::CalendarFeedSource::BeaJson),
                outcome: CalendarCollectorOutcome::Refused(
                    CalendarCollectorRefusalReason::Forbidden,
                ),
                http_status: Some(403),
                raw_sha256: None,
                raw_length_bytes: None,
            },
            CalendarCollectorAudit {
                source: CalendarCollectorSource::Feed(crate::features::calendar_day::native::feeds::CalendarFeedSource::ForexFactoryExport),
                outcome: CalendarCollectorOutcome::Unavailable(
                    CalendarCollectorUnavailableReason::Transport,
                ),
                http_status: None,
                raw_sha256: None,
                raw_length_bytes: None,
            },
        ];

        assert_eq!(
            no_coverage_error_code(&audits),
            "calendar_native_sources_refused"
        );
    }

    #[test]
    fn only_a_terminal_refusal_uses_the_fail_closed_audit_persistence_path() {
        let unavailable = [CalendarCollectorAudit {
            source: CalendarCollectorSource::Feed(
                crate::features::calendar_day::native::feeds::CalendarFeedSource::BeaJson,
            ),
            outcome: CalendarCollectorOutcome::Unavailable(
                CalendarCollectorUnavailableReason::Transport,
            ),
            http_status: None,
            raw_sha256: None,
            raw_length_bytes: None,
        }];
        let refused = [CalendarCollectorAudit {
            source: CalendarCollectorSource::Feed(crate::features::calendar_day::native::feeds::CalendarFeedSource::ForexFactoryExport),
            outcome: CalendarCollectorOutcome::Refused(CalendarCollectorRefusalReason::RateLimited),
            http_status: Some(429),
            raw_sha256: None,
            raw_length_bytes: None,
        }];

        assert!(!audits_contain_terminal_refusal(&unavailable));
        assert!(audits_contain_terminal_refusal(&refused));
    }

    #[tokio::test]
    async fn refused_source_audit_persistence_failure_fails_closed_without_a_new_admission() {
        let state = crate::features::test_support::app_state().await;
        let local_day = "2026-06-18";
        let now = OffsetDateTime::now_utc();
        let run = state
            .calendar_day
            .insert_run(local_day, "ingestion", "refusal_audit_write_failure", 0)
            .await
            .expect("ingestion run");
        let work = work_store(&state);
        let request =
            CalendarWorkRequest::try_new(CalendarWorkKind::Ingestion, local_day, &run.run_id)
                .expect("ingestion request");
        work.admit(&request, now)
            .await
            .expect("ingestion work admission");
        let claimed = work
            .claim("refusal-audit-failure", now, Duration::from_secs(60), 1)
            .await
            .expect("ingestion work claim")
            .pop()
            .expect("one ingestion work");

        // Simule une indisponibilite limitee a la persistance des audits :
        // l'ecriture du verrou dans `calendar_native_work` doit rester
        // possible et empecher toute seconde collecte.
        sqlx::query(
            r#"
            CREATE TRIGGER reject_native_source_audit_for_test
            BEFORE INSERT ON calendar_day_source_audits
            BEGIN
                SELECT RAISE(ABORT, 'source_audit_unavailable');
            END
            "#,
        )
        .execute(&state.calendar_day.pool)
        .await
        .expect("audit rejection trigger");
        let refusal = CalendarCollectorAudit {
            source: CalendarCollectorSource::Feed(crate::features::calendar_day::native::feeds::CalendarFeedSource::ForexFactoryExport),
            outcome: CalendarCollectorOutcome::Refused(CalendarCollectorRefusalReason::Forbidden),
            http_status: Some(403),
            raw_sha256: None,
            raw_length_bytes: None,
        };
        assert!(
            state
                .calendar_day
                .persist_native_source_audits(
                    &work,
                    &claimed.lease,
                    &run.run_id,
                    local_day,
                    &[refusal],
                    now,
                )
                .await
                .is_err()
        );

        assert!(matches!(
            fail_closed_after_refused_source_audit_persist_failure(&state, &work, &claimed).await,
            NativeWorkExecution::Committed
        ));

        let state_row = state
            .calendar_day
            .get_state(local_day)
            .await
            .expect("calendar state")
            .expect("calendar state must exist");
        assert_eq!(state_row.calendar_ingestion_status, "error");
        assert_eq!(
            state_row.last_error.as_deref(),
            Some(CALENDAR_SOURCE_REFUSAL_TERMINAL_ERROR)
        );
        let run = state
            .calendar_day
            .get_run(&run.run_id)
            .await
            .expect("run")
            .expect("run must exist");
        assert_eq!(run.status, "error");
        assert_eq!(
            run.error_message.as_deref(),
            Some(CALENDAR_SOURCE_REFUSAL_TERMINAL_ERROR)
        );
        let (work_status, work_error): (String, Option<String>) = sqlx::query_as(
            "SELECT status, last_error_code FROM calendar_native_work WHERE id = ?1",
        )
        .bind(claimed.work.id)
        .fetch_one(&state.calendar_day.pool)
        .await
        .expect("durable refusal work");
        assert_eq!(work_status, "dead_letter");
        assert_eq!(
            work_error.as_deref(),
            Some(CALENDAR_SOURCE_REFUSAL_TERMINAL_ERROR)
        );
        assert!(
            work.has_terminal_ingestion_source_refusal(local_day)
                .await
                .expect("durable refusal lookup")
        );
        let persisted_audits: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM calendar_day_source_audits WHERE local_day = ?1",
        )
        .bind(local_day)
        .fetch_one(&state.calendar_day.pool)
        .await
        .expect("source audit count");
        assert_eq!(persisted_audits, 0);

        // La sentinelle de file suffit meme si une projection admin perdait
        // son erreur : une relance forcee ne cree aucun nouveau run ni job.
        sqlx::query("UPDATE calendar_day_state SET last_error = NULL WHERE local_day = ?1")
            .bind(local_day)
            .execute(&state.calendar_day.pool)
            .await
            .expect("simulate a missing admin error projection");
        assert!(matches!(
            admit_ingestion(&state, local_day, "forced_after_refusal", true, 0)
                .await
                .expect("admission after refusal"),
            CalendarDayIngestTriggerOutcome::Skipped { ref reason }
                if reason == "sources_refused_terminal"
        ));
        let work_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM calendar_native_work WHERE local_day = ?1 AND kind = 'ingestion'",
        )
        .bind(local_day)
        .fetch_one(&state.calendar_day.pool)
        .await
        .expect("ingestion work count");
        assert_eq!(work_count, 1);
    }
}
