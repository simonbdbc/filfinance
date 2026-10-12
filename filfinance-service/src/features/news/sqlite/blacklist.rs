//! Acces SQLite a la blacklist news et aux audits associes.

use sqlx::{QueryBuilder, Sqlite};

use crate::{
    errors::AppError,
    features::{
        blacklist::domain::{
            NewNewsBlacklistHit, NewNewsTruncationAudit, NewsAuditListParams, NewsBlacklistEntry,
            NewsBlacklistHit, NewsBlacklistListParams, NewsTruncationAudit,
            NormalizedCreateNewsBlacklistEntry, NormalizedUpdateNewsBlacklistEntry,
        },
        news::sqlite::SqliteNewsStore,
    },
    shared::time::now_rfc3339,
};

/// Projection SQL commune des regles blacklist.
const NEWS_BLACKLIST_SELECT: &str = r#"
SELECT
    id,
    term,
    match_mode,
    target_scope,
    enabled,
    notes,
    created_at,
    updated_at
FROM news_blacklist_entries
"#;

/// Projection SQL commune des hits blacklist.
const NEWS_BLACKLIST_HIT_SELECT: &str = r#"
SELECT
    id,
    blacklist_entry_id,
    term,
    match_mode,
    target_scope,
    action,
    source,
    event_kind,
    matched_field,
    matched_text,
    source_title,
    source_body_excerpt,
    raw_payload_ref,
    signature,
    created_at
FROM news_blacklist_hits
"#;

/// Projection SQL commune des audits de troncature upstream.
const NEWS_TRUNCATION_AUDIT_SELECT: &str = r#"
SELECT
    id,
    source,
    field_name,
    marker,
    source_title,
    field_excerpt,
    raw_payload_ref,
    signature,
    created_at
FROM news_truncation_audit
"#;

impl SqliteNewsStore {
    /// Liste les regles blacklist dans leur ordre d'application.
    pub(crate) async fn list_news_blacklist_entries(
        &self,
        params: &NewsBlacklistListParams,
    ) -> Result<Vec<NewsBlacklistEntry>, AppError> {
        let mut query_builder = QueryBuilder::<Sqlite>::new(NEWS_BLACKLIST_SELECT);
        query_builder.push(" WHERE 1 = 1");

        if let Some(enabled) = params.enabled {
            query_builder.push(" AND enabled = ");
            query_builder.push_bind(enabled);
        }

        if let Some(query) = &params.query {
            let pattern = format!("%{}%", query.to_ascii_lowercase());
            query_builder.push(" AND (LOWER(term) LIKE ");
            query_builder.push_bind(pattern.clone());
            query_builder.push(" OR LOWER(COALESCE(notes, '')) LIKE ");
            query_builder.push_bind(pattern);
            query_builder.push(")");
        }

        query_builder.push(" ORDER BY enabled DESC, LENGTH(term) DESC, term ASC");
        query_builder.push(" LIMIT ");
        query_builder.push_bind(params.limit);

        Ok(query_builder
            .build_query_as::<NewsBlacklistEntry>()
            .fetch_all(&self.pool)
            .await?)
    }

