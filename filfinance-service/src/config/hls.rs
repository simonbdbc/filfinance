//! Configuration de l'origin HLS audio.

// Les helpers d'environnement gardent le parsing NEWS_* uniforme dans tous les sous-modules.
use super::{
    HlsConfig, TtsConfig,
    environment::{env_bool, env_optional, env_or, env_u32, env_u64},
};

// Les erreurs de validation doivent remonter au bootstrap applicatif.
use crate::errors::AppError;

impl HlsConfig {
    /// Construit la configuration HLS depuis l'environnement.
    pub fn from_env() -> Result<Self, AppError> {
        // Le flag reste false par defaut tant que l'environnement n'a pas besoin de clips/replay HLS.
        let enabled = env_bool("NEWS_HLS_ENABLED", false)?;
        // Le dossier par defaut reste sous `data/` pour etre monte facilement dans Docker.
        let segments_dir = env_or("NEWS_HLS_SEGMENTS_DIR", "data/hls");
        // `ffmpeg` reste une commande configurable pour les environnements ou le binaire est ailleurs.
        let ffmpeg_command = env_or("NEWS_HLS_FFMPEG_COMMAND", "ffmpeg");
        // Le replay n'est pas le chemin live temps reel; une cible plus longue reste acceptable.
        let replay_segment_target_duration_secs =
            env_u32("NEWS_HLS_REPLAY_SEGMENT_TARGET_DURATION_SECS", 4)?;
        // Les clips evenementiels visent une latence plus basse que le replay.
        let clip_segment_target_duration_secs =
            env_u32("NEWS_HLS_CLIP_SEGMENT_TARGET_DURATION_SECS", 2)?;
        // Le controle reste explicite pour ne pas casser les environnements locaux sans CDN.
        let access_control_enabled = env_bool("NEWS_HLS_ACCESS_CONTROL_ENABLED", false)?;
        // Secret partage par le backend pour signer et verifier les tickets HLS.
        let access_ticket_secret = env_optional("NEWS_HLS_ACCESS_TICKET_SECRET");
        // Bucket long par defaut pour couvrir un replay du jour sans renouvellement natif HLS.
        let access_ticket_ttl_secs = env_u64("NEWS_HLS_ACCESS_TICKET_TTL_SECS", 3600)?;

        Ok(Self {
            enabled,
            access_control_enabled,
            access_ticket_secret,
            access_ticket_ttl_secs,
            segments_dir,
            ffmpeg_command,
            replay_segment_target_duration_secs,
            clip_segment_target_duration_secs,
        })
    }

    /// Retourne une configuration HLS desactivee pratique pour les tests unitaires.
    #[cfg(test)]
    pub fn disabled_for_tests() -> Self {
        // Les tests historiques du service n'ont pas besoin d'encoder d'audio.
        Self {
            enabled: false,
            access_control_enabled: false,
            access_ticket_secret: None,
            access_ticket_ttl_secs: 3600,
            segments_dir: "data/hls-test".to_owned(),
            ffmpeg_command: "ffmpeg".to_owned(),
            replay_segment_target_duration_secs: 4,
            clip_segment_target_duration_secs: 2,
        }
    }
}

/// Valide la coherence de l'origin HLS avec la TTS source.
pub(super) fn validate_hls_config(config: &HlsConfig, tts: &TtsConfig) -> Result<(), AppError> {
    // HLS desactive ne doit pas imposer `ffmpeg` aux environnements backend texte seulement.
    if !config.enabled {
        return Ok(());
    }

    // Sans TTS, aucun WAV source ne peut alimenter les segments HLS.
    if !tts.enabled {
        return Err(AppError::Config(
            "NEWS_TTS_ENABLED must be true when NEWS_HLS_ENABLED=true".to_owned(),
        ));
    }

    // Un dossier vide ne peut pas recevoir les segments produits.
    if config.segments_dir.trim().is_empty() {
        return Err(AppError::Config(
            "NEWS_HLS_SEGMENTS_DIR cannot be empty when NEWS_HLS_ENABLED=true".to_owned(),
        ));
    }

    // `ffmpeg` est le pont d'encodage WAV interne -> AAC/MPEG-TS public.
    if config.ffmpeg_command.trim().is_empty() {
        return Err(AppError::Config(
            "NEWS_HLS_FFMPEG_COMMAND cannot be empty when NEWS_HLS_ENABLED=true".to_owned(),
        ));
    }

    // Une duree nulle rendrait la playlist HLS invalide.
    if config.replay_segment_target_duration_secs == 0 {
        return Err(AppError::Config(
            "NEWS_HLS_REPLAY_SEGMENT_TARGET_DURATION_SECS must be greater than 0".to_owned(),
        ));
    }

    // Une duree nulle rendrait les playlists clip invalides et bloquerait `audio_ready`.
    if config.clip_segment_target_duration_secs == 0 {
        return Err(AppError::Config(
            "NEWS_HLS_CLIP_SEGMENT_TARGET_DURATION_SECS must be greater than 0".to_owned(),
        ));
    }

    if config.access_control_enabled {
        if config
            .access_ticket_secret
            .as_deref()
            .is_none_or(|secret| secret.trim().len() < 32)
        {
            return Err(AppError::Config(
                "NEWS_HLS_ACCESS_TICKET_SECRET must contain at least 32 characters when NEWS_HLS_ACCESS_CONTROL_ENABLED=true"
                    .to_owned(),
            ));
        }

        if config.access_ticket_ttl_secs == 0 {
            return Err(AppError::Config(
                "NEWS_HLS_ACCESS_TICKET_TTL_SECS must be greater than 0".to_owned(),
            ));
        }
    }

    Ok(())
}
