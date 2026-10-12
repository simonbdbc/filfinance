//! Formatage texte du message Telegram.

use crate::features::live::dto::LiveNewsItem;

/// Longueur maximale conservee pour le corps texte Telegram afin d'eviter des messages enormes.
const TELEGRAM_BODY_LIMIT: usize = 1_000;

/// Construit un format Telegram stable et lisible, avec l'anglais source et le francais derive.
pub(in crate::features::notifications) fn build_telegram_message(item: &LiveNewsItem) -> String {
    // On construit le message comme un tableau de lignes pour garder un format stable et facile a lire.
    let mut lines = vec![
        format!("[{}:{}][{}]", item.source, item.event_kind, item.language),
        format!("EN: {}", item.source_title),
        format!("FR: {}", item.translated_title),
    ];

    if let Some(body) = item.source_body.as_deref().and_then(non_empty_str) {
        lines.push(format!(
            "EN body: {}",
            truncate_text(body, TELEGRAM_BODY_LIMIT)
        ));
    }

    if let Some(body) = item.translated_body.as_deref().and_then(non_empty_str) {
        lines.push(format!(
            "FR body: {}",
            truncate_text(body, TELEGRAM_BODY_LIMIT)
        ));
    }

    lines.push(format!("Audio FR: {}", item.narration_text));

    lines.push(format!("timestamp: {}", item.timestamp));

    if let Some(url) = item.url.as_deref() {
        lines.push(url.to_owned());
    }

    // `join("\n")` garde une presentation simple cote Telegram sans parseur Markdown.
    lines.join("\n")
}

/// Tronque proprement un texte long pour eviter de gonfler inutilement les messages Telegram.
fn truncate_text(value: &str, limit: usize) -> String {
    let compact = value.trim();

    if compact.chars().count() <= limit {
        return compact.to_owned();
    }

    let truncated = compact.chars().take(limit).collect::<String>();
    format!("{truncated}...")
}

/// Normalise une chaine vide vers `None` pour les sorties downstream.
fn non_empty_str(value: &str) -> Option<&str> {
    let trimmed = value.trim();

    // `Option<&str>` evite d'allouer une nouvelle `String` juste pour distinguer vide / non vide.
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}
