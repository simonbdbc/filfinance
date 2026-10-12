//! Replay HLS explicite du jour.

// `publish` porte l'encodage, la planification et le commit des segments replay.
mod publish;
// `read` porte la playlist, les segments et la metadata HTTP du replay.
mod read;
