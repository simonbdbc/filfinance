//! DTO live partages par WebSocket et edge.

// `edge` reserve les payloads du control plane compatible Durable Objects.
mod edge;
// `item` reserve le contrat live user-facing reutilise par plusieurs transports.
mod item;
// `websocket` reserve les messages JSON du fallback WebSocket backend.
mod websocket;

#[allow(unused_imports)]
pub use edge::{
    EdgeAudioFailedPayload, EdgeAudioFinalizedPayload, EdgeAudioReadyPayload, EdgeAudioState,
    EdgeAudioStatePayload, EdgeCalendarAlertReadyPayload, EdgeLiveAudioStatus,
    EdgeLiveHeartbeatPayload, EdgeLiveHelloPayload, EdgeLiveLagPayload, EdgeLiveMessage,
    EdgeLiveSnapshot, EdgeNewsCard, EdgeNewsCardPayload, EdgeNowPlaying, EdgeNowPlayingPayload,
    EdgeSourceStatusPayload, EdgeSystemNoticePayload, EdgeSystemNoticeSeverity,
    EdgeTranslatedNewsItemPayload,
};
#[allow(unused_imports)]
pub use item::{LiveNewsHistoryResponse, LiveNewsItem};
#[allow(unused_imports)]
pub use websocket::{
    TestRunAcceptedResponse, TestTraceEvent, WsErrorPayload, WsHeartbeatPayload, WsHelloPayload,
    WsLagPayload, WsServerMessage, WsTestTraceHelloPayload, WsTestTraceMessage,
};
