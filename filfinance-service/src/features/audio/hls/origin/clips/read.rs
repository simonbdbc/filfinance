//! Lecture HTTP des clips HLS progressifs.

use std::{collections::HashSet, io::ErrorKind};

// `fs` lit les segments HLS déjà rendus.
use tokio::fs;

use crate::{
    errors::AppError,
    features::audio::hls::playlist::{
        calendar_summary_current_playlist_url, calendar_summary_immutable_playlist_url,
        render_calendar_summary_playlist, render_clip_playlist,
    },
};

use super::super::{
    HlsOriginState,
    helpers::{looks_like_local_day, parse_clip_segment_name, sanitize_file_component},
    types::{CalendarSummaryHlsManifest, HlsClipPlaylist},
};

impl HlsOriginState {
    /// Indique si une playlist de clip est actuellement servable par l'origin en mémoire.
    #[allow(dead_code)]
    pub(crate) fn has_clip_playlist(&self, stream_id: &str) -> bool {
        if !self.enabled {
            return false;
        }

        let inner = self.inner.lock().expect("hls origin mutex poisoned");
        match inner.clips.get(stream_id) {
            Some(clip) => !clip.segments.is_empty(),
            None => false,
        }
    }

    /// Génère la playlist d'un clip HLS progressif.
    pub(in crate::features::audio::hls) fn clip_playlist(
        &self,
        stream_id: &str,
    ) -> Result<HlsClipPlaylist, AppError> {
        if !self.enabled {
            return Err(AppError::not_found("hls clip origin is disabled"));
        }

        let clip =
            {
                let inner = self.inner.lock().expect("hls origin mutex poisoned");
                inner.clips.get(stream_id).cloned().ok_or_else(|| {
                    AppError::not_found(format!("hls clip not found: {stream_id}"))
                })?
            };

        if clip.segments.is_empty() {
            return Err(AppError::not_found(format!(
                "hls clip has no segment yet: {stream_id}"
            )));
        }

        Ok(render_clip_playlist(&clip))
    }

    /// Lit un segment de clip par son `stream_id` et son nom public `segment-NNN.ts`.
    pub(in crate::features::audio::hls) async fn clip_segment_bytes(
        &self,
        stream_id: &str,
        segment_name: &str,
    ) -> Result<Vec<u8>, AppError> {
        if !self.enabled {
            return Err(AppError::not_found("hls clip origin is disabled"));
        }

        let segment_name = parse_clip_segment_name(segment_name)?;
        let file_path = {
            let inner = self.inner.lock().expect("hls origin mutex poisoned");
            let clip = inner
                .clips
                .get(stream_id)
                .ok_or_else(|| AppError::not_found(format!("hls clip not found: {stream_id}")))?;
            clip.segments
                .iter()
                .find(|segment| segment.name == segment_name)
                .map(|segment| segment.file_path.clone())
                .ok_or_else(|| {
                    AppError::not_found(format!("hls clip segment not found: {segment_name}"))
                })?
        };

        let bytes = fs::read(file_path).await?;
        Ok(bytes)
    }

