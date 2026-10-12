//! Facade du worker TTS serveur.

// `lifecycle` porte le spawn, le warmup et la boucle de réception.
mod calendar_day;
mod lifecycle;
// `replay_backfill` rattrape les traductions du jour qui n'ont pas traversé le bus live.
mod replay_backfill;
// `translation` porte la transformation d'une traduction prête en audio/HLS.
mod translation;

pub(crate) use calendar_day::spawn_calendar_day_audio_worker;
pub(crate) use lifecycle::spawn_tts_worker;
