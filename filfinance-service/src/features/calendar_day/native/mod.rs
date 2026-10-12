//! Chemin canonique Rust du calendrier, isole du pipeline live news.

// Le client OpenRouter et les prompts restent purs : ils ne portent aucun
// effet de bord SQLite, Piper, HLS ou alertes.
pub(crate) mod openrouter;
pub(crate) mod prompts;
// Les collecteurs gardent le HTML brut exclusivement en memoire et n'exposent que des audits rediges.
pub(crate) mod collectors;
// Exports officiels normalises en memoire avant OpenRouter.
pub mod feeds;
// La file SQLite reste la source de verite entre admission, reprise et dispatch audio.
pub(crate) mod work;
// Le runtime relie les contrats types aux tables `calendar_day_*` et aux workers existants.
pub(crate) mod diagnostics;
pub(crate) mod runtime;
