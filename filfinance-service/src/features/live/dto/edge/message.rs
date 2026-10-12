//! Message JSON tagué du transport edge live.

use serde::{Deserialize, Serialize};

use super::payloads::{
    EdgeAudioFailedPayload, EdgeAudioFinalizedPayload, EdgeAudioReadyPayload,
    EdgeAudioStatePayload, EdgeCalendarAlertReadyPayload, EdgeLiveHeartbeatPayload,
    EdgeLiveHelloPayload, EdgeLiveLagPayload, EdgeNewsCardPayload, EdgeNowPlayingPayload,
    EdgeSourceStatusPayload, EdgeSystemNoticePayload, EdgeTranslatedNewsItemPayload,
};

/// Contrat du futur WebSocket edge porté par Durable Objects.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EdgeLiveMessage {
    // Message d'ouverture contenant la version et un snapshot minimal.
    Hello {
        payload: EdgeLiveHelloPayload,
    },
    // Heartbeat sans payload métier.
    Heartbeat {
        payload: EdgeLiveHeartbeatPayload,
    },
    // Carte de news compacte, utile avant ou en parallèle de la traduction complète.
    NewsCard {
        payload: EdgeNewsCardPayload,
    },
    // Item traduit complet, réutilisant le contrat stable du flux actuel.
    TranslatedNewsItem {
        payload: Box<EdgeTranslatedNewsItemPayload>,
    },
    // Etat "now playing" du flux audio global.
    NowPlaying {
        payload: EdgeNowPlayingPayload,
    },
    // Etat public du control plane audio HLS.
    AudioState {
        payload: EdgeAudioStatePayload,
    },
    // Clip HLS prêt à être ajouté dans la queue audio client.
    AudioReady {
        payload: EdgeAudioReadyPayload,
    },
    // Alerte calendrier high prete a etre jouee apres le clip courant.
    CalendarAlertReady {
        payload: EdgeCalendarAlertReadyPayload,
    },
    // Echec de génération ou publication d'un clip audio.
    AudioFailed {
        payload: EdgeAudioFailedPayload,
    },
    // Clip HLS finalisé, utile pour l'observabilité du control plane.
    AudioFinalized {
        payload: EdgeAudioFinalizedPayload,
    },
    // Notification système non audio.
    SystemNotice {
        payload: EdgeSystemNoticePayload,
    },
    // Etat public des sources affiche par les badges.
    SourceStatus {
        payload: EdgeSourceStatusPayload,
    },
    // Signal de perte d'événements côté client.
    Lagged {
        payload: EdgeLiveLagPayload,
    },
}
