//! Segments HLS du replay du jour.

// `PathBuf` garde les chemins des segments HLS immutable sur disque.
use std::path::PathBuf;

/// Segment HLS connu par l'origin replay du jour.
#[derive(Debug, Clone)]
pub(in crate::features::audio) struct HlsSegment {
    /// Numero monotone expose dans l'URL replay `/audio/replay/today/segments/{day}/{sequence}.ts`.
    pub(in crate::features::audio) sequence: u64,
    /// Duree annoncee dans `#EXTINF`.
    pub(in crate::features::audio) duration_secs: f64,
    /// Chemin local du fichier `.ts` deja encode.
    pub(in crate::features::audio) file_path: PathBuf,
    /// Jour local produit, utile pour repartir proprement a minuit.
    pub(in crate::features::audio) local_day: String,
    /// Traduction qui a produit ce segment, quand l'index courant le connait.
    pub(in crate::features::audio) translation_id: Option<i64>,
    /// News source de la traduction, utile au diagnostic du replay journalier.
    pub(in crate::features::audio) news_event_id: Option<i64>,
    /// Stream TTS d'origine, conserve pour relier les segments aux logs audio.
    pub(in crate::features::audio) stream_id: Option<String>,
    /// Timestamp metier de la traduction, utilise pour rejouer les rattrapages dans l'ordre du feed.
    pub(in crate::features::audio) timestamp: Option<String>,
}
