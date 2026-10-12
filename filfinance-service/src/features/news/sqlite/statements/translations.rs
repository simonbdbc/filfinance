//! Projections SQL des traductions news persistees.

/// Projection SQL commune pour toutes les lectures de variantes texte derivees.
// Meme logique pour les traductions: une seule projection stable alimente toutes les requetes de lecture.
pub(in crate::features::news::sqlite) const NEWS_TRANSLATION_SELECT: &str = r#"
    SELECT
        id,
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
    FROM news_translations
"#;

/// Projection SQL des traductions avec jointure sur `news_events` pour filtrer les runs de test.
pub(in crate::features::news::sqlite) const NEWS_TRANSLATION_HISTORY_SELECT: &str = r#"
    SELECT
        news_translations.id AS id,
        news_translations.news_event_id AS news_event_id,
        news_translations.language AS language,
        news_translations.status AS status,
        news_translations.source AS source,
        news_translations.event_kind AS event_kind,
        news_translations.source_title AS source_title,
        news_translations.source_body AS source_body,
        news_translations.translated_title AS translated_title,
        news_translations.translated_body AS translated_body,
        news_translations.narration_text AS narration_text,
        news_translations.translation_mode AS translation_mode,
        news_translations.error_message AS error_message,
        news_translations.published_at AS published_at,
        news_translations.created_at AS created_at,
        news_translations.updated_at AS updated_at
    FROM news_translations
    INNER JOIN news_events ON news_events.id = news_translations.news_event_id
"#;
