//! Clips HLS progressifs.

// `publish` porte l'intro, le corps et le commit mémoire des clips.
mod publish;
// `read` porte la playlist et les segments servis en HTTP.
mod read;
// `storage` porte le déplacement des segments rendus vers le dossier immutable du clip.
mod storage;
