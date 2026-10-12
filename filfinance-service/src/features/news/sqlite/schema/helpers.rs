//! Helpers de lecture du schema SQLite news.

use crate::errors::AppError;

use super::super::SqliteNewsStore;

impl SqliteNewsStore {
    /// Vérifie si une colonne existe déjà dans la table cible pour garder les migrations idempotentes.
    pub(in crate::features::news) async fn column_exists(
        &self,
        table: &str,
        column: &str,
    ) -> Result<bool, AppError> {
        // `query_scalar::<_, i64>` dit à SQLx qu'on veut une seule colonne scalaire de type `i64`.
        let exists = sqlx::query_scalar::<_, i64>(
            "SELECT 1 FROM pragma_table_info(?) WHERE name = ? LIMIT 1",
        )
        .bind(table)
        .bind(column)
        .fetch_optional(&self.pool)
        .await?
        .is_some();

        Ok(exists)
    }
}
