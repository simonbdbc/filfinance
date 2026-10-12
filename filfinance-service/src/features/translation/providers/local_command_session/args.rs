//! Arguments propres au helper local long-vivant.

/// Ajoute automatiquement le flag `--session` pour un helper long-vivant.
pub(super) fn ensure_session_flag(args: &[String]) -> Vec<String> {
    if args.iter().any(|arg| arg == "--session") {
        return args.to_vec();
    }

    // On ajoute `--session` sans modifier le tableau d'origine pour garder la config immuable.
    let mut session_args = args.to_vec();
    session_args.push("--session".to_owned());
    session_args
}
