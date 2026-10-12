//! Arguments de lancement du helper Piper local.

/// Ajoute `--session` une seule fois pour garder le helper Piper charge en memoire.
pub(in crate::features::audio::tts::piper_session) fn ensure_piper_session_args(
    args: &[String],
) -> Vec<String> {
    let mut resolved = if args.is_empty() {
        vec![default_piper_helper_script_path()]
    } else {
        args.to_vec()
    };

    if !resolved.iter().any(|arg| arg == "--session") {
        resolved.push("--session".to_owned());
    }

    resolved
}

/// Retourne le chemin par defaut du helper Piper versionne avec le service.
fn default_piper_helper_script_path() -> String {
    format!(
        "{}/scripts/piper_local_session.py",
        env!("CARGO_MANIFEST_DIR")
    )
}