    /// Reconstitue les manifests calendrier d'un stream reserve sans reencoder
    /// le clip deja finalise.
    ///
    /// Le premier cas couvre une reparation dans le meme processus: le clip
    /// finalise est encore indexe en memoire et `persist_calendar_summary_clip`
    /// recree a l'identique l'immuable et `current.m3u8`. Apres un redemarrage,
    /// l'index des clips est vide mais l'immuable peut deja etre sur disque;
    /// dans ce cas, le pointeur courant est restaure depuis ce manifest.
    pub(crate) async fn restore_calendar_summary_manifests_from_existing_clip(
        &self,
        local_day: &str,
        stream_id: &str,
    ) -> Result<Option<CalendarSummaryHlsManifest>, AppError> {
        crate::features::calendar_logs::operation(
            "restore_calendar_summary_manifests_from_existing_clip",
            serde_json::json!({}),
            async {
                if !self.enabled {
                    return Err(AppError::not_found(
                        "hls calendar summary origin is disabled",
                    ));
                }
                if !looks_like_local_day(local_day) {
                    return Err(AppError::not_found("calendar summary day is invalid"));
                }

                let has_finalized_clip = {
                    let inner = self.inner.lock().expect("hls origin mutex poisoned");
                    inner
                        .clips
                        .get(stream_id)
                        .is_some_and(|clip| clip.finalized && !clip.segments.is_empty())
                };
                if has_finalized_clip {
                    return self
                        .persist_calendar_summary_clip(local_day, stream_id)
                        .await
                        .map(Some);
                }

                let safe_stream_id = sanitize_file_component(stream_id);
                let day_dir = self.segments_dir.join("calendar-summary").join(local_day);
                let summary_dir = day_dir.join(&safe_stream_id);
                let immutable_path = summary_dir.join("playlist.m3u8");
                let playlist = match fs::read_to_string(&immutable_path).await {
                    Ok(playlist) => playlist,
                    Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
                    Err(error) => return Err(error.into()),
                };
                if !playlist.starts_with("#EXTM3U") {
                    return Err(AppError::downstream(
                        "calendar summary immutable playlist is invalid",
                    ));
                }

                fs::create_dir_all(&day_dir).await?;
                let current_path = day_dir.join("current.m3u8");
                let current_tmp_path = day_dir.join("current.m3u8.tmp");
                fs::write(&current_tmp_path, &playlist).await?;
                fs::rename(&current_tmp_path, &current_path).await?;

                Ok(Some(CalendarSummaryHlsManifest {
                    stream_id: safe_stream_id.clone(),
                    playlist_path: format!(
                        "calendar-summary/{local_day}/{safe_stream_id}/playlist.m3u8"
                    ),
                    immutable_playlist_url: calendar_summary_immutable_playlist_url(
                        local_day,
                        &safe_stream_id,
                    ),
                    current_playlist_url: calendar_summary_current_playlist_url(local_day),
                }))
            },
        )
        .await
    }

    /// Persiste un clip finalise comme resume calendrier immutable et met a jour le pointeur courant.
    pub(crate) async fn persist_calendar_summary_clip(
        &self,
        local_day: &str,
        stream_id: &str,
    ) -> Result<CalendarSummaryHlsManifest, AppError> {
        crate::features::calendar_logs::operation(
            "persist_calendar_summary_clip",
            serde_json::json!({}),
            async {
                if !self.enabled {
                    return Err(AppError::not_found(
                        "hls calendar summary origin is disabled",
                    ));
                }
                if !looks_like_local_day(local_day) {
                    return Err(AppError::not_found("calendar summary day is invalid"));
                }

                let clip = {
                    let inner = self.inner.lock().expect("hls origin mutex poisoned");
                    inner.clips.get(stream_id).cloned().ok_or_else(|| {
                        AppError::not_found(format!("hls clip not found: {stream_id}"))
                    })?
                };
                if !clip.finalized || clip.segments.is_empty() {
                    return Err(AppError::downstream(format!(
                        "hls clip is not finalized for calendar summary: {stream_id}"
                    )));
                }

                let safe_stream_id = sanitize_file_component(stream_id);
                let relative_dir =
                    ["calendar-summary", local_day, safe_stream_id.as_str()].join("/");
                let summary_dir = self
                    .segments_dir
                    .join("calendar-summary")
                    .join(local_day)
                    .join(&safe_stream_id);
                fs::create_dir_all(&summary_dir).await?;

                for segment in &clip.segments {
                    let target_path = summary_dir.join(&segment.name);
                    match fs::metadata(&target_path).await {
                        // Le meme stream est reserve par la file durable: un segment
                        // deja present est donc l'artefact immutable d'une tentative
                        // precedente, a reutiliser pour rebatir les manifests.
                        Ok(metadata) if metadata.is_file() => continue,
                        Ok(_) => {
                            return Err(AppError::downstream(format!(
                                "calendar summary segment path is not a file: {}",
                                target_path.display()
                            )));
                        }
                        Err(error) if error.kind() == ErrorKind::NotFound => {}
                        Err(error) => return Err(error.into()),
                    }
                    fs::copy(&segment.file_path, &target_path).await?;
                }

                let playlist = render_calendar_summary_playlist(&clip, local_day, &safe_stream_id);
                let playlist_path = summary_dir.join("playlist.m3u8");
                let playlist_tmp_path = summary_dir.join("playlist.m3u8.tmp");
                fs::write(&playlist_tmp_path, &playlist.body).await?;
                fs::rename(&playlist_tmp_path, &playlist_path).await?;

                let current_path = self
                    .segments_dir
                    .join("calendar-summary")
                    .join(local_day)
                    .join("current.m3u8");
                let current_tmp_path = self
                    .segments_dir
                    .join("calendar-summary")
                    .join(local_day)
                    .join("current.m3u8.tmp");
                fs::write(&current_tmp_path, &playlist.body).await?;
                fs::rename(&current_tmp_path, &current_path).await?;

                Ok(CalendarSummaryHlsManifest {
                    stream_id: safe_stream_id.clone(),
                    playlist_path: format!("{relative_dir}/playlist.m3u8"),
                    immutable_playlist_url: calendar_summary_immutable_playlist_url(
                        local_day,
                        &safe_stream_id,
                    ),
                    current_playlist_url: calendar_summary_current_playlist_url(local_day),
                })
            },
        )
        .await
    }

