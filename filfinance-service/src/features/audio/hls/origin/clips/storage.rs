//! Stockage disque des segments et de l'etat des clips HLS.

use std::{collections::HashSet, io::ErrorKind, path::PathBuf};

use serde::{Deserialize, Serialize};
// `fs` deplace les segments HLS deja rendus et conserve un petit etat de
// reprise a cote des artefacts immutables.
use tokio::fs;

use crate::errors::AppError;

use super::super::{
    HlsOriginState,
    helpers::sanitize_file_component,
    types::{HlsClip, HlsClipSegment, RenderedSegment},
};

const CLIP_STATE_VERSION: u8 = 1;
const CLIP_STATE_FILE: &str = "clip-state.json";
const CLIP_STATE_TMP_FILE: &str = ".clip-state.json.tmp";

/// Projection persistante minimale d'un clip. Les chemins ne sont pas
/// serialises: ils sont toujours reconstruits sous `NEWS_HLS_SEGMENTS_DIR`.
/// Cela permet a un travail calendrier avec `stream_id` reserve de reprendre
/// apres un redemarrage sans creer un second clip.
#[derive(Debug, Deserialize, Serialize)]
struct PersistedHlsClip {
    version: u8,
    stream_id: String,
    target_duration_secs: u32,
    segments: Vec<PersistedHlsClipSegment>,
    finalized: bool,
    updated_at_ms: i64,
}

#[derive(Debug, Deserialize, Serialize)]
struct PersistedHlsClipSegment {
    name: String,
    duration_secs: f64,
    discontinuity_before: bool,
}

