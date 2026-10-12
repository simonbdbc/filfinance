//! Facade des handlers HTTP HLS et contrats de cache/CDN.

mod calendar_summary;
mod clips;
mod headers;
mod metadata;
mod replay;
mod ticket;

#[allow(unused_imports)]
pub(in crate::features::audio) use headers::{
    CLOUDFLARE_CDN_CACHE_CONTROL_HEADER, HLS_CLIP_PLAYLIST_FINAL_CACHE_CONTROL,
    HLS_CLIP_PLAYLIST_FINAL_CDN_CACHE_CONTROL, HLS_CLIP_PLAYLIST_MUTABLE_CACHE_CONTROL,
    HLS_CLIP_PLAYLIST_MUTABLE_CDN_CACHE_CONTROL, HLS_PLAYLIST_CACHE_CONTROL,
    HLS_PLAYLIST_CDN_CACHE_CONTROL, HLS_PLAYLIST_CONTENT_TYPE, HLS_SEGMENT_CACHE_CONTROL,
    HLS_SEGMENT_CDN_CACHE_CONTROL, HLS_SEGMENT_CONTENT_TYPE, X_CONTENT_TYPE_OPTIONS_HEADER,
    X_CONTENT_TYPE_OPTIONS_NOSNIFF, clip_playlist_response_headers, playlist_response_headers,
    segment_response_headers,
};
pub use metadata::get_replay_today;
pub use ticket::create_hls_access_ticket_handler;

pub(crate) use calendar_summary::{
    calendar_summary_current_playlist, calendar_summary_immutable_playlist,
    calendar_summary_segment,
};
pub(crate) use clips::{clip_playlist, clip_segment};
pub(crate) use replay::{replay_today_playlist, replay_today_segment};
