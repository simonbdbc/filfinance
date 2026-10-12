//! Feature `audio`, responsable de la TTS et de l'origin HLS.

// `domain` reserve les types audio communs lorsque TTS et HLS seront entierement decouples.
pub(crate) mod domain;
// `hls` sert les clips evenementiels et le replay du jour.
pub(crate) mod hls;
// `service` reserve les cas d'usage audio transverses.
pub(crate) mod service;
// `tts` transforme les traductions FR en audio.
pub(crate) mod tts;
// Les tests audio communs restent separes des tests TTS/HLS.
#[cfg(test)]
mod tests;
