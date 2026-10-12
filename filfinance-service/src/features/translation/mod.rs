//! Feature `translation`, responsable de la derivation francaise du flux news.

// `domain` reserve les types metier de traduction.
pub(crate) mod domain;
// `http` reserve les handlers runtime et historique traduction.
pub(crate) mod http;
// `provider` reserve le port de provider de traduction.
pub(crate) mod provider;
// `providers` reserve les implementations concretes de traduction.
pub(crate) mod providers;
// `runtime` reserve l'etat d'observabilite de traduction.
pub(crate) mod runtime;
// `service` porte le worker et les use cases de traduction.
pub(crate) mod service;
// Les tests traduction communs restent separes du code de production.
#[cfg(test)]
mod tests;

// La facade conserve le chemin public `features::translation::TranslationRuntimeState`.
pub(crate) use runtime::TranslationRuntimeState;
// Le bootstrap lance le worker via la facade sans connaitre l'implementation interne.
pub(crate) use service::spawn;
