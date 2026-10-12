//! Facade du bootstrap SQLite de la feature news.

// `connection` porte l'ouverture du pool et les options SQLite.
mod connection;
// `helpers` porte les petites lectures de schema idempotentes.
mod helpers;
// `migration` porte le bootstrap des tables, colonnes et indexes news.
mod migration;
