//! Helpers de reconstruction et lecture de l'index replay HLS.

// `fs` standard sert seulement au bootstrap synchrone de l'index replay au demarrage.
use std::fs as std_fs;
// `VecDeque` garde les segments replay dans l'ordre d'emission.
use std::collections::VecDeque;
// `Path` represente le dossier HLS local scanne au demarrage.
use std::path::Path;

use super::{files::parse_segment_sequence, sidecar::load_segment_sidecar};
use crate::features::audio::hls::origin::types::HlsSegment;

/// Retourne tous les segments du jour courant, dans l'ordre de lecture replay.
pub(in crate::features::audio::hls::origin) fn replay_segments_for_day(
    segments: &VecDeque<HlsSegment>,
    local_day: &str,
) -> Vec<HlsSegment> {
    // Le replay est un parcours complet du jour, pas une fenetre glissante.
    segments
        .iter()
        .filter(|segment| segment.local_day == local_day)
        .cloned()
        .collect()
}

/// Recharge les segments deja presents sur disque pour reconstruire le replay apres redemarrage.
pub(in crate::features::audio::hls::origin) fn load_existing_segments_for_day(
    segments_dir: &Path,
    local_day: &str,
    default_duration_secs: u32,
) -> VecDeque<HlsSegment> {
    // Le dossier jour est la seule zone scannee: pas d'historique multi-jours.
    let day_dir = segments_dir.join(local_day);
    // Si le dossier n'existe pas encore, le replay demarre simplement vide.
    let Ok(entries) = std_fs::read_dir(&day_dir) else {
        return VecDeque::new();
    };
    // On stocke d'abord en `Vec` pour trier par sequence avant de construire le `VecDeque`.
    let mut segments = Vec::new();

    for entry in entries.flatten() {
        // Chaque entree est validee par extension et nom numerique avant d'entrer dans l'index.
        let path = entry.path();
        // Seuls les fichiers `.ts` appartiennent au contrat public HLS.
        if path.extension().and_then(|value| value.to_str()) != Some("ts") {
            continue;
        }

        // Le nom de fichier doit rester lisible comme `{sequence}.ts`.
        let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        // Un fichier mal nomme est ignore plutot que de bloquer le demarrage du service.
        let Ok(sequence) = parse_segment_sequence(file_name) else {
            continue;
        };

        let sidecar = load_segment_sidecar(&path);

        segments.push(HlsSegment {
            sequence,
            duration_secs: sidecar
                .as_ref()
                .map(|metadata| metadata.duration_secs)
                .unwrap_or(default_duration_secs.max(1) as f64),
            file_path: path,
            local_day: sidecar
                .as_ref()
                .map(|metadata| metadata.local_day.clone())
                .unwrap_or_else(|| local_day.to_owned()),
            translation_id: sidecar
                .as_ref()
                .and_then(|metadata| metadata.translation_id),
            news_event_id: sidecar.as_ref().and_then(|metadata| metadata.news_event_id),
            stream_id: sidecar
                .as_ref()
                .and_then(|metadata| metadata.stream_id.clone()),
            timestamp: sidecar
                .as_ref()
                .and_then(|metadata| metadata.timestamp.clone()),
        });
    }

    // Le replay doit rester ancien -> recent, meme apres reconstruction depuis le filesystem.
    segments.sort_by_key(|segment| segment.sequence);
    // `VecDeque::from` donne ensuite la meme structure que les segments ajoutes au replay.
    VecDeque::from(segments)
}