    /// Cree une regle blacklist.
    pub(crate) async fn create_news_blacklist_entry(
        &self,
        entry: &NormalizedCreateNewsBlacklistEntry,
    ) -> Result<NewsBlacklistEntry, AppError> {
        let now = now_rfc3339();
        let id = sqlx::query(
            r#"
            INSERT INTO news_blacklist_entries (
                term,
                match_mode,
                target_scope,
                enabled,
                notes,
                created_at,
                updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&entry.term)
        .bind(&entry.match_mode)
        .bind(&entry.target_scope)
        .bind(entry.enabled)
        .bind(&entry.notes)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?
        .last_insert_rowid();

        self.get_news_blacklist_entry(id).await
    }

    /// Met a jour partiellement une regle blacklist.
    pub(crate) async fn update_news_blacklist_entry(
        &self,
        id: i64,
        patch: &NormalizedUpdateNewsBlacklistEntry,
    ) -> Result<NewsBlacklistEntry, AppError> {
        let current = self.get_news_blacklist_entry(id).await?;
        let now = now_rfc3339();

        sqlx::query(
            r#"
            UPDATE news_blacklist_entries
            SET
                term = ?,
                match_mode = ?,
                target_scope = ?,
                enabled = ?,
                notes = ?,
                updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(patch.term.as_ref().unwrap_or(&current.term))
        .bind(patch.match_mode.as_ref().unwrap_or(&current.match_mode))
        .bind(patch.target_scope.as_ref().unwrap_or(&current.target_scope))
        .bind(patch.enabled.unwrap_or(current.enabled))
        .bind(patch.notes.clone().unwrap_or_else(|| current.notes.clone()))
        .bind(&now)
        .bind(id)
        .execute(&self.pool)
        .await?;

        self.get_news_blacklist_entry(id).await
    }

    /// Desactive une regle blacklist sans supprimer son historique.
    pub(crate) async fn disable_news_blacklist_entry(
        &self,
        id: i64,
    ) -> Result<NewsBlacklistEntry, AppError> {
        let now = now_rfc3339();
        let result = sqlx::query(
            r#"
            UPDATE news_blacklist_entries
            SET enabled = 0, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&now)
        .bind(id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::not_found(format!("news blacklist entry {id}")));
        }

        self.get_news_blacklist_entry(id).await
    }

    /// Enregistre un rejet blacklist.
    pub(crate) async fn insert_news_blacklist_hit(
        &self,
        hit: &NewNewsBlacklistHit,
    ) -> Result<NewsBlacklistHit, AppError> {
        let now = now_rfc3339();
        let id = sqlx::query(
            r#"
            INSERT INTO news_blacklist_hits (
                blacklist_entry_id,
                term,
                match_mode,
                target_scope,
                action,
                source,
                event_kind,
                matched_field,
                matched_text,
                source_title,
                source_body_excerpt,
                raw_payload_ref,
                signature,
                created_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(hit.blacklist_entry_id)
        .bind(&hit.term)
        .bind(&hit.match_mode)
        .bind(&hit.target_scope)
        .bind(&hit.action)
        .bind(&hit.source)
        .bind(&hit.event_kind)
        .bind(&hit.matched_field)
        .bind(&hit.matched_text)
        .bind(&hit.source_title)
        .bind(&hit.source_body_excerpt)
        .bind(&hit.raw_payload_ref)
        .bind(&hit.signature)
        .bind(&now)
        .execute(&self.pool)
        .await?
        .last_insert_rowid();

        self.get_news_blacklist_hit(id).await
    }

    /// Liste les derniers rejets blacklist.
    pub(crate) async fn list_news_blacklist_hits(
        &self,
        params: &NewsAuditListParams,
    ) -> Result<Vec<NewsBlacklistHit>, AppError> {
        let mut query_builder = QueryBuilder::<Sqlite>::new(NEWS_BLACKLIST_HIT_SELECT);
        query_builder.push(" ORDER BY id DESC LIMIT ");
        query_builder.push_bind(params.limit);

        Ok(query_builder
            .build_query_as::<NewsBlacklistHit>()
            .fetch_all(&self.pool)
            .await?)
    }

    /// Supprime une news rejetee par blacklist avec les lignes derivees directes.
    pub(crate) async fn delete_news_event_for_blacklist_reapply(
        &self,
        news_event_id: i64,
    ) -> Result<(), AppError> {
        sqlx::query("DELETE FROM daily_news_qualifications WHERE news_event_id = ?")
            .bind(news_event_id)
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM news_translations WHERE news_event_id = ?")
            .bind(news_event_id)
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM news_events WHERE id = ?")
            .bind(news_event_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Retire le contenu source/traduit d'une news sans supprimer son titre.
    pub(crate) async fn strip_news_event_content_for_blacklist_reapply(
        &self,
        news_event_id: i64,
        language: &str,
    ) -> Result<(), AppError> {
        let now = now_rfc3339();
        sqlx::query("UPDATE news_events SET body = NULL WHERE id = ?")
            .bind(news_event_id)
            .execute(&self.pool)
            .await?;
        sqlx::query(
            r#"
            UPDATE news_translations
            SET source_body = NULL,
                translated_body = NULL,
                updated_at = ?
            WHERE news_event_id = ?
              AND language = ?
            "#,
        )
        .bind(now)
        .bind(news_event_id)
        .bind(language)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Enregistre une troncature upstream de maniere idempotente par signature/champ/marqueur.
    pub(crate) async fn insert_news_truncation_audit(
        &self,
        audit: &NewNewsTruncationAudit,
    ) -> Result<(), AppError> {
        let now = now_rfc3339();
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO news_truncation_audit (
                source,
                field_name,
                marker,
                source_title,
                field_excerpt,
                raw_payload_ref,
                signature,
                created_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&audit.source)
        .bind(&audit.field_name)
        .bind(&audit.marker)
        .bind(&audit.source_title)
        .bind(&audit.field_excerpt)
        .bind(&audit.raw_payload_ref)
        .bind(&audit.signature)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Liste les dernieres troncatures upstream detectees.
    pub(crate) async fn list_news_truncation_audits(
        &self,
        params: &NewsAuditListParams,
    ) -> Result<Vec<NewsTruncationAudit>, AppError> {
        let mut query_builder = QueryBuilder::<Sqlite>::new(NEWS_TRUNCATION_AUDIT_SELECT);
        query_builder.push(" ORDER BY id DESC LIMIT ");
        query_builder.push_bind(params.limit);

        Ok(query_builder
            .build_query_as::<NewsTruncationAudit>()
            .fetch_all(&self.pool)
            .await?)
    }

    /// Lit une regle precise.
    async fn get_news_blacklist_entry(&self, id: i64) -> Result<NewsBlacklistEntry, AppError> {
        let mut query_builder = QueryBuilder::<Sqlite>::new(NEWS_BLACKLIST_SELECT);
        query_builder.push(" WHERE id = ");
        query_builder.push_bind(id);

        query_builder
            .build_query_as::<NewsBlacklistEntry>()
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| AppError::not_found(format!("news blacklist entry {id}")))
    }

    /// Lit un hit blacklist precis.
    async fn get_news_blacklist_hit(&self, id: i64) -> Result<NewsBlacklistHit, AppError> {
        let mut query_builder = QueryBuilder::<Sqlite>::new(NEWS_BLACKLIST_HIT_SELECT);
        query_builder.push(" WHERE id = ");
        query_builder.push_bind(id);

        query_builder
            .build_query_as::<NewsBlacklistHit>()
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| AppError::not_found(format!("news blacklist hit {id}")))
    }
}
