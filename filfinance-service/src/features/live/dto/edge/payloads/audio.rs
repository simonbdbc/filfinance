//! Payloads edge du control plane audio HLS.

use serde::{Deserialize, Serialize};

use super::super::state::{EdgeAudioState, EdgeNowPlaying};

/// Payload edge de changement de "now playing".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeNowPlayingPayload {
    /// Identifiant opaque et monotone fourni par le backend publisher.
    pub event_id: String,
    /// Date serveur d'émission.
    pub server_time: String,
    /// Metadata de lecture courante, ou `null` si rien n'est en cours.
    pub now_playing: Option<EdgeNowPlaying>,
}

/// Payload edge de changement d'état audio.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeAudioStatePayload {
    /// Identifiant opaque et monotone fourni par le backend publisher.
    pub event_id: String,
    /// Date serveur d'émission.
    pub server_time: String,
    /// Etat audio HLS public.
    pub state: EdgeAudioState,
}

/// Payload edge qui annonce qu'un clip HLS de news peut être ajouté à la queue audio.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeAudioReadyPayload {
    /// Identifiant opaque et monotone fourni par le backend publisher.
    pub event_id: String,
    /// Date serveur d'émission.
    pub server_time: String,
    /// Identifiant de traduction rattaché au clip audio.
    pub translation_id: i64,
    /// Identifiant de la news source, utile pour la corrélation et les smokes produit.
    pub news_event_id: i64,
    /// Timestamp métier de la news, utilisé par les clients comme watermark audio.
    pub timestamp: String,
    /// Identifiant stable du clip HLS, utilisé pour la déduplication.
    pub stream_id: String,
    /// URL de la playlist HLS progressive du clip.
    pub playlist_url: String,
    /// Date serveur à laquelle le premier segment est devenu disponible.
    pub published_at: String,
    /// Durée cible des segments du clip.
    pub target_duration_secs: u32,
}

/// Payload edge qui annonce qu'une alerte calendrier high peut passer en priorite apres le clip courant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeCalendarAlertReadyPayload {
    /// Identifiant opaque et monotone fourni par le backend publisher.
    pub event_id: String,
    /// Date serveur d'emission.
    pub server_time: String,
    /// Identifiant canonique de l'evenement calendrier.
    pub calendar_item_id: i64,
    /// Offset d'alerte en secondes avant l'evenement, par exemple 300 ou 60.
    pub alert_offset_secs: u64,
    /// Horaire source de l'evenement high.
    pub scheduled_at: String,
    /// Identifiant stable du clip HLS d'alerte.
    pub stream_id: String,
    /// URL de la playlist HLS de l'alerte.
    pub playlist_url: String,
    /// Priorite client; `high` passe devant les clips en attente sans couper le clip courant.
    pub priority: String,
    /// Date apres laquelle l'alerte ne doit plus etre jouee.
    pub expires_at: String,
    /// Date serveur a laquelle le premier segment est devenu disponible.
    pub published_at: String,
}

/// Payload edge qui annonce l'échec d'un clip audio avant ou pendant sa publication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeAudioFailedPayload {
    /// Identifiant opaque et monotone fourni par le backend publisher.
    pub event_id: String,
    /// Date serveur d'émission.
    pub server_time: String,
    /// Traduction concernée si elle était déjà connue.
    pub translation_id: Option<i64>,
    /// Clip concerné si le stream avait déjà été réservé.
    pub stream_id: Option<String>,
    /// Message court exploitable par le client ou les logs edge.
    pub error_message: String,
    /// Date serveur de constat de l'échec.
    pub failed_at: String,
}

/// Payload edge optionnel qui signale qu'un clip HLS est maintenant finalisé.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeAudioFinalizedPayload {
    /// Identifiant opaque et monotone fourni par le backend publisher.
    pub event_id: String,
    /// Date serveur d'émission.
    pub server_time: String,
    /// Identifiant de traduction rattaché au clip audio.
    pub translation_id: i64,
    /// Identifiant stable du clip HLS finalisé.
    pub stream_id: String,
    /// URL de la playlist HLS qui doit maintenant contenir `#EXT-X-ENDLIST`.
    pub playlist_url: String,
    /// Date serveur de finalisation du clip.
    pub finalized_at: String,
}