    /// Verifie si le manifest persistant courant existe sur disque.
    pub(crate) fn has_calendar_summary_current_manifest(&self, local_day: &str) -> bool {
        if !self.enabled || !looks_like_local_day(local_day) {
            return false;
        }
        self.segments_dir
            .join("calendar-summary")
            .join(local_day)
            .join("current.m3u8")
            .is_file()
    }

    /// Verifie si le manifest immutable existe sur disque.
    pub(crate) fn has_calendar_summary_manifest(&self, local_day: &str, stream_id: &str) -> bool {
        if !self.enabled || !looks_like_local_day(local_day) {
            return false;
        }
        self.segments_dir
            .join("calendar-summary")
            .join(local_day)
            .join(sanitize_file_component(stream_id))
            .join("playlist.m3u8")
            .is_file()
    }

    /// Lit un manifest persistant de resume calendrier.
    pub(in crate::features::audio) async fn calendar_summary_playlist(
        &self,
        local_day: &str,
        stream_id: Option<&str>,
    ) -> Result<String, AppError> {
        if !self.enabled {
            return Err(AppError::not_found(
                "hls calendar summary origin is disabled",
            ));
        }
        if !looks_like_local_day(local_day) {
            return Err(AppError::not_found("calendar summary day is invalid"));
        }
        let path = match stream_id {
            Some(stream_id) => self
                .segments_dir
                .join("calendar-summary")
                .join(local_day)
                .join(sanitize_file_component(stream_id))
                .join("playlist.m3u8"),
            None => self
                .segments_dir
                .join("calendar-summary")
                .join(local_day)
                .join("current.m3u8"),
        };
        fs::read_to_string(path).await.map_err(AppError::from)
    }

    /// Lit un segment persistant de resume calendrier.
    pub(in crate::features::audio) async fn calendar_summary_segment_bytes(
        &self,
        local_day: &str,
        stream_id: &str,
        segment_name: &str,
    ) -> Result<Vec<u8>, AppError> {
        if !self.enabled {
            return Err(AppError::not_found(
                "hls calendar summary origin is disabled",
            ));
        }
        if !looks_like_local_day(local_day) {
            return Err(AppError::not_found("calendar summary day is invalid"));
        }
        let segment_name = parse_clip_segment_name(segment_name)?;
        let path = self
            .segments_dir
            .join("calendar-summary")
            .join(local_day)
            .join(sanitize_file_component(stream_id))
            .join(segment_name);
        fs::read(path).await.map_err(AppError::from)
    }

    /// Supprime les dossiers HLS calendrier qui ne font plus partie de la retention ouvrée.
    pub(crate) async fn prune_calendar_summary_manifests(
        &self,
        retained_days: &[String],
    ) -> Result<u64, AppError> {
        if !self.enabled {
            return Ok(0);
        }

        let retained_days = retained_days
            .iter()
            .map(String::as_str)
            .collect::<HashSet<_>>();
        let root = self.segments_dir.join("calendar-summary");
        let mut entries = match fs::read_dir(&root).await {
            Ok(entries) => entries,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(0),
            Err(error) => return Err(AppError::from(error)),
        };
        let mut deleted = 0_u64;

        while let Some(entry) = entries.next_entry().await? {
            let file_type = entry.file_type().await?;
            if !file_type.is_dir() {
                continue;
            }
            let day = entry.file_name().to_string_lossy().to_string();
            if retained_days.contains(day.as_str()) && looks_like_local_day(&day) {
                continue;
            }
            fs::remove_dir_all(entry.path()).await?;
            deleted += 1;
        }

        Ok(deleted)
    }
}
