//! Persistence SQLite des parametres du collecteur X.

// `SqlitePool` partage la meme base locale que le pipeline news/audio.
use sqlx::SqlitePool;

use crate::{
    errors::AppError,
    features::{ingestion::runtime::XRuntimeSettings, news::NewsStore},
    shared::time::now_rfc3339,
};

/// Store SQLite dedie aux parametres admin de la source X.
#[derive(Clone)]
pub struct XSettingsStore {
    pool: SqlitePool,
}

impl XSettingsStore {
    /// Ouvre le store X depuis le pool SQLite principal.
    pub fn from_news_store(storage: &NewsStore) -> XSettingsStore {
        Self {
            pool: storage.clone_pool_for_feature_store(),
        }
    }

    /// Cree la table locale si elle n'existe pas encore.
    pub async fn migrate(&self) -> Result<(), AppError> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS x_source_settings (
                id TEXT PRIMARY KEY DEFAULT 'default',
                enabled INTEGER NOT NULL DEFAULT 0,
                account_usernames_json TEXT NOT NULL DEFAULT '[]',
                keywords_json TEXT NOT NULL DEFAULT '[]',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                CHECK (id = 'default')
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            INSERT OR IGNORE INTO schema_migrations (name)
            VALUES ('0011_create_x_source_settings_sqlite');
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Lit les parametres X persistés s'ils existent.
    pub async fn load(&self) -> Result<Option<XRuntimeSettings>, AppError> {
        let row = sqlx::query_as::<_, XSettingsRow>(
            r#"
            SELECT
                enabled,
                account_usernames_json,
                keywords_json,
                updated_at
            FROM x_source_settings
            WHERE id = 'default'
            LIMIT 1;
            "#,
        )
        .fetch_optional(&self.pool)
        .await?;

        row.map(XRuntimeSettings::try_from).transpose()
    }

    /// Persiste puis retourne la ligne X canonique.
    pub async fn save(&self, settings: &XRuntimeSettings) -> Result<XRuntimeSettings, AppError> {
        let now = now_rfc3339();
        let account_usernames_json = serde_json::to_string(&settings.account_usernames)?;
        let keywords_json = serde_json::to_string(&settings.keywords)?;
        let row = sqlx::query_as::<_, XSettingsRow>(
            r#"
            INSERT INTO x_source_settings (
                id,
                enabled,
                account_usernames_json,
                keywords_json,
                created_at,
                updated_at
            )
            VALUES ('default', ?1, ?2, ?3, ?4, ?4)
            ON CONFLICT (id) DO UPDATE SET
                enabled = excluded.enabled,
                account_usernames_json = excluded.account_usernames_json,
                keywords_json = excluded.keywords_json,
                updated_at = excluded.updated_at
            RETURNING
                enabled,
                account_usernames_json,
                keywords_json,
                updated_at;
            "#,
        )
        .bind(settings.enabled)
        .bind(account_usernames_json)
        .bind(keywords_json)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        XRuntimeSettings::try_from(row)
    }
}

/// Projection SQLx de `x_source_settings`.
#[derive(Debug, sqlx::FromRow)]
struct XSettingsRow {
    enabled: bool,
    account_usernames_json: String,
    keywords_json: String,
    updated_at: String,
}

impl TryFrom<XSettingsRow> for XRuntimeSettings {
    type Error = AppError;

    fn try_from(row: XSettingsRow) -> Result<Self, Self::Error> {
        let account_usernames = serde_json::from_str::<Vec<String>>(&row.account_usernames_json)?;
        let keywords = serde_json::from_str::<Vec<String>>(&row.keywords_json)?;

        Ok(Self {
            enabled: row.enabled,
            disabled_reason: None,
            account_usernames,
            keywords,
            updated_at: Some(row.updated_at),
        })
    }
}
