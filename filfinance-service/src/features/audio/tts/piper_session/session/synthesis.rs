//! Synthese audio exposee par le provider Piper garde en session.

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};
use tokio::time::{Duration, timeout};

use crate::{errors::AppError, features::audio::tts::provider::SynthesizedAudio};

use super::super::protocol::{AUDIO_FORMAT_WAV, AUDIO_MIME_TYPE_WAV, PiperCommandPayload};
use super::PiperLocalSessionProvider;

impl PiperLocalSessionProvider {
    /// Genere un clip WAV via le helper Python `Piper` garde en session.
    pub(in crate::features::audio::tts) async fn synthesize(
        &mut self,
        text: &str,
    ) -> Result<SynthesizedAudio, AppError> {
        // On clone d'abord les valeurs dynamiques pour liberer tout emprunt immuable avant
        // l'appel mutable a `send_session_command`.
        let current_voice = self.runtime_state.current_voice();
        let models_dir = self.models_dir.clone();
        let response = timeout(
            Duration::from_millis(self.timeout_ms),
            self.send_session_command(PiperCommandPayload {
                command: "synthesize",
                data_dir: &models_dir,
                voice: &current_voice,
                text,
                rate_words_per_minute: self.rate_words_per_minute,
            }),
        )
        .await
        .map_err(|_| {
            AppError::downstream(format!(
                "tts session command timed out after {}ms",
                self.timeout_ms
            ))
        })??;

        if let Some(error) = response.error {
            return Err(AppError::downstream(error));
        }

        let voice = response
            .voice
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| self.runtime_state.current_voice());
        let audio_base64 = response.audio_base64.ok_or_else(|| {
            AppError::downstream("tts session response is missing audio_base64".to_owned())
        })?;
        let bytes = BASE64_STANDARD.decode(audio_base64).map_err(|error| {
            AppError::downstream(format!(
                "tts session returned invalid base64 audio: {error}"
            ))
        })?;

        if bytes.is_empty() {
            return Err(AppError::downstream(
                "tts session returned an empty audio payload".to_owned(),
            ));
        }

        Ok(SynthesizedAudio {
            bytes,
            audio_format: response
                .audio_format
                .unwrap_or_else(|| AUDIO_FORMAT_WAV.to_owned()),
            mime_type: response
                .mime_type
                .unwrap_or_else(|| AUDIO_MIME_TYPE_WAV.to_owned()),
            voice,
        })
    }
}
