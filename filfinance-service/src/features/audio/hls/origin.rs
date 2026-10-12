//! Etat, publication et encodage HLS.

// `clips` reserve la publication et la lecture des clips HLS progressifs.
mod clips;
// `encoder` reserve l'appel ffmpeg et le rendu WAV -> segments HLS.
mod encoder;
// `helpers` reserve les parsers de chemins, l'horloge locale et les estimations audio.
mod helpers;
// `metrics` reserve les compteurs et snapshots runtime HLS.
mod metrics;
// `replay` reserve la publication et la lecture du replay HLS du jour.
mod replay;
// `state` reserve l'etat mutable partage entre worker TTS et handlers HTTP.
mod state;
// `types` reserve les DTO et structures internes de l'origin HLS.
mod types;

// Ces helpers restent réexportés pour les tests HLS et les modules audio voisins.
#[allow(unused_imports)]
pub(in crate::features::audio) use helpers::{
    current_local_day, estimate_wav_duration_secs, local_day_for_rfc3339, local_day_for_utc,
    now_unix_ms, parse_clip_segment_name, parse_segment_sequence,
};
// Ces types internes restent accessibles depuis `origin` pour conserver les chemins existants.
#[allow(unused_imports)]
pub(in crate::features::audio) use types::{
    CalendarSummaryHlsManifest, HlsClip, HlsClipPlaylist, HlsClipSegment, HlsOriginInner,
    HlsSegment,
};
// Ces types restent réexportés pour conserver les chemins publics de l'origin HLS.
#[allow(unused_imports)]
pub use types::{
    HlsClipFinalizeMetrics, HlsClipInitialPublish, HlsReplayPublishMetrics, HlsReplayTodayMetadata,
};
// L'etat origin conserve son chemin public historique via la facade `origin`.
pub use state::HlsOriginState;

/// Bitrate AAC du premier palier, suffisant pour une voix mono.
const HLS_AUDIO_BITRATE: &str = "64k";
/// URL publique stable de la playlist replay du jour.
pub(in crate::features::audio) const HLS_REPLAY_TODAY_PLAYLIST_URL: &str =
    "/audio/replay/today/playlist.m3u8";
