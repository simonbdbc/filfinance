//! Projections SQL des evenements news persistes.

/// Projection SQL commune pour toutes les lectures d'evenements persistes.
// Centraliser ce `SELECT` evite de dupliquer la liste des colonnes dans chaque methode de lecture.
pub(in crate::features::news::sqlite) const NEWS_EVENT_SELECT: &str = r#"
    SELECT
        id,
        source,
        event_kind,
        title,
        url,
        published_at,
        event_time,
        ingested_at,
        body,
        news_level,
        importance,
        country_code,
        currency,
        actual_value,
        forecast_value,
        previous_value,
        raw_payload_ref,
        signature
    FROM news_events
"#;

/// Projection SQL des `news_events` avec alias explicites, utile dès qu'une jointure entre en jeu.
pub(in crate::features::news::sqlite) const NEWS_EVENT_QUALIFIED_SELECT: &str = r#"
    SELECT
        news_events.id AS id,
        news_events.source AS source,
        news_events.event_kind AS event_kind,
        news_events.title AS title,
        news_events.url AS url,
        news_events.published_at AS published_at,
        news_events.event_time AS event_time,
        news_events.ingested_at AS ingested_at,
        news_events.body AS body,
        news_events.news_level AS news_level,
        news_events.importance AS importance,
        news_events.country_code AS country_code,
        news_events.currency AS currency,
        news_events.actual_value AS actual_value,
        news_events.forecast_value AS forecast_value,
        news_events.previous_value AS previous_value,
        news_events.raw_payload_ref AS raw_payload_ref,
        news_events.signature AS signature
    FROM news_events
"#;
