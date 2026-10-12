//! Facade des DTO du contrat live edge-compatible.

// `message` porte l'enum JSON taguée envoyée sur le transport edge.
mod message;
// `payloads` porte les corps d'événements edge.
mod payloads;
// `state` porte les états partagés entre snapshot et événements.
mod state;

pub use message::EdgeLiveMessage;
pub use payloads::{
    EdgeAudioFailedPayload, EdgeAudioFinalizedPayload, EdgeAudioReadyPayload,
    EdgeAudioStatePayload, EdgeCalendarAlertReadyPayload, EdgeLiveHeartbeatPayload,
    EdgeLiveHelloPayload, EdgeLiveLagPayload, EdgeNewsCardPayload, EdgeNowPlayingPayload,
    EdgeSourceStatusPayload, EdgeSystemNoticePayload, EdgeTranslatedNewsItemPayload,
};
pub use state::{
    EdgeAudioState, EdgeLiveAudioStatus, EdgeLiveSnapshot, EdgeNewsCard, EdgeNowPlaying,
    EdgeSystemNoticeSeverity,
};
