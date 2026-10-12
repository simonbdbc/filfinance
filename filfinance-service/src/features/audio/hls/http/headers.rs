//! Facade des contrats de cache et headers HTTP/CDN des endpoints HLS.

// `builders` porte la construction `HeaderMap`, `constants` porte les contrats testés.
mod builders;
mod constants;

pub(in crate::features::audio) use builders::{
    clip_playlist_response_headers, playlist_response_headers, segment_response_headers,
};
pub(in crate::features::audio) use constants::{
    CLOUDFLARE_CDN_CACHE_CONTROL_HEADER, HLS_CLIP_PLAYLIST_FINAL_CACHE_CONTROL,
    HLS_CLIP_PLAYLIST_FINAL_CDN_CACHE_CONTROL, HLS_CLIP_PLAYLIST_MUTABLE_CACHE_CONTROL,
    HLS_CLIP_PLAYLIST_MUTABLE_CDN_CACHE_CONTROL, HLS_PLAYLIST_CACHE_CONTROL,
    HLS_PLAYLIST_CDN_CACHE_CONTROL, HLS_PLAYLIST_CONTENT_TYPE, HLS_SEGMENT_CACHE_CONTROL,
    HLS_SEGMENT_CDN_CACHE_CONTROL, HLS_SEGMENT_CONTENT_TYPE, X_CONTENT_TYPE_OPTIONS_HEADER,
    X_CONTENT_TYPE_OPTIONS_NOSNIFF,
};
