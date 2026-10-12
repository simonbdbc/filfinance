//! Generation audio Piper du calendrier du jour.

use std::time::Instant;

use time::{Duration as TimeDuration, OffsetDateTime};
use tokio::{
    sync::{mpsc, watch},
    task::JoinHandle,
};
use tracing::{info, warn};

use crate::{
    app_state::AppState,
    errors::AppError,
    features::{
        audio::{
            domain::{AudioStartPayload, CalendarDayAudioJob, CalendarDayAudioKind},
            service::segmented_tts::synthesize_punctuation_segmented,
            tts::{
                normalization::normalize_narration_text_for_tts,
                provider::{SynthesizedAudio, TtsProvider},
            },
        },
        calendar_day::{
            self, CalendarDayAdminBroadcastEvent, CalendarNativeAudioReadyTarget,
            native::work::{
                CalendarWorkErrorCode, CalendarWorkKind, CalendarWorkLeaseMutation,
                CalendarWorkStore,
            },
        },
        glossary::{domain::GlossaryListParams, pipeline::apply_spoken_glossary},
    },
};

/// Le nombre de passages audio reste borne, meme quand Piper ou HLS est
/// temporairement indisponible. Cette limite est independante du flux live.
const CALENDAR_NATIVE_AUDIO_MAX_ATTEMPTS: u32 = 3;
const CALENDAR_NATIVE_AUDIO_RETRY_BACKOFF: TimeDuration = TimeDuration::seconds(5);

/// Lance un worker dedie aux clips calendrier pour ne pas polluer le live news.
pub(crate) fn spawn_calendar_day_audio_worker(
    state: AppState,
    mut calendar_day_audio_rx: mpsc::Receiver<CalendarDayAudioJob>,
    mut shutdown_rx: watch::Receiver<bool>,
) -> Option<JoinHandle<()>> {
    if !state.config.tts.enabled {
        info!("calendar day audio worker disabled because tts is disabled");
        return None;
    }

    let mut provider = TtsProvider::from_config(&state.config.tts, state.tts_runtime.clone());

    Some(tokio::spawn(async move {
        loop {
            tokio::select! {
                changed = shutdown_rx.changed() => {
                    match changed {
                        Ok(()) if *shutdown_rx.borrow() => {
                            info!("calendar day audio worker received shutdown signal");
                            break;
                        }
                        Ok(()) => {}
                        Err(_) => {
                            info!("calendar day audio worker shutdown channel closed");
                            break;
                        }
                    }
                }
                received = calendar_day_audio_rx.recv() => {
                    let Some(job) = received else {
                        info!("calendar day audio queue closed");
                        break;
                    };
                    handle_calendar_day_audio_job(&state, &mut provider, job).await;
                }
            }
        }

        provider.shutdown().await;
    }))
}

