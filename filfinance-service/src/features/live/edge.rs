//! Facade du control plane edge live.

// `audio` reserve la publication des evenements audio edge non bloquants.
mod audio;
// `publisher` reserve le client HTTP et la politique de retry vers le Worker edge.
mod publisher;
// `ticket` reserve la signature des tickets courts pour le WebSocket edge.
mod ticket;
// `translation` reserve le mapping des traductions persistantes vers le contrat edge.
mod translation;
// `worker` reserve la boucle Rust -> Worker edge.
mod worker;

#[allow(unused_imports)]
pub(crate) use audio::{
    edge_event_id_for_audio_failed, edge_event_id_for_audio_finalized,
    edge_event_id_for_audio_ready, edge_event_id_for_calendar_alert,
};
pub use audio::{
    publish_audio_failed, publish_audio_finalized, publish_audio_ready,
    publish_calendar_alert_ready,
};
#[allow(unused_imports)]
pub use ticket::{
    EdgeLiveTicketResponse, create_edge_live_ticket, create_live_ticket, verify_live_ticket,
};
#[cfg(test)]
pub(crate) use translation::{
    build_translated_news_item_message, edge_event_id_for_translation_id,
    is_publishable_translation,
};
pub use worker::spawn;
