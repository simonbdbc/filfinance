//! PRAGMA SQLite appliques au bootstrap news.

use crate::errors::AppError;

/// Applique les pragmas indispensables au mode WAL.
pub(super) async fn apply_sqlite_pragmas(pool: &sqlx::SqlitePool) -> Result<(), AppError> {
    // Ces PRAGMA sont posés au bootstrap pour garder un profil SQLite adapté au flux append-heavy.
    // `journal_mode = WAL` favorise les lectures concurrentes et les writes append-heavy du MVP.
    sqlx::query("PRAGMA journal_mode = WAL;")
        .execute(pool)
        .await?;
    // `synchronous = NORMAL` est un compromis courant entre sûreté et débit sur SQLite WAL.
    sqlx::query("PRAGMA synchronous = NORMAL;")
        .execute(pool)
        .await?;

    Ok(())
}