async fn handle_calendar_day_audio_job(
    state: &AppState,
    provider: &mut TtsProvider,
    job: CalendarDayAudioJob,
) {
    state.calendar_logs.scope(serde_json::json!({"local_day":job.local_day,"work_id":job.native_work.lease().work_id(),"attempt":job.native_work.attempt_count(),"stream_id":job.reserved_stream_id,"workflow_kind":format!("audio_{:?}",job.kind)}), async {
crate::features::calendar_logs::observe("handle_calendar_day_audio_job", serde_json::json!({"job":format!("{:?}",job)}), async {

    // Un travail audio natif peut avoir ete repris pendant qu'il attendait
    // dans le canal en memoire. Dans ce cas, son ancien consumer ne doit ni
    // produire un second clip Piper/HLS, ni faire passer l'item en erreur.
    if !native_audio_lease_is_current(state, &job).await {
        return;
    }

    if !state.config.hls.enabled {
        record_calendar_audio_failure(
            state,
            &job,
            "calendar day audio requires NEWS_HLS_ENABLED=true",
        )
        .await;
        return;
    }

    let started_at = Instant::now();
    // Le stream est obligatoirement reserve avant l'entree dans ce worker.
    // Une reprise re-ecrit donc le meme artefact HLS au lieu de creer un clip
    // supplementaire.
    let stream_id = job.reserved_stream_id.clone();
    match restore_calendar_summary_manifests_if_possible(state, &job, &stream_id).await {
        Ok(Some(true)) => {
            info!(
                local_day = %job.local_day,
                stream_id = %stream_id,
                "calendar summary HLS manifests restored from the reserved clip"
            );
            return;
        }
        // Le fencing est aussi requis pour cette ecriture HLS de reparation.
        Ok(Some(false)) => return,
        Ok(None) => {}
        Err(error) => {
            warn!(%error, "failed to restore calendar summary HLS manifests");
            record_calendar_audio_failure(
                state,
                &job,
                "calendar_native_summary_hls_manifest_restore_failed",
            )
            .await;
            return;
        }
    }
    let intro_text = format!("{}.", job.title);
    let intro_audio = match synthesize_calendar_text(state, provider, &job, &intro_text).await {
        Ok(Some(audio)) => audio,
        Ok(None) => return,
        Err(error) => {
            record_calendar_audio_failure(
                state,
                &job,
                &format!("calendar day intro tts failed: {error}"),
            )
            .await;
            return;
        }
    };

    let start_payload = calendar_day_start_payload(&job, &stream_id, &intro_audio);
    if !native_audio_lease_is_current(state, &job).await {
        return;
    }
    let initial_publish = match state
        .hls_origin
        .publish_clip_intro(&start_payload, &intro_audio.bytes)
        .await
    {
        Ok(initial_publish) if initial_publish.hls_enabled => initial_publish,
        Ok(_) => {
            record_calendar_audio_failure(state, &job, "calendar_native_audio_hls_disabled").await;
            return;
        }
        Err(error) => {
            record_calendar_audio_failure(
                state,
                &job,
                &format!("calendar day intro hls failed: {error}"),
            )
            .await;
            return;
        }
    };

    let body_audio =
        match synthesize_calendar_text(state, provider, &job, &job.narration_text).await {
            Ok(Some(audio)) => audio,
            Ok(None) => return,
            Err(error) => {
                record_calendar_audio_failure(
                    state,
                    &job,
                    &format!("calendar day body tts failed: {error}"),
                )
                .await;
                return;
            }
        };

    if !native_audio_lease_is_current(state, &job).await {
        return;
    }
    let finalize_metrics = match state
        .hls_origin
        .append_clip_body_and_finalize(&stream_id, &body_audio.bytes)
        .await
    {
        Ok(metrics) => metrics,
        Err(error) => {
            record_calendar_audio_failure(
                state,
                &job,
                &format!("calendar day body hls failed: {error}"),
            )
            .await;
            return;
        }
    };
    crate::features::calendar_logs::event("audio.rendered", serde_json::json!({
        "kind": format!("{:?}", job.kind),
        "stream_id": stream_id,
        "segment_count": finalize_metrics.total_segment_count,
        "duration_ms": started_at.elapsed().as_millis(),
    }));

    let ready_result = match job.kind {
        CalendarDayAudioKind::Summary => {
            if !native_audio_lease_is_current(state, &job).await {
                return;
            }
            match state
                .hls_origin
                .persist_calendar_summary_clip(&job.local_day, &stream_id)
                .await
            {
                Ok(manifest) => record_calendar_summary_audio_ready(state, &job, manifest).await,
                Err(error) => Err(AppError::downstream(format!(
                    "calendar day summary hls persistence failed: {error}"
                ))),
            }
        }
        CalendarDayAudioKind::Item | CalendarDayAudioKind::Alert => {
            record_calendar_audio_ready(state, &job, &stream_id, &initial_publish.playlist_url)
                .await
        }
    };

    match ready_result {
        Ok(true) => {}
        // La verification de lease a echoue juste avant la persistence. Le
        // nouveau possesseur garde la reservation HLS et deciddera du statut.
        Ok(false) => return,
        Err(error) => {
            warn!(%error, "failed to persist calendar day audio ready");
            record_calendar_audio_failure(
                state,
                &job,
                "calendar_native_audio_ready_persistence_failed",
            )
            .await;
            return;
        }
    }

    info!(
        local_day = %job.local_day,
        stream_id = %stream_id,
        kind = ?job.kind,
        segment_count = finalize_metrics.total_segment_count,
        duration_ms = started_at.elapsed().as_millis() as u64,
        "calendar day audio ready"
    );

}).await
}).await
}

