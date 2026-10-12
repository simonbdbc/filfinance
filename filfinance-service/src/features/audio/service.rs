//! Facade des cas d'usage audio serveur.

// `progressive` réserve les clips HLS live et les événements audio du control-plane.
mod progressive;
// `segmented_tts` découpe les corps longs avant Piper, puis recompose un WAV unique.
mod segmented_tts;
// `stream` réserve les helpers de préparation audio et de construction des payloads HLS.
mod stream;
// `worker` réserve la boucle TTS long-vivante.
mod worker;

// Le module TTS conserve son point d'entrée historique `audio::service::spawn_tts_worker`.
pub(crate) use worker::{spawn_calendar_day_audio_worker, spawn_tts_worker};

// Ces helpers restent réexportés pour les tests audio et modules voisins.
#[allow(unused_imports)]
pub(in crate::features::audio) use stream::{build_live_clip_body_text, next_audio_stream_id};
