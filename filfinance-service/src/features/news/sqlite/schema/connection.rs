//! Ouverture du pool SQLite news.

use std::{path::Path, str::FromStr};

use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};

use crate::{config::StorageConfig, errors::AppError};

use super::super::SqliteNewsStore;

impl SqliteNewsStore {
    /// Ouvre le pool SQLite après avoir préparé le dossier parent si nécessaire.
    pub async fn connect(config: &StorageConfig) -> Result<Self, AppError> {
        // On crée le dossier parent avant l'ouverture du pool si l'URL SQLite vise un fichier local.
        ensure_parent_directory(&config.database_url).await?;

        // Les fichiers SQLite locaux doivent être créés explicitement si absents,
        // sinon SQLx peut remonter un `code: 14 unable to open database file`.
        let connect_options = sqlite_connect_options(&config.database_url)?;
        // `SqlitePoolOptions::new()` suit le pattern builder pour configurer clairement le pool.
        let pool = SqlitePoolOptions::new()
            .max_connections(config.max_connections)
            .connect_with(connect_options)
            .await?;

        Ok(Self { pool })
    }

    /// Clone le pool SQLite pour les stores feature-first extraits progressivement.
    pub(crate) fn clone_pool_for_feature_store(&self) -> SqlitePool {
        // SQLx clone seulement le handle du pool; toutes les features continuent donc à partager la même base.
        self.pool.clone()
    }
}

/// Crée le dossier cible de la base si l'URL SQLite pointe vers un chemin local.
async fn ensure_parent_directory(database_url: &str) -> Result<(), AppError> {
    // Le `if let ... && let ... && ...` compacte plusieurs gardes Option/condition sans pyramide de `if`.
    if let Some(path) = database_url.strip_prefix("sqlite://")
        && let Some(parent) = Path::new(path).parent()
        && !parent.as_os_str().is_empty()
    {
        // Le dossier est créé uniquement pour les URLs SQLite basées sur un chemin local.
        tokio::fs::create_dir_all(parent).await?;
    }

    Ok(())
}

/// Construit des options SQLite explicites pour les fichiers locaux et la mémoire.
fn sqlite_connect_options(database_url: &str) -> Result<SqliteConnectOptions, AppError> {
    // `from_str` parse l'URL SQLite et remonte une erreur de config si elle est invalide.
    let connect_options = SqliteConnectOptions::from_str(database_url)
        .map_err(|error| AppError::Config(format!("invalid sqlite url {database_url}: {error}")))?;

    // Le mode `create_if_missing` ne doit pas s'appliquer à `sqlite::memory:`.
    if database_url == "sqlite::memory:" {
        Ok(connect_options)
    } else {
        Ok(connect_options.create_if_missing(true))
    }
}
