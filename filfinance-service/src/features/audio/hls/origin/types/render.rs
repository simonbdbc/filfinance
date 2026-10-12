//! Segments temporaires produits par l'encodeur avant publication.

// `PathBuf` garde les chemins des segments HLS immutable sur disque.
use std::path::PathBuf;

use super::replay::HlsSegment;

/// Segment temporaire produit par `ffmpeg` avant son nom canonique public.
#[derive(Debug)]
pub(in crate::features::audio::hls::origin) struct RenderedSegment {
    /// Chemin temporaire produit dans le dossier de travail du stream.
    pub(in crate::features::audio::hls::origin) temp_path: PathBuf,
    /// Duree estimee pour le futur `#EXTINF`.
    pub(in crate::features::audio::hls::origin) duration_secs: f64,
}

/// Segment planifie avec sa sequence definitive, avant publication dans l'index replay.
#[derive(Debug, Clone)]
pub(in crate::features::audio::hls::origin) struct PlannedSegment {
    /// Metadata HLS qui sera ajoutee a la playlist apres deplacement du fichier.
    pub(in crate::features::audio::hls::origin) segment: HlsSegment,
    /// Chemin temporaire que l'on doit deplacer vers `segment.file_path`.
    pub(in crate::features::audio::hls::origin) temp_path: PathBuf,
}
