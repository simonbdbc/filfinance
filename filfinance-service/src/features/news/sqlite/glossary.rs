//! Acces SQLite au glossaire metier traduction/TTS.

// `QueryBuilder` construit les filtres dynamiques sans interpoler les valeurs.
use sqlx::{QueryBuilder, Sqlite};

use crate::{
    errors::AppError,
    features::{
        glossary::domain::{
            GlossaryEntry, GlossaryListParams, NormalizedCreateGlossaryEntry,
            NormalizedUpdateGlossaryEntry, validate_glossary_pattern_capture_terms,
        },
        news::sqlite::SqliteNewsStore,
    },
    shared::time::now_rfc3339,
};

/// Projection SQL commune des entrees glossaire.
const GLOSSARY_SELECT: &str = r#"
SELECT
    id,
    source_term,
    translated_term,
    spoken_term,
    source_language,
    target_language,
    match_mode,
    case_sensitive,
    priority,
    enabled,
    scope,
    notes,
    created_at,
    updated_at
FROM translation_glossary_entries
"#;

impl SqliteNewsStore {
    /// Liste les entrees glossaire dans leur ordre d'application.
    pub(crate) async fn list_glossary_entries(
        &self,
        params: &GlossaryListParams,
    ) -> Result<Vec<GlossaryEntry>, AppError> {
        let mut query_builder = QueryBuilder::<Sqlite>::new(GLOSSARY_SELECT);
        query_builder.push(" WHERE 1 = 1");

        if let Some(enabled) = params.enabled {
            query_builder.push(" AND enabled = ");
            query_builder.push_bind(enabled);
        }

        if let Some(query) = &params.query {
            let pattern = format!("%{}%", query.to_ascii_lowercase());
            query_builder.push(" AND (LOWER(source_term) LIKE ");
            query_builder.push_bind(pattern.clone());
            query_builder.push(" OR LOWER(translated_term) LIKE ");
            query_builder.push_bind(pattern.clone());
            query_builder.push(" OR LOWER(COALESCE(spoken_term, '')) LIKE ");
            query_builder.push_bind(pattern.clone());
            query_builder.push(" OR LOWER(COALESCE(notes, '')) LIKE ");
            query_builder.push_bind(pattern);
            query_builder.push(")");
        }

        query_builder.push(" ORDER BY priority DESC, LENGTH(source_term) DESC, source_term ASC");
        query_builder.push(" LIMIT ");
        query_builder.push_bind(params.limit);

        let items = query_builder
            .build_query_as::<GlossaryEntry>()
            .fetch_all(&self.pool)
            .await?;

        Ok(items)
    }

    /// Compte les entrees actives pour l'observabilite runtime.
    pub(crate) async fn active_glossary_count(&self) -> Result<u64, AppError> {
        let count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM translation_glossary_entries
            WHERE enabled = 1
            "#,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(count.max(0) as u64)
    }

    /// Cree une entree glossaire.
    pub(crate) async fn create_glossary_entry(
        &self,
        entry: &NormalizedCreateGlossaryEntry,
    ) -> Result<GlossaryEntry, AppError> {
        let now = now_rfc3339();
        let id = sqlx::query(
            r#"
            INSERT INTO translation_glossary_entries (
                source_term,
                translated_term,
                spoken_term,
                source_language,
                target_language,
                match_mode,
                case_sensitive,
                priority,
                enabled,
                scope,
                notes,
                created_at,
                updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&entry.source_term)
        .bind(&entry.translated_term)
        .bind(&entry.spoken_term)
        .bind(&entry.source_language)
        .bind(&entry.target_language)
        .bind(&entry.match_mode)
        .bind(entry.case_sensitive)
        .bind(entry.priority)
        .bind(entry.enabled)
        .bind(&entry.scope)
        .bind(&entry.notes)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?
        .last_insert_rowid();

        self.get_glossary_entry(id).await
    }

    /// Met a jour partiellement une entree glossaire.
    pub(crate) async fn update_glossary_entry(
        &self,
        id: i64,
        patch: &NormalizedUpdateGlossaryEntry,
    ) -> Result<GlossaryEntry, AppError> {
        let current = self.get_glossary_entry(id).await?;
        let now = now_rfc3339();
        let source_term = patch
            .source_term
            .as_ref()
            .unwrap_or(&current.source_term)
            .clone();
        let translated_term = patch
            .translated_term
            .as_ref()
            .unwrap_or(&current.translated_term)
            .clone();
        let spoken_term = patch
            .spoken_term
            .clone()
            .unwrap_or_else(|| current.spoken_term.clone());
        let source_language = patch
            .source_language
            .as_ref()
            .unwrap_or(&current.source_language)
            .clone();
        let target_language = patch
            .target_language
            .as_ref()
            .unwrap_or(&current.target_language)
            .clone();
        let match_mode = patch
            .match_mode
            .as_ref()
            .unwrap_or(&current.match_mode)
            .clone();
        let case_sensitive = patch.case_sensitive.unwrap_or(current.case_sensitive);
        let priority = patch.priority.unwrap_or(current.priority);
        let enabled = patch.enabled.unwrap_or(current.enabled);
        let scope = patch.scope.as_ref().unwrap_or(&current.scope).clone();
        let notes = patch.notes.clone().unwrap_or_else(|| current.notes.clone());

        validate_glossary_pattern_capture_terms(
            &match_mode,
            &source_term,
            &translated_term,
            spoken_term.as_deref(),
        )?;

        sqlx::query(
            r#"
            UPDATE translation_glossary_entries
            SET
                source_term = ?,
                translated_term = ?,
                spoken_term = ?,
                source_language = ?,
                target_language = ?,
                match_mode = ?,
                case_sensitive = ?,
                priority = ?,
                enabled = ?,
                scope = ?,
                notes = ?,
                updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&source_term)
        .bind(&translated_term)
        .bind(spoken_term.as_deref())
        .bind(&source_language)
        .bind(&target_language)
        .bind(&match_mode)
        .bind(case_sensitive)
        .bind(priority)
        .bind(enabled)
        .bind(&scope)
        .bind(notes.as_deref())
        .bind(&now)
        .bind(id)
        .execute(&self.pool)
        .await?;

        self.get_glossary_entry(id).await
    }

    /// Desactive une entree glossaire sans la supprimer physiquement.
    pub(crate) async fn disable_glossary_entry(&self, id: i64) -> Result<GlossaryEntry, AppError> {
        let now = now_rfc3339();
        let result = sqlx::query(
            r#"
            UPDATE translation_glossary_entries
            SET enabled = 0, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&now)
        .bind(id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::not_found(format!("glossary entry {id}")));
        }

        self.get_glossary_entry(id).await
    }

    /// Lit une entree glossaire precise.
    async fn get_glossary_entry(&self, id: i64) -> Result<GlossaryEntry, AppError> {
        let mut query_builder = QueryBuilder::<Sqlite>::new(GLOSSARY_SELECT);
        query_builder.push(" WHERE id = ");
        query_builder.push_bind(id);

        query_builder
            .build_query_as::<GlossaryEntry>()
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| AppError::not_found(format!("glossary entry {id}")))
    }
}
