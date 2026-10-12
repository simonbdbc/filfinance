//! Lifecycle et operations du provider TTS.

use crate::{
    config::TtsConfig,
    errors::AppError,
    features::audio::tts::provider::{
        kind::TtsProvider,
        types::{SynthesizedAudio, TtsRequest},
    },
};

impl TtsProvider {
    /// Retourne le nom du provider pour les logs et les payloads d'accueil.
    pub(in crate::features::audio) fn provider_name(&self) -> &str {
        match self {
            Self::PiperLocalSession(provider) => provider.provider_name(),
        }
    }

    /// Retourne la voix actuellement active.
    pub(in crate::features::audio) fn voice(&self) -> String {
        match self {
            Self::PiperLocalSession(provider) => provider.voice(),
        }
    }

    /// Permet de detecter les erreurs de provider avant la premiere vraie news.
    pub(in crate::features::audio) async fn warmup(
        &mut self,
        config: &TtsConfig,
    ) -> Result<(), AppError> {
        if !config.warmup_enabled {
            return Ok(());
        }

        // Le warmup passe par `synthesize` pour tester le provider complet, pas seulement la config.
        let request = TtsRequest {
            text: config.warmup_text.clone(),
        };
        let _ = self.synthesize(&request).await?;
        Ok(())
    }

    /// Synthesise un clip audio complet a partir d'un texte.
    pub(in crate::features::audio) async fn synthesize(
        &mut self,
        request: &TtsRequest,
    ) -> Result<SynthesizedAudio, AppError> {
        let mut diagnostic = crate::features::calendar_logs::Operation::start(
            "tts.synthesize",
            serde_json::json!({"text":request.text,"provider":self.provider_name(),"voice":self.voice()}),
        );
        let result = async {
            match self {
                // Le `match` sur l'enum cache les details du provider au reste du worker.
                Self::PiperLocalSession(provider) => provider.synthesize(&request.text).await,
            }
        }
        .await;
        let metadata = result.as_ref().map(|audio| serde_json::json!({"bytes":audio.bytes.len(),"format":audio.audio_format,"mime_type":audio.mime_type,"voice":audio.voice}));
        diagnostic.complete(&metadata);
        result
    }

    /// Libere les ressources du provider si necessaire.
    pub(in crate::features::audio) async fn shutdown(&mut self) {
        match self {
            Self::PiperLocalSession(provider) => provider.shutdown().await,
        }
    }
}
