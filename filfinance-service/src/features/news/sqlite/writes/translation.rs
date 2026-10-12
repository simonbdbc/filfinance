//! Upsert des traductions persistantes SQLite.

use super::super::SqliteNewsStore;
use crate::{
    errors::AppError,
    features::{
        news::domain::PersistedNewsEvent,
        translation::domain::{NewsTranslationDraft, PersistedNewsTranslation},
    },
    shared::time::now_rfc3339,
};

impl SqliteNewsStore {
    /// Insere ou met a jour une variante texte pour un evenement et une langue donnees.
    pub async fn upsert_translation(
        &self,
        event: &PersistedNewsEvent,
        draft: &NewsTranslationDraft,
    ) -> Result<PersistedNewsTranslation, AppError> {
        // On capture une seule fois les timestamps pour garder l'upsert coherent.
        let created_at = now_rfc3339();
        let updated_at = created_at.clone();

        // `ON CONFLICT(news_event_id, language)` fait de `(event, langue)` la cle fonctionnelle
        // d'une traduction, ce qui colle au modele metier du service.
        sqlx::query(
            r#"
            INSERT INTO news_translations (
                news_event_id,
                language,
                status,
                source,
                event_kind,
                source_title,
                source_body,
                translated_title,
                translated_body,
                narration_text,
                translation_mode,
                error_message,
                published_at,
                created_at,
                updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
            ON CONFLICT(news_event_id, language) DO UPDATE SET
                status = excluded.status,
                source = excluded.source,
                event_kind = excluded.event_kind,
                source_title = excluded.source_title,
                source_body = excluded.source_body,
                translated_title = excluded.translated_title,
                translated_body = excluded.translated_body,
                narration_text = excluded.narration_text,
                translation_mode = excluded.translation_mode,
                error_message = excluded.error_message,
                published_at = excluded.published_at,
                updated_at = excluded.updated_at;
            "#,
        )
        .bind(event.id)
        .bind(&draft.language)
        .bind(&draft.status)
        .bind(&event.source)
        .bind(&event.event_kind)
        .bind(&event.title)
        .bind(&event.body)
        .bind(&draft.translated_title)
        .bind(&draft.translated_body)
        .bind(&draft.narration_text)
        .bind(&draft.translation_mode)
        .bind(&draft.error_message)
        .bind(&event.published_at)
        .bind(&created_at)
        .bind(&updated_at)
        .execute(&self.pool)
        .await?;

        // On relit ensuite la ligne finalisee pour retourner exactement la projection persistente.
        self.get_translation_by_news_event_id_and_language(event.id, &draft.language)
            .await?
            .ok_or_else(|| {
                // `ok_or_else(...)` convertit ici un `Option` en `Result` avec une erreur paresseuse.
                AppError::storage(format!(
                    "translation row missing after upsert for news_event_id={} language={}",
                    event.id, draft.language
                ))
            })
    }
}