/// Tente d'abord la reparation legere d'un resume dont le clip finalise est
/// deja disponible. Cette branche est atteinte uniquement depuis un
/// `SummaryAudio` durable avec stream reserve: elle ne contourne donc ni la
/// lease SQLite ni le commit fence de l'etat `audio_ready`.
///
/// `None` signifie qu'aucun clip/manifeste exploitable n'existe et que le
/// worker doit produire l'audio normalement. `Some(false)` indique qu'un
/// nouveau possesseur a repris la lease avant le commit visible.
async fn restore_calendar_summary_manifests_if_possible(
    state: &AppState,
    job: &CalendarDayAudioJob,
    stream_id: &str,
) -> Result<Option<bool>, AppError> {
    crate::features::calendar_logs::operation(
        "restore_calendar_summary_manifests_if_possible",
        serde_json::json!({}),
        async {
            if !matches!(&job.kind, CalendarDayAudioKind::Summary) {
                return Ok(None);
            }
            if !native_audio_lease_is_current(state, job).await {
                return Ok(Some(false));
            }
            let Some(manifest) = state
                .hls_origin
                .restore_calendar_summary_manifests_from_existing_clip(&job.local_day, stream_id)
                .await?
            else {
                return Ok(None);
            };
            record_calendar_summary_audio_ready(state, job, manifest)
                .await
                .map(Some)
        },
    )
    .await
}

async fn synthesize_calendar_text(
    state: &AppState,
    provider: &mut TtsProvider,
    job: &CalendarDayAudioJob,
    text: &str,
) -> Result<Option<SynthesizedAudio>, AppError> {
    // Verifier avant tout appel Piper, puis une seconde fois apres les
    // lectures auxiliaires: celles-ci peuvent etre lentes, mais ne doivent
    // jamais laisser un ancien possesseur demarrer la synthese.
    if !native_audio_lease_is_current(state, job).await {
        return Ok(None);
    }

    let started_at = Instant::now();
    let glossary_entries = match state
        .news
        .list_glossary_entries(&GlossaryListParams::active_for_pipeline())
        .await
    {
        Ok(entries) => entries,
        Err(error) => {
            state
                .translation_runtime
                .record_glossary_error(error.to_string());
            Vec::new()
        }
    };
    crate::features::calendar_logs::payload("tts.input_text", text);
    crate::features::calendar_logs::payload(
        "tts.glossary_entries",
        &format!("{:?}", glossary_entries),
    );
    let (spoken_text, glossary_hits) = apply_spoken_glossary(text, &glossary_entries);
    crate::features::calendar_logs::event(
        "tts.glossary_applied",
        serde_json::json!({"text_bytes": spoken_text.len(), "hits":glossary_hits}),
    );
    state
        .translation_runtime
        .record_glossary_hits(glossary_hits);
    let normalized_text = normalize_narration_text_for_tts(&spoken_text);
    crate::features::calendar_logs::payload("tts.normalized_text", &normalized_text);
    if !native_audio_lease_is_current(state, job).await {
        return Ok(None);
    }
    let audio = synthesize_punctuation_segmented(provider, &normalized_text)
        .await
        .map(|segmented| segmented.audio);

    match &audio {
        Ok(audio) => state.tts_runtime.record_generated(
            &format!("calendar-day-{}", calendar_day_audio_identity(job)),
            started_at.elapsed().as_millis() as u64,
            audio.bytes.len() as u64,
            if audio.bytes.is_empty() { 0 } else { 1 },
        ),
        Err(error) => state.tts_runtime.record_failed(format!(
            "calendar day {} synthesis failed after {}ms: {error}",
            calendar_day_audio_identity(job),
            started_at.elapsed().as_millis() as u64
        )),
    }
    audio.map(Some)
}

