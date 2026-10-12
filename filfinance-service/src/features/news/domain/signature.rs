//! Signature de deduplication stable des evenements news.

// DefaultHasher suffit ici pour une signature de dedup non cryptographique.
use std::{
    collections::{HashMap, hash_map::DefaultHasher},
    hash::{Hash, Hasher},
};

use super::event::NewsEvent;

/// Genere une signature stable pour la dedup memoire et l'idempotence durable.
pub fn signature_for(event: &NewsEvent) -> String {
    // Le hasher est alimente dans un ordre fixe pour garder une signature deterministe.
    let mut hasher = DefaultHasher::new();
    let is_financial_juice = is_financial_juice_family(&event.source);
    signature_source_family(&event.source).hash(&mut hasher);
    event.event_kind.as_str().hash(&mut hasher);
    // Les titres FinancialJuice normalisent aussi la ponctuation source, car le meme texte
    // peut revenir sous forme `: The Information` puis `- The Information`.
    if is_financial_juice {
        financial_juice_title_key(&event.title).hash(&mut hasher);
    } else {
        // `normalized(...)` limite les faux doublons dus aux espaces/casse.
        normalized(&event.title).hash(&mut hasher);
    }
    if !is_financial_juice {
        normalized(&event.url).hash(&mut hasher);
    }
    // `news_level` est volontairement exclu: c'est un attribut enrichissant, pas l'identite d'une news.
    normalized_timestamp(
        event
            .event_time
            .as_deref()
            .or(event.published_at.as_deref())
            // `unwrap_or_default()` remplace proprement l'absence de date par la chaine vide.
            .unwrap_or_default(),
        is_financial_juice,
    )
    .hash(&mut hasher);
    // On formate en hexadecimal fixe pour un identifiant compact et stable.
    format!("{:016x}", hasher.finish())
}

fn signature_source_family(source: &str) -> &str {
    if is_financial_juice_family(source) {
        "financial_juice"
    } else {
        source
    }
}

pub(in crate::features::news) fn is_financial_juice_family(source: &str) -> bool {
    matches!(source, "financial_juice" | "financial_juice_browser")
}

/// Cle texte plus stricte pour dedupliquer les doublons quasi-identiques FinancialJuice.
pub(in crate::features::news) fn financial_juice_title_key(title: &str) -> String {
    financial_juice_title_words(title).join(" ")
}

/// Indique si deux titres FinancialJuice sont suffisamment proches pour etre bypasses.
pub(in crate::features::news) fn financial_juice_titles_are_near_duplicates(
    left: &str,
    right: &str,
) -> bool {
    let left_words = financial_juice_title_words(left);
    let right_words = financial_juice_title_words(right);
    let longer_ratio = financial_juice_word_similarity_ratio(&left_words, &right_words);

    longer_ratio >= 0.80
}

fn financial_juice_title_words(title: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();

    for character in title.trim().chars() {
        for normalized in character.to_lowercase() {
            if normalized.is_ascii_alphanumeric() {
                current.push(normalized);
                continue;
            }

            if matches!(normalized, '\'' | '`' | '’' | '‘' | 'ʼ') {
                continue;
            }

            flush_word(&mut words, &mut current);
        }
    }

    flush_word(&mut words, &mut current);
    words
}

fn flush_word(words: &mut Vec<String>, current: &mut String) {
    if current.is_empty() {
        return;
    }

    words.push(std::mem::take(current));
}

fn financial_juice_word_similarity_ratio(left: &[String], right: &[String]) -> f64 {
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }

    let mut counts: HashMap<&str, usize> = HashMap::new();
    for word in left {
        *counts.entry(word).or_default() += 1;
    }

    let mut common_words = 0usize;
    for word in right {
        let Some(count) = counts.get_mut(word.as_str()) else {
            continue;
        };

        if *count == 0 {
            continue;
        }

        *count -= 1;
        common_words += 1;
    }

    common_words as f64 / left.len().max(right.len()) as f64
}

fn normalized_timestamp(value: &str, truncate_to_minute: bool) -> String {
    if truncate_to_minute && value.len() >= 16 {
        value[..16].to_owned()
    } else {
        value.to_owned()
    }
}

/// Uniformise les chaines avant calcul de signature.
fn normalized(value: &str) -> String {
    // `trim()` retire les espaces parasites et `to_ascii_lowercase()` harmonise la casse.
    value.trim().to_ascii_lowercase()
}