impl HlsOriginState {
    /// Recharge l'etat d'un clip deja durable, si present.
    ///
    /// Un clip calendrier peut etre arrete apres l'ecriture HLS mais avant
    /// le commit SQLite fence. La reprise du meme travail possede le meme
    /// `stream_id`; recharger cet etat evite de reencoder ou de publier un
    /// second clip. Un etat en memoire gagne afin de ne jamais faire de lecture
    /// disque sur le hot path ordinaire.
    pub(in crate::features::audio::hls::origin) async fn restore_clip_if_persisted(
        &self,
        stream_id: &str,
    ) -> Result<Option<HlsClip>, AppError> {
        if let Some(clip) = self
            .inner
            .lock()
            .expect("hls origin mutex poisoned")
            .clips
            .get(stream_id)
            .cloned()
        {
            return Ok(Some(clip));
        }

        let clip_dir = self.clip_directory(stream_id);
        let state_path = clip_dir.join(CLIP_STATE_FILE);
        let bytes = match fs::read(&state_path).await {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        let persisted = serde_json::from_slice::<PersistedHlsClip>(&bytes)?;
        if persisted.version != CLIP_STATE_VERSION || persisted.stream_id != stream_id {
            return Err(AppError::downstream(
                "hls persisted clip state does not match the reserved stream",
            ));
        }
        if persisted.segments.is_empty() {
            return Err(AppError::downstream(
                "hls persisted clip state has no segments",
            ));
        }

        let mut names = HashSet::with_capacity(persisted.segments.len());
        let mut segments = Vec::with_capacity(persisted.segments.len());
        for (index, segment) in persisted.segments.into_iter().enumerate() {
            let expected_name = format!("segment-{index:03}.ts");
            if segment.name != expected_name || !names.insert(segment.name.clone()) {
                return Err(AppError::downstream(
                    "hls persisted clip state has invalid segment ordering",
                ));
            }
            let file_path = clip_dir.join(&segment.name);
            match fs::metadata(&file_path).await {
                Ok(metadata) if metadata.is_file() => {}
                Ok(_) => {
                    return Err(AppError::downstream(
                        "hls persisted clip segment path is not a file",
                    ));
                }
                Err(error) if error.kind() == ErrorKind::NotFound => {
                    return Err(AppError::downstream(
                        "hls persisted clip state references a missing segment",
                    ));
                }
                Err(error) => return Err(error.into()),
            }
            segments.push(HlsClipSegment {
                name: segment.name,
                duration_secs: segment.duration_secs,
                file_path,
                discontinuity_before: segment.discontinuity_before,
            });
        }

        let restored = HlsClip {
            stream_id: stream_id.to_owned(),
            target_duration_secs: persisted.target_duration_secs.max(1),
            segments,
            finalized: persisted.finalized,
            durable_recovery: true,
            updated_at_ms: persisted.updated_at_ms,
        };
        let mut inner = self.inner.lock().expect("hls origin mutex poisoned");
        let clip = inner
            .clips
            .entry(stream_id.to_owned())
            .or_insert_with(|| restored.clone())
            .clone();
        Ok(Some(clip))
    }

    /// Ecrit atomiquement l'etat necessaire a la reprise d'un clip reserve.
    /// L'etat est pose apres les segments et avant le commit memoire: un crash
    /// entre ces deux etapes reste donc recuperable au prochain claim SQLite.
    pub(in crate::features::audio::hls::origin) async fn persist_clip_state(
        &self,
        clip: &HlsClip,
    ) -> Result<(), AppError> {
        if clip.segments.is_empty() {
            return Err(AppError::downstream(
                "cannot persist hls clip state without segments",
            ));
        }
        let clip_dir = self.clip_directory(&clip.stream_id);
        fs::create_dir_all(&clip_dir).await?;

        let mut names = HashSet::with_capacity(clip.segments.len());
        let mut segments = Vec::with_capacity(clip.segments.len());
        for (index, segment) in clip.segments.iter().enumerate() {
            let expected_name = format!("segment-{index:03}.ts");
            if segment.name != expected_name || !names.insert(segment.name.clone()) {
                return Err(AppError::downstream(
                    "cannot persist hls clip state with invalid segment ordering",
                ));
            }
            match fs::metadata(&segment.file_path).await {
                Ok(metadata) if metadata.is_file() => {}
                Ok(_) => {
                    return Err(AppError::downstream(
                        "cannot persist hls clip state for a non-file segment",
                    ));
                }
                Err(error) => return Err(error.into()),
            }
            segments.push(PersistedHlsClipSegment {
                name: segment.name.clone(),
                duration_secs: segment.duration_secs,
                discontinuity_before: segment.discontinuity_before,
            });
        }

        let bytes = serde_json::to_vec(&PersistedHlsClip {
            version: CLIP_STATE_VERSION,
            stream_id: clip.stream_id.clone(),
            target_duration_secs: clip.target_duration_secs.max(1),
            segments,
            finalized: clip.finalized,
            updated_at_ms: clip.updated_at_ms,
        })?;
        let temporary_path = clip_dir.join(CLIP_STATE_TMP_FILE);
        let state_path = clip_dir.join(CLIP_STATE_FILE);
        fs::write(&temporary_path, bytes).await?;
        fs::rename(&temporary_path, &state_path).await?;
        Ok(())
    }

    /// Deplace les segments rendus vers le dossier immutable d'un clip.
    ///
    /// Quand un processus calendrier s'interrompt pendant le deplacement, son
    /// consumer durable rejoue exactement le meme stream reserve. Seul ce cas
    /// peut reutiliser un fichier deja la sans ecrasement ; le live conserve
    /// son erreur de collision historique et ne peut donc jamais servir un
    /// segment precedent par erreur.
    pub(in crate::features::audio::hls::origin) async fn move_rendered_clip_segments(
        &self,
        stream_id: &str,
        rendered_segments: Vec<RenderedSegment>,
        start_index: usize,
        discontinuity_before_first: bool,
        reuse_reserved_segments: bool,
    ) -> Result<Vec<HlsClipSegment>, AppError> {
        let clip_dir = self.clip_directory(stream_id);
        fs::create_dir_all(&clip_dir).await?;

        let mut clip_segments = Vec::with_capacity(rendered_segments.len());
        for (offset, rendered_segment) in rendered_segments.into_iter().enumerate() {
            let segment_index = start_index + offset;
            let name = format!("segment-{segment_index:03}.ts");
            let file_path = clip_dir.join(&name);
            match fs::metadata(&file_path).await {
                Ok(metadata) if metadata.is_file() => {
                    if !reuse_reserved_segments {
                        return Err(AppError::downstream(format!(
                            "hls clip segment already exists: {name}"
                        )));
                    }
                    // Ne pas remplacer l'artefact immutable d'une tentative
                    // precedente. Le stream est reserve durablement, donc ce
                    // segment est precisement celui que la reprise doit servir.
                    if let Err(error) = fs::remove_file(&rendered_segment.temp_path).await
                        && error.kind() != ErrorKind::NotFound
                    {
                        return Err(error.into());
                    }
                }
                Ok(_) => {
                    return Err(AppError::downstream(format!(
                        "hls clip segment path is not a file: {}",
                        file_path.display()
                    )));
                }
                Err(error) if error.kind() == ErrorKind::NotFound => {
                    fs::rename(&rendered_segment.temp_path, &file_path).await?;
                }
                Err(error) => return Err(error.into()),
            }
            clip_segments.push(HlsClipSegment {
                name,
                duration_secs: rendered_segment.duration_secs,
                file_path,
                discontinuity_before: discontinuity_before_first && offset == 0,
            });
        }

        Ok(clip_segments)
    }

    fn clip_directory(&self, stream_id: &str) -> PathBuf {
        self.segments_dir
            .join("clips")
            .join(sanitize_file_component(stream_id))
    }
}

#[cfg(test)]
mod tests {
    use std::{
        path::Path,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;
    use crate::config::HlsConfig;

    fn test_hls_config(root: &Path) -> HlsConfig {
        HlsConfig {
            enabled: true,
            access_control_enabled: false,
            access_ticket_secret: None,
            access_ticket_ttl_secs: 3600,
            segments_dir: root.to_string_lossy().to_string(),
            ffmpeg_command: "ffmpeg".to_owned(),
            replay_segment_target_duration_secs: 4,
            clip_segment_target_duration_secs: 2,
        }
    }

    async fn rendered_segments(
        root: &Path,
        attempt: &str,
        contents: &[&[u8]],
    ) -> Vec<RenderedSegment> {
        let attempt_dir = root.join("rendered").join(attempt);
        fs::create_dir_all(&attempt_dir)
            .await
            .expect("rendered test directory should be created");
        let mut segments = Vec::with_capacity(contents.len());
        for (index, bytes) in contents.iter().enumerate() {
            let temp_path = attempt_dir.join(format!("segment_{index:03}.ts"));
            fs::write(&temp_path, bytes)
                .await
                .expect("rendered test segment should be written");
            segments.push(RenderedSegment {
                temp_path,
                duration_secs: 1.0 + index as f64,
            });
        }
        segments
    }

    #[tokio::test]
    async fn reserved_calendar_streams_recover_partial_segments_after_restart_without_duplicates() {
        // Les trois sortes de clips calendrier utilisent le meme pipeline HLS.
        // Cette fixture simule deux coupures: d'abord juste apres les segments
        // d'intro, puis juste apres ceux du corps. Les reprises conservent le
        // stream reserve et les octets deja produits, sans erreur "already exists".
        let root = std::env::temp_dir().join(format!(
            "filfinance-calendar-hls-crash-recovery-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock should be after epoch")
                .as_nanos()
        ));
        let config = test_hls_config(&root);

        for stream_id in [
            "calendar-item-reserved-stream",
            "calendar-alert-reserved-stream",
            "calendar-summary-reserved-stream",
        ] {
            // Crash 1: l'intro est deplacee, mais aucun etat ni commit memoire
            // ne sont encore poses.
            let first_process = HlsOriginState::from_config(&config);
            let initial_intro = first_process
                .move_rendered_clip_segments(
                    stream_id,
                    rendered_segments(&root, &format!("{stream_id}-intro-first"), &[b"intro-a"])
                        .await,
                    0,
                    false,
                    false,
                )
                .await
                .expect("initial intro segments should move");
            drop(first_process);

            // Le nouveau processus rerend l'intro mais reutilise le segment
            // immutable initial au lieu de l'ecraser.
            let second_process = HlsOriginState::from_config(&config);
            let retried_intro_temp = root
                .join("rendered")
                .join(format!("{stream_id}-intro-retry"))
                .join("segment_000.ts");
            let retried_intro = second_process
                .move_rendered_clip_segments(
                    stream_id,
                    rendered_segments(&root, &format!("{stream_id}-intro-retry"), &[b"intro-b"])
                        .await,
                    0,
                    false,
                    true,
                )
                .await
                .expect("retry must reuse the existing intro segment");
            assert_eq!(
                fs::read(&retried_intro[0].file_path)
                    .await
                    .expect("reused intro should be readable"),
                b"intro-a"
            );
            assert!(
                !retried_intro_temp.exists(),
                "the discarded retry render must not remain on disk"
            );

            let intro_clip = HlsClip {
                stream_id: stream_id.to_owned(),
                target_duration_secs: 2,
                segments: retried_intro,
                finalized: false,
                durable_recovery: true,
                updated_at_ms: 1,
            };
            second_process
                .persist_clip_state(&intro_clip)
                .await
                .expect("intro state should be durable before memory commit");
            drop(second_process);

            // Le redemarrage rehydrate l'intro durable, puis subit une seconde
            // coupure apres l'ecriture du corps et avant son etat final.
            let third_process = HlsOriginState::from_config(&config);
            let restored_intro = third_process
                .restore_clip_if_persisted(stream_id)
                .await
                .expect("intro state should be readable")
                .expect("intro state should exist");
            assert!(!restored_intro.finalized);
            let partial_body = third_process
                .move_rendered_clip_segments(
                    stream_id,
                    rendered_segments(&root, &format!("{stream_id}-body-first"), &[b"body-a"])
                        .await,
                    restored_intro.segments.len(),
                    true,
                    true,
                )
                .await
                .expect("body segment should move before simulated crash");
            drop(third_process);

            // Le dernier processus repart de l'etat d'intro, rencontre le
            // segment du corps deja present et le reutilise egalement.
            let fourth_process = HlsOriginState::from_config(&config);
            let restored_intro = fourth_process
                .restore_clip_if_persisted(stream_id)
                .await
                .expect("intro state should survive the body crash")
                .expect("intro state should exist after the body crash");
            let retried_body_temp = root
                .join("rendered")
                .join(format!("{stream_id}-body-retry"))
                .join("segment_000.ts");
            let retried_body = fourth_process
                .move_rendered_clip_segments(
                    stream_id,
                    rendered_segments(&root, &format!("{stream_id}-body-retry"), &[b"body-b"])
                        .await,
                    restored_intro.segments.len(),
                    true,
                    true,
                )
                .await
                .expect("retry must reuse the existing body segment");
            assert_eq!(
                fs::read(&retried_body[0].file_path)
                    .await
                    .expect("reused body should be readable"),
                b"body-a"
            );
            assert!(
                !retried_body_temp.exists(),
                "the discarded body retry render must not remain on disk"
            );

            let mut finalized_clip = restored_intro;
            finalized_clip.segments.extend(retried_body);
            finalized_clip.finalized = true;
            finalized_clip.updated_at_ms = 2;
            fourth_process
                .persist_clip_state(&finalized_clip)
                .await
                .expect("final clip state should be durable");
            drop(fourth_process);

            let restarted_final = HlsOriginState::from_config(&config)
                .restore_clip_if_persisted(stream_id)
                .await
                .expect("final state should be readable after restart")
                .expect("final state should exist after restart");
            assert!(restarted_final.finalized);
            assert_eq!(restarted_final.segments.len(), 2);
            assert_eq!(initial_intro[0].name, "segment-000.ts");
            assert_eq!(partial_body[0].name, "segment-001.ts");
        }

        let _ = fs::remove_dir_all(&root).await;
    }

    #[tokio::test]
    async fn live_streams_reject_preexisting_segments() {
        let root = std::env::temp_dir().join(format!(
            "filfinance-live-hls-existing-segment-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock should be after epoch")
                .as_nanos()
        ));
        let config = test_hls_config(&root);
        let origin = HlsOriginState::from_config(&config);
        let stream_id = "live-stream-must-not-reuse";

        origin
            .move_rendered_clip_segments(
                stream_id,
                rendered_segments(&root, "live-first", &[b"first"]).await,
                0,
                false,
                false,
            )
            .await
            .expect("initial live segment should move");

        let error = origin
            .move_rendered_clip_segments(
                stream_id,
                rendered_segments(&root, "live-second", &[b"second"]).await,
                0,
                false,
                false,
            )
            .await
            .expect_err("a live stream must not reuse a previous segment");
        assert!(matches!(
            error,
            AppError::Downstream(message) if message.contains("already exists")
        ));

        let _ = fs::remove_dir_all(&root).await;
    }
}