async fn record_calendar_audio_ready(
    state: &AppState,
    job: &CalendarDayAudioJob,
    stream_id: &str,
    playlist_url: &str,
) -> Result<bool, AppError> {
    crate::features::calendar_logs::operation(
        "record_calendar_audio_ready",
        serde_json::json!({}),
        async {
            let target = match job.kind {
                CalendarDayAudioKind::Item => {
                    let item_id = job.item_id.ok_or_else(|| {
                        AppError::storage("calendar_native_item_audio_ready_target_invalid")
                    })?;
                    CalendarNativeAudioReadyTarget::Item {
                        item_id,
                        stream_id: stream_id.to_owned(),
                        playlist_url: playlist_url.to_owned(),
                    }
                }
                CalendarDayAudioKind::Alert => {
                    let alert_id = job.alert_id.ok_or_else(|| {
                        AppError::storage("calendar_native_alert_audio_ready_target_invalid")
                    })?;
                    CalendarNativeAudioReadyTarget::Alert {
                        alert_id,
                        stream_id: stream_id.to_owned(),
                        playlist_url: playlist_url.to_owned(),
                    }
                }
                CalendarDayAudioKind::Summary => {
                    return Err(AppError::storage(
                        "calendar_native_summary_audio_ready_requires_manifest",
                    ));
                }
            };
            commit_calendar_audio_ready(state, job, target).await
        },
    )
    .await
}

async fn record_calendar_summary_audio_ready(
    state: &AppState,
    job: &CalendarDayAudioJob,
    manifest: crate::features::audio::hls::origin::CalendarSummaryHlsManifest,
) -> Result<bool, AppError> {
    crate::features::calendar_logs::operation(
        "record_calendar_summary_audio_ready",
        serde_json::json!({}),
        async {
            let summary_id = job.summary_id.ok_or_else(|| {
                AppError::storage("calendar_native_summary_audio_ready_target_invalid")
            })?;
            commit_calendar_audio_ready(
                state,
                job,
                CalendarNativeAudioReadyTarget::Summary {
                    summary_id,
                    stream_id: manifest.stream_id,
                    playlist_url: manifest.current_playlist_url.clone(),
                    playlist_path: manifest.playlist_path,
                    immutable_playlist_url: manifest.immutable_playlist_url,
                    current_playlist_url: manifest.current_playlist_url,
                },
            )
            .await
        },
    )
    .await
}

/// Persiste l'etat visible et complete la lease dans une transaction unique.
/// Le worker peut perdre sa lease entre les effets HLS et ce commit, mais il
/// ne peut alors pas publier un statut `audio_ready` pour le compte d'un
/// nouveau possesseur.
async fn commit_calendar_audio_ready(
    state: &AppState,
    job: &CalendarDayAudioJob,
    target: CalendarNativeAudioReadyTarget,
) -> Result<bool, AppError> {
    crate::features::calendar_logs::operation(
        "commit_calendar_audio_ready",
        serde_json::json!({}),
        async {
            let store = CalendarWorkStore::new(state.news.clone_pool_for_feature_store());
            match state
                .calendar_day
                .commit_native_audio_ready(
                    &store,
                    job.native_work.lease(),
                    target,
                    OffsetDateTime::now_utc(),
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
                    if let Some(alert) = commit.alert {
                        calendar_day::emit_admin_event(
                            state,
                            CalendarDayAdminBroadcastEvent::AlertUpdated(alert),
                        );
                    }
                    Ok(true)
                }
                Err(AppError::Conflict(message))
                    if message == "calendar_native_work_fenced_out" =>
                {
                    Ok(false)
                }
                Err(error) => Err(error),
            }
        },
    )
    .await
}

