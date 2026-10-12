//! Domaine audio commun partage entre TTS et HLS.

// `Serialize` permet d'inclure ces metadonnees dans les traces runtime sans exposer l'audio brut.
use serde::Serialize;

use crate::features::calendar_day::native::work::CalendarAudioWorkContext;

/// Metadonnees internes associees a un clip audio HLS.
#[derive(Debug, Clone, Serialize)]
pub struct AudioStartPayload {
    /// Identifiant stable du stream audio courant.
    pub stream_id: String,
    /// Identifiant de la traduction source.
    pub translation_id: i64,
    /// Identifiant de l'evenement source.
    pub news_event_id: i64,
    /// Langue du flux parle.
    pub language: String,
    /// Source logique de la news.
    pub source: String,
    /// Type d'evenement.
    pub event_kind: String,
    /// Titre EN source.
    pub source_title: String,
    /// Titre FR prononce.
    pub translated_title: String,
    /// Run de test rattache si present.
    pub test_run_id: Option<String>,
    /// Horodatage metier expose cote client.
    pub timestamp: String,
    /// Voix TTS employee pour ce clip.
    pub voice: String,
    /// Format audio du clip.
    pub audio_format: String,
    /// MIME type du clip.
    pub mime_type: String,
    /// Taille totale attendue du clip en octets.
    pub total_bytes: usize,
    /// Nombre logique de blocs source. Il reste a 1 pour un WAV complet rendu en HLS.
    pub chunk_count: u64,
}

/// Type de clip audio produit pour le calendrier du jour.
#[derive(Debug, Clone)]
pub enum CalendarDayAudioKind {
    /// Clip descriptif d'un evenement high du calendrier.
    Item,
    /// Clip du resume global du calendrier du jour.
    Summary,
    /// Clip d'alerte T-5/T-1 avant un evenement high.
    Alert,
}

/// Travail audio asynchrone pour le calendrier du jour.
#[derive(Debug, Clone)]
pub struct CalendarDayAudioJob {
    /// Lease native a acquitter ou replanifier apres Piper/HLS.
    ///
    /// Tout job calendrier passe par la file SQLite native. Rendre ce contexte
    /// obligatoire interdit a un appelant de contourner le fencing en envoyant
    /// directement un travail dans le canal audio en memoire.
    pub(crate) native_work: CalendarAudioWorkContext,
    /// Identifiant HLS reserve durablement par l'orchestrateur calendrier.
    ///
    /// Une reprise reutilise cette valeur plutot que d'allouer un second clip
    /// lorsque le processus s'arrete entre la publication HLS et le marquage
    /// SQLite `audio_ready`.
    pub(crate) reserved_stream_id: String,
    /// Jour metier Europe/Paris du calendrier.
    pub local_day: String,
    /// Type de clip calendrier a produire.
    pub kind: CalendarDayAudioKind,
    /// Identifiant `calendar_day_items` quand le job cible un evenement.
    pub item_id: Option<i64>,
    /// Identifiant `calendar_day_summaries` quand le job cible le resume.
    pub summary_id: Option<i64>,
    /// Identifiant `calendar_day_alerts` quand le job cible une alerte.
    pub alert_id: Option<i64>,
    /// Titre court servant d'intro HLS.
    pub title: String,
    /// Texte complet a lire par Piper.
    pub narration_text: String,
    /// Timestamp metier utilise par le client pour ordonner le clip.
    pub timestamp: String,
}
