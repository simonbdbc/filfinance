//! Socle partage volontairement limite entre les features.

// `auth_context` reserve la place du contexte utilisateur transversal lorsque l'auth sera separee plus finement.
pub(crate) mod auth_context;
// `pagination` reserve les primitives de pagination partagees par les APIs de lecture.
pub(crate) mod pagination;
// `text` reserve les helpers texte vraiment transverses, hors logique metier d'une feature.
pub(crate) mod text;
// `time` reserve les helpers temporels reutilisables hors domaine news.
pub(crate) mod time;
