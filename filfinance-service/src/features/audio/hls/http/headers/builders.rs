//! Builders `HeaderMap` des endpoints HLS.

// `HeaderMap` et `HeaderValue` construisent les réponses cacheables.
use axum::http::{HeaderMap, HeaderName, HeaderValue, header};

use super::constants::{
    CLOUDFLARE_CDN_CACHE_CONTROL_HEADER, HLS_CLIP_PLAYLIST_FINAL_CACHE_CONTROL,
    HLS_CLIP_PLAYLIST_FINAL_CDN_CACHE_CONTROL, HLS_CLIP_PLAYLIST_MUTABLE_CACHE_CONTROL,
    HLS_CLIP_PLAYLIST_MUTABLE_CDN_CACHE_CONTROL, HLS_PLAYLIST_CACHE_CONTROL,
    HLS_PLAYLIST_CDN_CACHE_CONTROL, HLS_PLAYLIST_CONTENT_TYPE, HLS_SEGMENT_CACHE_CONTROL,
    HLS_SEGMENT_CDN_CACHE_CONTROL, HLS_SEGMENT_CONTENT_TYPE, X_CONTENT_TYPE_OPTIONS_HEADER,
    X_CONTENT_TYPE_OPTIONS_NOSNIFF,
};

/// Construit les headers HTTP/CDN du manifest replay HLS.
pub(in crate::features::audio) fn playlist_response_headers() -> HeaderMap {
    // `HeaderMap` accepte des noms standard et custom, ce qui évite de dupliquer les tuples.
    let headers = playlist_base_headers(HLS_PLAYLIST_CACHE_CONTROL, HLS_PLAYLIST_CDN_CACHE_CONTROL);
    // La map complète est ensuite consommée par `IntoResponse`.
    headers
}

/// Construit les headers HTTP/CDN d'une playlist clip progressive.
pub(in crate::features::audio) fn clip_playlist_response_headers(finalized: bool) -> HeaderMap {
    // Une playlist mutable ne doit pas être figée par le navigateur avant l'arrivée du corps.
    let browser_cache = if finalized {
        HLS_CLIP_PLAYLIST_FINAL_CACHE_CONTROL
    } else {
        HLS_CLIP_PLAYLIST_MUTABLE_CACHE_CONTROL
    };
    let cdn_cache = if finalized {
        HLS_CLIP_PLAYLIST_FINAL_CDN_CACHE_CONTROL
    } else {
        HLS_CLIP_PLAYLIST_MUTABLE_CDN_CACHE_CONTROL
    };

    playlist_base_headers(browser_cache, cdn_cache)
}

/// Construit les headers HTTP/CDN d'un segment HLS immutable.
pub(in crate::features::audio) fn segment_response_headers() -> HeaderMap {
    // Même structure que la playlist, mais avec une stratégie cache longue.
    response_headers(
        HLS_SEGMENT_CONTENT_TYPE,
        HLS_SEGMENT_CACHE_CONTROL,
        HLS_SEGMENT_CDN_CACHE_CONTROL,
    )
}

/// Construit les headers communs des playlists `.m3u8`.
fn playlist_base_headers(browser_cache: &'static str, cdn_cache: &'static str) -> HeaderMap {
    // Le MIME type `.m3u8` permet au player HLS d'identifier le manifest sans sniffing.
    response_headers(HLS_PLAYLIST_CONTENT_TYPE, browser_cache, cdn_cache)
}

/// Construit le jeu commun `Content-Type`, cache navigateur, cache CDN et `nosniff`.
fn response_headers(
    content_type: &'static str,
    browser_cache: &'static str,
    cdn_cache: &'static str,
) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static(browser_cache),
    );
    // Cloudflare reçoit sa propre consigne edge, séparée du cache navigateur.
    headers.insert(
        HeaderName::from_static(CLOUDFLARE_CDN_CACHE_CONTROL_HEADER),
        HeaderValue::from_static(cdn_cache),
    );
    // `nosniff` limite les interprétations MIME surprises sur un endpoint public.
    headers.insert(
        HeaderName::from_static(X_CONTENT_TYPE_OPTIONS_HEADER),
        HeaderValue::from_static(X_CONTENT_TYPE_OPTIONS_NOSNIFF),
    );
    headers
}
