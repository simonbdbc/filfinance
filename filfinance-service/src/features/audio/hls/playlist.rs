//! Rendu de playlists HLS.

use super::origin::{HlsClip, HlsClipPlaylist};

/// Prefixe public des clips HLS finis par news.
const HLS_CLIP_BASE_URL: &str = "/audio/clips";
/// Prefixe public des resumes calendrier persistants.
const HLS_CALENDAR_SUMMARY_BASE_URL: &str = "/audio/calendar-summary";

/// Rend une playlist HLS `EVENT` pour un clip progressif.
pub(in crate::features::audio) fn render_clip_playlist(clip: &HlsClip) -> HlsClipPlaylist {
    let target_duration = playlist_target_duration(
        clip.target_duration_secs,
        clip.segments.iter().map(|segment| segment.duration_secs),
    );
    let mut playlist = String::new();
    playlist.push_str("#EXTM3U\n");
    playlist.push_str("#EXT-X-VERSION:3\n");
    playlist.push_str("#EXT-X-PLAYLIST-TYPE:EVENT\n");
    playlist.push_str(&format!("#EXT-X-TARGETDURATION:{target_duration}\n"));
    playlist.push_str("#EXT-X-MEDIA-SEQUENCE:0\n");

    for segment in &clip.segments {
        if segment.discontinuity_before {
            playlist.push_str("#EXT-X-DISCONTINUITY\n");
        }
        playlist.push_str(&format!("#EXTINF:{:.3},\n", segment.duration_secs));
        playlist.push_str(&format!(
            "{}/{}/{}\n",
            HLS_CLIP_BASE_URL, clip.stream_id, segment.name
        ));
    }

    if clip.finalized {
        playlist.push_str("#EXT-X-ENDLIST\n");
    }

    HlsClipPlaylist {
        body: playlist,
        finalized: clip.finalized,
    }
}

/// Construit l'URL publique d'une playlist clip a partir de son `stream_id`.
pub(super) fn clip_playlist_url(stream_id: &str) -> String {
    format!("{HLS_CLIP_BASE_URL}/{stream_id}/playlist.m3u8")
}

/// Construit l'URL publique immutable d'un resume calendrier.
pub(in crate::features::audio) fn calendar_summary_immutable_playlist_url(
    local_day: &str,
    stream_id: &str,
) -> String {
    format!("{HLS_CALENDAR_SUMMARY_BASE_URL}/{local_day}/{stream_id}/playlist.m3u8")
}

/// Construit l'URL publique stable du resume calendrier courant d'un jour.
pub(in crate::features::audio) fn calendar_summary_current_playlist_url(local_day: &str) -> String {
    format!("{HLS_CALENDAR_SUMMARY_BASE_URL}/{local_day}/current.m3u8")
}

/// Rend une playlist HLS finale pour un resume calendrier persiste sur disque.
pub(in crate::features::audio) fn render_calendar_summary_playlist(
    clip: &HlsClip,
    local_day: &str,
    stream_id: &str,
) -> HlsClipPlaylist {
    let target_duration = playlist_target_duration(
        clip.target_duration_secs,
        clip.segments.iter().map(|segment| segment.duration_secs),
    );
    let mut playlist = String::new();
    playlist.push_str("#EXTM3U\n");
    playlist.push_str("#EXT-X-VERSION:3\n");
    playlist.push_str("#EXT-X-PLAYLIST-TYPE:VOD\n");
    playlist.push_str(&format!("#EXT-X-TARGETDURATION:{target_duration}\n"));
    playlist.push_str("#EXT-X-MEDIA-SEQUENCE:0\n");

    for segment in &clip.segments {
        if segment.discontinuity_before {
            playlist.push_str("#EXT-X-DISCONTINUITY\n");
        }
        playlist.push_str(&format!("#EXTINF:{:.3},\n", segment.duration_secs));
        playlist.push_str(&format!(
            "{HLS_CALENDAR_SUMMARY_BASE_URL}/{local_day}/{stream_id}/{}\n",
            segment.name
        ));
    }

    playlist.push_str("#EXT-X-ENDLIST\n");

    HlsClipPlaylist {
        body: playlist,
        finalized: true,
    }
}

/// Calcule `#EXT-X-TARGETDURATION`, qui doit couvrir le plus long segment annonce.
pub(in crate::features::audio) fn playlist_target_duration(
    configured_target_duration_secs: u32,
    durations: impl Iterator<Item = f64>,
) -> u32 {
    // La valeur de config reste le plancher pour eviter une target duration a zero sur playlist vide.
    durations
        .fold(
            configured_target_duration_secs.max(1),
            |target, duration| target.max(duration.ceil() as u32),
        )
        .max(1)
}
