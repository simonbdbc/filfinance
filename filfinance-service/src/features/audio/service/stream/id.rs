//! Generation des identifiants de streams audio.

use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

/// Compteur monotone pour construire des identifiants de stream stables sans dependance externe.
static NEXT_AUDIO_STREAM_SEQUENCE: AtomicU64 = AtomicU64::new(1);

/// Construit un identifiant de stream audio stable sans crate supplementaire.
pub(in crate::features::audio) fn next_audio_stream_id() -> String {
    // Le timestamp humain aide le debug, la sequence atomique evite les collisions dans la meme milliseconde.
    let unix_millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    let sequence = NEXT_AUDIO_STREAM_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!("audio-{unix_millis}-{sequence}")
}