async fn record_calendar_audio_failure(
    state: &AppState,
    job: &CalendarDayAudioJob,
    _message: &str,
) {
    crate::features::calendar_logs::observe(
        "record_calendar_audio_failure",
        serde_json::json!({}),
        async {
            // Ne pas degrader l'etat metier lorsqu'un autre worker a deja repris le
            // meme travail: le token fence l'ancien job avant toute ecriture.
            if !native_audio_lease_is_current(state, job).await {
                return;
            }

            // Les details du fournisseur ne sortent jamais dans SQLite ni dans les
            // logs de ce worker; l'etat durable ne contient qu'un code ferme.
            warn!(
                local_day = %job.local_day,
                kind = ?job.kind,
                "calendar native audio attempt failed"
            );
            crate::features::calendar_logs::event(
                "audio.failed",
                serde_json::json!({
                    "kind": format!("{:?}", job.kind),
                    "attempt": job.native_work.attempt_count(),
                }),
            );
            finish_native_audio_work_failure(state, job).await;
        },
    )
    .await
}

/// Libere la lease sans attendre le pipeline live. Les details du fournisseur
/// sont volontairement remplaces par un code ferme dans la file durable.
async fn finish_native_audio_work_failure(state: &AppState, job: &CalendarDayAudioJob) {
    crate::features::calendar_logs::observe("finish_native_audio_work_failure", serde_json::json!({}), async {

    if !native_audio_lease_is_current(state, job).await {
        return;
    }
    let store = CalendarWorkStore::new(state.news.clone_pool_for_feature_store());
    let now = OffsetDateTime::now_utc();
    let Ok(error_code) = CalendarWorkErrorCode::try_new("calendar_native_audio_failed") else {
        return;
    };
    if !native_audio_attempt_can_retry(job.native_work.attempt_count()) {
        let Some((kind, target_id)) = calendar_audio_work_target(job) else {
            match store.fail(job.native_work.lease(), &error_code, now).await {
                Ok(CalendarWorkLeaseMutation::Applied(_))
                | Ok(CalendarWorkLeaseMutation::FencedOut) => {}
                Err(error) => {
                    warn!(%error, "calendar native malformed audio work could not be dead-lettered")
                }
            }
            return;
        };
        match state
            .calendar_day
            .commit_native_audio_terminal_error(
                &store,
                job.native_work.lease(),
                kind,
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
            Err(error) => {
                warn!(%error, "calendar native audio terminal failure could not be committed");
            }
        }
        return;
    }

    match store
        .retry(
            job.native_work.lease(),
            &error_code,
            now + CALENDAR_NATIVE_AUDIO_RETRY_BACKOFF,
            now,
        )
        .await
    {
        Ok(CalendarWorkLeaseMutation::Applied(_)) | Ok(CalendarWorkLeaseMutation::FencedOut) => {}
        Err(_error) => warn!("calendar native audio work could not be re-planned"),
    }

}).await
}

/// Relie un job audio a la ligne SQLite que son dead-letter doit rendre
/// visible. Les constructeurs runtime fixent toujours l'identifiant adapte au
/// type; une incoherence legacy est isolee au niveau du travail durable.
fn calendar_audio_work_target(job: &CalendarDayAudioJob) -> Option<(CalendarWorkKind, i64)> {
    match job.kind {
        CalendarDayAudioKind::Item => job.item_id.map(|id| (CalendarWorkKind::ItemAudio, id)),
        CalendarDayAudioKind::Alert => job.alert_id.map(|id| (CalendarWorkKind::AlertAudio, id)),
        CalendarDayAudioKind::Summary => job
            .summary_id
            .map(|id| (CalendarWorkKind::SummaryAudio, id)),
    }
}

