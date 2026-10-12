//! Facade HLS pour clips audio evenementiels et replay explicite du jour.

// `access` signe et verifie les tickets courts utilises sur les URLs audio publiques.
mod access;
// `http` reserve les handlers HTTP HLS et leurs contrats de cache.
pub(crate) mod http;
// `origin` reserve l'etat mutable, la publication HLS et l'encodage `ffmpeg`.
pub(crate) mod origin;
// `playlist` reserve le rendu pur des playlists HLS.
pub(crate) mod playlist;
// `runtime` reserve l'observabilite HLS.
pub(crate) mod runtime;

// La facade conserve les chemins publics des handlers HLS servis par `features::router`.
pub(crate) use http::{
    calendar_summary_current_playlist, calendar_summary_immutable_playlist,
    calendar_summary_segment, clip_playlist, clip_segment, replay_today_playlist,
    replay_today_segment,
};
// Les types d'origin necessaires au wiring et aux handlers restent exposes par la facade.
pub(crate) use origin::{HlsOriginState, HlsReplayTodayMetadata};
