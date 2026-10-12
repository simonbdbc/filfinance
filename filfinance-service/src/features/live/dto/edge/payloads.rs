//! Facade des payloads événementiels du contrat edge live.

// Les familles de payloads restent separees pour garder les contrats edge lisibles.
mod audio;
mod connection;
mod news;
mod sources;
mod system;

pub use audio::{
    EdgeAudioFailedPayload, EdgeAudioFinalizedPayload, EdgeAudioReadyPayload,
    EdgeAudioStatePayload, EdgeCalendarAlertReadyPayload, EdgeNowPlayingPayload,
};
pub use connection::{EdgeLiveHeartbeatPayload, EdgeLiveHelloPayload, EdgeLiveLagPayload};
pub use news::{EdgeNewsCardPayload, EdgeTranslatedNewsItemPayload};
pub use sources::EdgeSourceStatusPayload;
pub use system::EdgeSystemNoticePayload;
