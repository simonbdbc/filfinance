//! Constantes HTTP/CDN publiques des endpoints HLS.

/// Content-Type officiel des playlists HLS.
pub(in crate::features::audio) const HLS_PLAYLIST_CONTENT_TYPE: &str =
    "application/vnd.apple.mpegurl";
/// Content-Type standard des segments MPEG-TS audio-only.
pub(in crate::features::audio) const HLS_SEGMENT_CONTENT_TYPE: &str = "video/MP2T";
/// Cache court pour que Cloudflare garde les playlists fraîches.
pub(in crate::features::audio) const HLS_PLAYLIST_CACHE_CONTROL: &str =
    "public, max-age=1, must-revalidate";
/// Cache long pour les segments immuables, qui portent un numéro de séquence unique.
pub(in crate::features::audio) const HLS_SEGMENT_CACHE_CONTROL: &str =
    "public, max-age=86400, immutable";
/// Header Cloudflare dédié: il contrôle le CDN sans être renvoyé au navigateur.
pub(in crate::features::audio) const CLOUDFLARE_CDN_CACHE_CONTROL_HEADER: &str =
    "cloudflare-cdn-cache-control";
/// TTL edge très court pour les manifests replay qui changent à chaque nouveau segment.
pub(in crate::features::audio) const HLS_PLAYLIST_CDN_CACHE_CONTROL: &str = "max-age=1";
/// Cache navigateur d'une playlist clip encore mutable: elle doit être revalidée à chaque lecture.
pub(in crate::features::audio) const HLS_CLIP_PLAYLIST_MUTABLE_CACHE_CONTROL: &str =
    "no-cache, must-revalidate";
/// Cache edge très court d'une playlist clip mutable pour ne pas figer l'intro sans le corps.
pub(in crate::features::audio) const HLS_CLIP_PLAYLIST_MUTABLE_CDN_CACHE_CONTROL: &str =
    "max-age=1";
/// Cache navigateur d'une playlist clip finalisée avec `#EXT-X-ENDLIST`.
pub(in crate::features::audio) const HLS_CLIP_PLAYLIST_FINAL_CACHE_CONTROL: &str =
    "public, max-age=300, must-revalidate";
/// Cache edge d'une playlist finalisée: le contenu ne change plus après `#EXT-X-ENDLIST`.
pub(in crate::features::audio) const HLS_CLIP_PLAYLIST_FINAL_CDN_CACHE_CONTROL: &str =
    "max-age=300";
/// TTL edge long pour les segments immuables dont l'URL ne change jamais de contenu.
pub(in crate::features::audio) const HLS_SEGMENT_CDN_CACHE_CONTROL: &str = "max-age=86400";
/// Header défensif pour empêcher un navigateur de deviner un autre type MIME.
pub(in crate::features::audio) const X_CONTENT_TYPE_OPTIONS_HEADER: &str = "x-content-type-options";
/// Valeur standard qui force le respect du `Content-Type` fourni par l'origin.
pub(in crate::features::audio) const X_CONTENT_TYPE_OPTIONS_NOSNIFF: &str = "nosniff";
