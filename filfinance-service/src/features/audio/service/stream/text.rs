//! Construction du texte lu dans les clips audio live.

use crate::features::live::dto::LiveNewsItem;

/// Construit le corps audio d'un clip live sans repeter le prefixe deja lu par l'intro.
pub(in crate::features::audio) fn build_live_clip_body_text(live_item: &LiveNewsItem) -> String {
    // Le titre traduit est le minimum audible attendu pour une news.
    let title = live_item.translated_title.trim();
    // Le corps traduit, quand il existe, prolonge le titre sans repeter le meme texte.
    let body = live_item
        .translated_body
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .filter(|value| *value != title);
    // On assemble explicitement les phrases pour eviter que `narration_text` ne relise `HH:MM`.
    let mut text = match body {
        Some(body) if !title.is_empty() => format!("{title}. {body}"),
        Some(body) => body.to_owned(),
        None => title.to_owned(),
    };
    // Piper n'a pas besoin d'une ponctuation parfaite, mais une phrase terminee sonne mieux.
    if !text.is_empty() && !text.ends_with(['.', '!', '?']) {
        text.push('.');
    }

    text
}
