//! Types des clips HLS progressifs par news.

// `PathBuf` garde les chemins des segments HLS immutable sur disque.
use std::path::PathBuf;

/// Segment appartenant a un clip HLS fini par news.
#[derive(Debug, Clone)]
pub(in crate::features::audio) struct HlsClipSegment {
    /// Nom public stable, par exemple `segment-000.ts`.
    pub(in crate::features::audio) name: String,
    /// Duree annoncee dans `#EXTINF`.
    pub(in crate::features::audio) duration_secs: f64,
    /// Chemin local du segment deja ecrit avant insertion playlist.
    pub(in crate::features::audio) file_path: PathBuf,
    /// Marque la transition intro -> corps quand les encodages ont ete produits separement.
    pub(in crate::features::audio) discontinuity_before: bool,
}

/// Clip HLS progressif mutable jusqu'a l'ajout de `#EXT-X-ENDLIST`.
#[derive(Debug, Clone)]
pub(in crate::features::audio) struct HlsClip {
    /// Identifiant stable recu par le client dans `audio_ready`.
    pub(in crate::features::audio) stream_id: String,
    /// Duree cible propre aux clips, generalement plus courte que le replay.
    pub(in crate::features::audio) target_duration_secs: u32,
    /// Segments du clip, dans l'ordre exact de lecture.
    pub(in crate::features::audio) segments: Vec<HlsClipSegment>,
    /// Indique si le corps a ete ajoute et si la playlist est definitive.
    pub(in crate::features::audio) finalized: bool,
    /// Active la reprise disque uniquement pour les clips calendrier dont le
    /// `stream_id` est reserve dans la file SQLite. Le live news reste sur son
    /// hot path memoire et ne lit jamais cet etat additionnel.
    pub(in crate::features::audio) durable_recovery: bool,
    /// Derniere mutation du clip, utile pour la future retention.
    pub(in crate::features::audio) updated_at_ms: i64,
}

/// Playlist de clip rendue avec son statut de cache.
#[derive(Debug, Clone)]
pub(in crate::features::audio) struct HlsClipPlaylist {
    /// Texte HLS complet.
    pub(in crate::features::audio) body: String,
    /// `true` quand `#EXT-X-ENDLIST` est present.
    pub(in crate::features::audio) finalized: bool,
}

/// Manifest HLS persiste pour un resume calendrier.
#[derive(Debug, Clone)]
pub(crate) struct CalendarSummaryHlsManifest {
    /// Identifiant immutable de la generation audio.
    pub(crate) stream_id: String,
    /// Chemin relatif sous `NEWS_HLS_SEGMENTS_DIR`.
    pub(crate) playlist_path: String,
    /// URL publique immutable de cette generation.
    pub(crate) immutable_playlist_url: String,
    /// URL publique stable du resume courant du jour.
    pub(crate) current_playlist_url: String,
}
