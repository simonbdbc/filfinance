//! Identifiants stables des providers de traduction.

/// Identifiant stable du fallback purement heuristique.
pub(crate) const HEURISTIC_PROVIDER: &str = "heuristic_fr";
/// Identifiant generique d'un provider local appele via une commande.
pub(crate) const LOCAL_COMMAND_PROVIDER: &str = "local_command";
/// Alias explicite du mode local vise pour brancher OPUS-MT.
pub(crate) const OPUS_MT_LOCAL_COMMAND_PROVIDER: &str = "opus_mt_local_command";
/// Variante session gardant le helper charge en memoire via stdin/stdout.
pub(crate) const LOCAL_COMMAND_SESSION_PROVIDER: &str = "local_command_session";
/// Alias explicite du mode session vise pour brancher OPUS-MT.
pub(crate) const OPUS_MT_LOCAL_SESSION_PROVIDER: &str = "opus_mt_local_session";