/// Un travail est reclame avant d'etre envoye au worker: l'essai `3` est donc
/// son dernier essai et doit etre termine au lieu d'etre replanifie.
fn native_audio_attempt_can_retry(attempt_count: u32) -> bool {
    attempt_count < CALENDAR_NATIVE_AUDIO_MAX_ATTEMPTS
}

/// Controle de fencing commun a tous les effets Piper, HLS et SQLite du
/// consumer audio. Une indisponibilite du store est traitee de la meme facon
/// qu'un token expiré: l'effet est abandonne, jamais transforme en erreur
/// metier dont un nouveau possesseur devrait heriter.
async fn native_audio_lease_is_current(state: &AppState, job: &CalendarDayAudioJob) -> bool {
    let store = CalendarWorkStore::new(state.news.clone_pool_for_feature_store());
    match store
        .lease_is_current(job.native_work.lease(), OffsetDateTime::now_utc())
        .await
    {
        Ok(true) => true,
        Ok(false) => {
            info!(
                local_day = %job.local_day,
                kind = ?job.kind,
                work_id = job.native_work.lease().work_id(),
                "calendar native audio job fenced out before side effect"
            );
            false
        }
        Err(error) => {
            warn!(
                %error,
                local_day = %job.local_day,
                kind = ?job.kind,
                work_id = job.native_work.lease().work_id(),
                "calendar native audio lease could not be verified; skipping side effect"
            );
            false
        }
    }
}

fn calendar_day_start_payload(
    job: &CalendarDayAudioJob,
    stream_id: &str,
    audio: &SynthesizedAudio,
) -> AudioStartPayload {
    let identity = calendar_day_audio_identity(job);
    AudioStartPayload {
        stream_id: stream_id.to_owned(),
        translation_id: -identity,
        news_event_id: job.item_id.unwrap_or_default(),
        language: "fr".to_owned(),
        source: "calendar_day".to_owned(),
        event_kind: match job.kind {
            CalendarDayAudioKind::Item => "calendar_day_item",
            CalendarDayAudioKind::Summary => "calendar_day_summary",
            CalendarDayAudioKind::Alert => "calendar_day_alert",
        }
        .to_owned(),
        source_title: job.title.clone(),
        translated_title: job.title.clone(),
        test_run_id: None,
        timestamp: job.timestamp.clone(),
        voice: audio.voice.clone(),
        audio_format: audio.audio_format.clone(),
        mime_type: audio.mime_type.clone(),
        total_bytes: audio.bytes.len(),
        chunk_count: if audio.bytes.is_empty() { 0 } else { 1 },
    }
}

fn calendar_day_audio_identity(job: &CalendarDayAudioJob) -> i64 {
    match job.kind {
        CalendarDayAudioKind::Item => job.item_id.unwrap_or_default(),
        CalendarDayAudioKind::Summary => job.summary_id.unwrap_or_default(),
        CalendarDayAudioKind::Alert => job.alert_id.unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use super::{CALENDAR_NATIVE_AUDIO_MAX_ATTEMPTS, native_audio_attempt_can_retry};

    #[test]
    fn native_audio_retries_are_bounded_to_three_claimed_attempts() {
        assert!(native_audio_attempt_can_retry(1));
        assert!(native_audio_attempt_can_retry(
            CALENDAR_NATIVE_AUDIO_MAX_ATTEMPTS - 1
        ));
        assert!(!native_audio_attempt_can_retry(
            CALENDAR_NATIVE_AUDIO_MAX_ATTEMPTS
        ));
        assert!(!native_audio_attempt_can_retry(u32::MAX));
    }
}
