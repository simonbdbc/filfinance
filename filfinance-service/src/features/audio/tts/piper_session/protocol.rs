//! Protocole JSONL echange avec le helper Python Piper.

use serde::{Deserialize, Serialize};

/// Format audio actuellement servi par le worker.
pub(in crate::features::audio::tts::piper_session) const AUDIO_FORMAT_WAV: &str = "wav";
/// MIME type associe au flux WAV.
pub(in crate::features::audio::tts::piper_session) const AUDIO_MIME_TYPE_WAV: &str = "audio/wav";

/// Requete JSONL envoyee au helper Python pour une synthese Piper.
#[derive(Debug, Serialize)]
pub(in crate::features::audio::tts::piper_session) struct PiperCommandPayload<'a> {
    /// Type de commande pour garder la porte ouverte a d'autres messages plus tard.
    pub(in crate::features::audio::tts::piper_session) command: &'a str,
    /// Repertoire racine des voix Piper installees.
    pub(in crate::features::audio::tts::piper_session) data_dir: &'a str,
    /// Identifiant de voix choisi par l'utilisateur ou la config.
    pub(in crate::features::audio::tts::piper_session) voice: &'a str,
    /// Texte deja normalise par le backend Rust.
    pub(in crate::features::audio::tts::piper_session) text: &'a str,
    /// Vitesse de diction approximatee depuis le contrat historique du projet.
    pub(in crate::features::audio::tts::piper_session) rate_words_per_minute: u32,
}

/// Reponse JSONL attendue du helper Piper.
#[derive(Debug, Deserialize)]
pub(in crate::features::audio::tts::piper_session) struct PiperCommandResponse {
    /// Voix effectivement utilisee pour la synthese.
    pub(in crate::features::audio::tts::piper_session) voice: Option<String>,
    /// Format audio logique du clip.
    pub(in crate::features::audio::tts::piper_session) audio_format: Option<String>,
    /// MIME type exploitable par le client.
    pub(in crate::features::audio::tts::piper_session) mime_type: Option<String>,
    /// Audio WAV serialise en base64 pour rester dans un protocole JSONL simple.
    pub(in crate::features::audio::tts::piper_session) audio_base64: Option<String>,
    /// Frequence d'echantillonnage lue par le helper dans le WAV genere.
    pub(in crate::features::audio::tts::piper_session) sample_rate_hz: Option<u32>,
    /// Message d'erreur applicatif renvoye par le helper.
    pub(in crate::features::audio::tts::piper_session) error: Option<String>,
}
