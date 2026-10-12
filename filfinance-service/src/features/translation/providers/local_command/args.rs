//! Resolution des arguments de commande locale selon le profil traduction.

use crate::{
    config::{TranslationConfig, translation_profile_beam_size},
    features::translation::provider::{
        OPUS_MT_LOCAL_COMMAND_PROVIDER, OPUS_MT_LOCAL_SESSION_PROVIDER,
    },
};

/// Applique le profil uniquement aux aliases OPUS-MT; les providers generiques restent inchanges.
pub(crate) fn resolved_local_command_args(config: &TranslationConfig) -> Vec<String> {
    match config.provider.as_str() {
        OPUS_MT_LOCAL_COMMAND_PROVIDER | OPUS_MT_LOCAL_SESSION_PROVIDER => {
            translation_command_args_for_profile(&config.local_command_args, &config.profile)
        }
        _ => config.local_command_args.clone(),
    }
}

/// Ajoute le `--beam-size` effectif derive du profil OPUS-MT.
pub(crate) fn translation_command_args_for_profile(args: &[String], profile: &str) -> Vec<String> {
    let Some(beam_size) = translation_profile_beam_size(profile) else {
        // La validation de config refuse ce cas, mais le helper reste defensif pour les tests directs.
        return args.to_vec();
    };

    let mut resolved_args = Vec::with_capacity(args.len() + 2);
    resolved_args.extend_from_slice(args);
    resolved_args.push("--beam-size".to_owned());
    resolved_args.push(beam_size.to_string());

    resolved_args
}
