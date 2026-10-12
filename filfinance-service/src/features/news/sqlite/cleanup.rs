//! Nettoyages cibles des donnees news connues comme invalides.

use sqlx::{QueryBuilder, Sqlite};
use time::{Duration, OffsetDateTime, format_description::well_known::Rfc3339};

use super::SqliteNewsStore;
use crate::errors::AppError;

const FINANCIAL_JUICE_BROWSER_SOURCE: &str = "financial_juice_browser";

/// Rapport du nettoyage des lignes invalides produites par le fallback navigateur.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserSourceCleanupReport {
    /// Nombre de traductions supprimees avant les news parentes.
    pub deleted_translations: u64,
    /// Nombre de news supprimees dans `news_events`.
    pub deleted_news_events: u64,
}

#[derive(Debug, sqlx::FromRow)]
struct BrowserSourceCleanupCandidate {
    id: i64,
    title: String,
    published_at: Option<String>,
    news_level: Option<String>,
    raw_payload_ref: Option<String>,
}

impl SqliteNewsStore {
    /// Supprime les lignes `financial_juice_browser` futures ou issues du chrome/calendrier FJ.
    pub async fn cleanup_invalid_financial_juice_browser_events(
        &self,
        now_utc: OffsetDateTime,
    ) -> Result<BrowserSourceCleanupReport, AppError> {
        let candidates = sqlx::query_as::<_, BrowserSourceCleanupCandidate>(
            r#"
            SELECT id, title, published_at, news_level, raw_payload_ref
            FROM news_events
            WHERE source = ?1
            "#,
        )
        .bind(FINANCIAL_JUICE_BROWSER_SOURCE)
        .fetch_all(&self.pool)
        .await?;

        let ids_to_delete: Vec<i64> = candidates
            .into_iter()
            .filter(|candidate| is_invalid_browser_source_candidate(candidate, now_utc))
            .map(|candidate| candidate.id)
            .collect();

        if ids_to_delete.is_empty() {
            return Ok(BrowserSourceCleanupReport {
                deleted_translations: 0,
                deleted_news_events: 0,
            });
        }

        let deleted_translations =
            delete_rows_by_news_event_id(&self.pool, "news_translations", &ids_to_delete).await?;
        let deleted_news_events =
            delete_rows_by_id(&self.pool, "news_events", &ids_to_delete).await?;

        Ok(BrowserSourceCleanupReport {
            deleted_translations,
            deleted_news_events,
        })
    }
}

async fn delete_rows_by_news_event_id(
    pool: &sqlx::SqlitePool,
    table_name: &str,
    ids: &[i64],
) -> Result<u64, AppError> {
    let mut query_builder = QueryBuilder::<Sqlite>::new("DELETE FROM ");
    query_builder
        .push(table_name)
        .push(" WHERE news_event_id IN (");
    {
        let mut separated = query_builder.separated(", ");
        for id in ids {
            separated.push_bind(id);
        }
    }
    query_builder.push(")");

    Ok(query_builder.build().execute(pool).await?.rows_affected())
}

async fn delete_rows_by_id(
    pool: &sqlx::SqlitePool,
    table_name: &str,
    ids: &[i64],
) -> Result<u64, AppError> {
    let mut query_builder = QueryBuilder::<Sqlite>::new("DELETE FROM ");
    query_builder.push(table_name).push(" WHERE id IN (");
    {
        let mut separated = query_builder.separated(", ");
        for id in ids {
            separated.push_bind(id);
        }
    }
    query_builder.push(")");

    Ok(query_builder.build().execute(pool).await?.rows_affected())
}

fn is_invalid_browser_source_candidate(
    candidate: &BrowserSourceCleanupCandidate,
    now_utc: OffsetDateTime,
) -> bool {
    if browser_news_level_is_blocked(candidate.news_level.as_deref()) {
        return true;
    }

    if browser_raw_payload_ref_is_blocked(candidate.raw_payload_ref.as_deref()) {
        return true;
    }

    if browser_title_is_blocked(&candidate.title) {
        return true;
    }

    browser_published_at_is_future(candidate.published_at.as_deref(), now_utc)
}

fn browser_news_level_is_blocked(value: Option<&str>) -> bool {
    normalize_browser_text(value.unwrap_or_default()).contains("event-header")
}

fn browser_raw_payload_ref_is_blocked(value: Option<&str>) -> bool {
    value
        .unwrap_or_default()
        .starts_with("financial_juice_browser:text:")
}

fn browser_title_is_blocked(value: &str) -> bool {
    let normalized = normalize_browser_text(value);
    if matches!(
        normalized.as_str(),
        "affiliates"
            | "china"
            | "cny"
            | "contact us"
            | "crypto"
            | "disclaimer"
            | "energy"
            | "eur"
            | "europe"
            | "forex"
            | "germany"
            | "my account"
            | "news settings"
            | "partners"
            | "privacy policy"
            | "search"
            | "search x"
            | "sign out"
            | "tradingview"
            | "us bonds"
            | "us indexes"
            | "us stocks"
            | "usd"
    ) {
        return true;
    }

    if normalized.starts_with("copyright ") && normalized.ends_with(" financialjuice") {
        return true;
    }

    if browser_title_starts_with_embedded_et_timestamp(&normalized) {
        return true;
    }

    matches!(
        normalized.as_str(),
        "join us and go real-time" | "don't like ads? go pro"
    ) || is_calendar_date_only_title(&normalized)
}

fn browser_title_starts_with_embedded_et_timestamp(value: &str) -> bool {
    let Some(rest) = value.strip_prefix('[') else {
        return false;
    };
    let Some((time, rest)) = rest.split_once(' ') else {
        return false;
    };

    rest.starts_with("et]") && browser_clock_label_is_valid(time)
}

fn browser_clock_label_is_valid(value: &str) -> bool {
    let Some((hour, minute)) = value.split_once(':') else {
        return false;
    };
    let Ok(hour) = hour.parse::<u8>() else {
        return false;
    };
    let Ok(minute) = minute.parse::<u8>() else {
        return false;
    };

    hour <= 23 && minute <= 59
}

fn browser_published_at_is_future(value: Option<&str>, now_utc: OffsetDateTime) -> bool {
    let Some(value) = value else {
        return false;
    };
    let Ok(published_at) = OffsetDateTime::parse(value, &Rfc3339) else {
        return false;
    };

    published_at > now_utc + Duration::minutes(2)
}

fn is_calendar_date_only_title(value: &str) -> bool {
    let mut parts = value.split_whitespace();
    let Some(month) = parts.next() else {
        return false;
    };
    let Some(day) = parts.next() else {
        return false;
    };
    if parts.next().is_some() {
        return false;
    }

    matches!(
        month,
        "january"
            | "february"
            | "march"
            | "april"
            | "may"
            | "june"
            | "july"
            | "august"
            | "september"
            | "october"
            | "november"
            | "december"
    ) && day
        .parse::<u8>()
        .is_ok_and(|value| (1..=31).contains(&value))
}

fn normalize_browser_text(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
        .replace('©', "copyright")
        .replace('×', "x")
        .replace('’', "'")
}
