//! Application du glossaire dans le pipeline EN -> FR -> vocal.

use super::domain::{
    GLOSSARY_SCOPE_FULL_PIPELINE, GLOSSARY_SCOPE_SPOKEN_ONLY, GlossaryEntry, MATCH_MODE_EXACT,
    MATCH_MODE_PATTERN, MATCH_MODE_WORD_BOUNDARY,
};

/// Resultat d'application du glossaire sur un texte source.
#[derive(Debug, Clone)]
pub(crate) struct SourceGlossaryApplication {
    /// Texte source avec placeholders stables avant appel OPUS-MT.
    pub text: String,
    /// Remplacements a restaurer sur le texte traduit.
    protections: Vec<GlossaryProtection>,
    /// Nombre de termes effectivement touches.
    pub hits: u64,
}

/// Protection d'un terme source par placeholder.
#[derive(Debug, Clone)]
struct GlossaryProtection {
    /// Placeholder transmis au provider de traduction.
    placeholder: String,
    /// Terme francais a restaurer dans le texte affiche.
    translated_term: String,
}

impl SourceGlossaryApplication {
    /// Restaure les placeholders puis impose les termes affiches restants.
    pub(crate) fn restore_display_text(
        &self,
        translated_text: &str,
        entries: &[GlossaryEntry],
    ) -> String {
        let restored = self
            .protections
            .iter()
            .fold(translated_text.to_owned(), |acc, protection| {
                acc.replace(&protection.placeholder, &protection.translated_term)
            });
        apply_display_glossary(&restored, entries)
    }
}

/// Protege les termes source avant traduction.
pub(crate) fn apply_source_glossary(
    text: &str,
    entries: &[GlossaryEntry],
) -> SourceGlossaryApplication {
    let mut next_text = text.to_owned();
    let mut protections = Vec::new();
    let mut hits = 0u64;

    for entry in sorted_display_entries(entries) {
        if entry.match_mode == MATCH_MODE_PATTERN {
            if !glossary_entry_protects_source(entry) {
                let (_, count) = replace_pattern_term(
                    &next_text,
                    &entry.source_term,
                    &entry.source_term,
                    entry.case_sensitive,
                    NumberMode::Raw,
                );
                hits += count;
                continue;
            }

            let (replaced, pattern_protections) = protect_pattern_term(
                &next_text,
                &entry.source_term,
                &entry.translated_term,
                entry.case_sensitive,
                protections.len(),
            );
            if pattern_protections.is_empty() {
                continue;
            }

            hits += pattern_protections.len() as u64;
            next_text = replaced;
            protections.extend(pattern_protections);
            continue;
        }

        let placeholder = format!("ZXQGLOSSARY{}QXZ", protections.len());
        let (replaced, count) = replace_term(
            &next_text,
            &entry.source_term,
            &placeholder,
            &entry.match_mode,
            entry.case_sensitive,
        );
        if count == 0 {
            continue;
        }

        hits += count;
        next_text = replaced;
        protections.push(GlossaryProtection {
            placeholder,
            translated_term: entry.translated_term.clone(),
        });
    }

    SourceGlossaryApplication {
        text: next_text,
        protections,
        hits,
    }
}

/// Indique si une entree doit proteger le texte source avant traduction.
pub(crate) fn glossary_entry_protects_source(entry: &GlossaryEntry) -> bool {
    entry.applies_to_display_pipeline()
        && (entry.match_mode != MATCH_MODE_PATTERN
            || pattern_should_protect_source(&entry.source_term))
}

/// Applique les termes FR affiches si le provider a laisse le terme source intact.
pub(crate) fn apply_display_glossary(text: &str, entries: &[GlossaryEntry]) -> String {
    sorted_display_entries(entries)
        .into_iter()
        .fold(text.to_owned(), |acc, entry| {
            let (replaced, _) = replace_term(
                &acc,
                &entry.source_term,
                &entry.translated_term,
                &entry.match_mode,
                entry.case_sensitive,
            );
            replaced
        })
}

/// Retourne les termes affiches attendus pour les entrees qui protegent la source.
pub(crate) fn source_glossary_display_targets(
    text: &str,
    entries: &[GlossaryEntry],
) -> Vec<String> {
    let mut targets = Vec::new();

    for entry in sorted_display_entries(entries) {
        if !glossary_entry_protects_source(entry) {
            continue;
        }

        if entry.match_mode == MATCH_MODE_PATTERN {
            targets.extend(pattern_display_targets(
                text,
                &entry.source_term,
                &entry.translated_term,
                entry.case_sensitive,
            ));
            continue;
        }

        let (_, count) = replace_term(
            text,
            &entry.source_term,
            &entry.translated_term,
            &entry.match_mode,
            entry.case_sensitive,
        );
        for _ in 0..count {
            targets.push(entry.translated_term.clone());
        }
    }

    deduplicate_non_empty(targets)
}

/// Applique les formes vocales uniquement au texte envoye a Piper.
pub(crate) fn apply_spoken_glossary(text: &str, entries: &[GlossaryEntry]) -> (String, u64) {
    let mut next_text = text.to_owned();
    let mut hits = 0u64;

    for entry in sorted_entries(entries) {
        let Some(spoken_term) = entry.spoken_term.as_deref() else {
            continue;
        };
        let (replaced, count) = replace_spoken_term(&next_text, entry, spoken_term);
        if count > 0 {
            hits += count;
            next_text = replaced;
        }
    }

    (next_text, hits)
}

/// Trie les entrees dans l'ordre fonctionnel attendu par le pipeline.
fn sorted_entries(entries: &[GlossaryEntry]) -> Vec<&GlossaryEntry> {
    let mut sorted = entries.iter().collect::<Vec<_>>();
    sorted.sort_by(|left, right| {
        right
            .priority
            .cmp(&left.priority)
            .then_with(|| {
                right
                    .source_term
                    .chars()
                    .count()
                    .cmp(&left.source_term.chars().count())
            })
            .then_with(|| left.source_term.cmp(&right.source_term))
    });
    sorted
}

fn sorted_display_entries(entries: &[GlossaryEntry]) -> Vec<&GlossaryEntry> {
    sorted_entries(entries)
        .into_iter()
        .filter(|entry| entry.applies_to_display_pipeline())
        .collect()
}

fn pattern_should_protect_source(pattern: &str) -> bool {
    parse_pattern_parts(pattern).is_some_and(|parts| {
        parts.iter().any(|part| match part {
            PatternPart::Literal(literal) => literal.chars().any(char::is_alphanumeric),
            PatternPart::Number | PatternPart::Text => false,
        })
    })
}

fn pattern_display_targets(
    text: &str,
    pattern: &str,
    replacement: &str,
    case_sensitive: bool,
) -> Vec<String> {
    let Some(parts) = parse_pattern_parts(pattern) else {
        return Vec::new();
    };

    let mut cursor = 0usize;
    let mut targets = Vec::new();

    while cursor < text.len() {
        let Some((_, end, captures)) =
            find_next_pattern_match(text, cursor, &parts, case_sensitive)
        else {
            break;
        };

        if let Some(target) = render_pattern_replacement(replacement, &captures, NumberMode::Raw) {
            targets.push(target);
        }
        cursor = end;
    }

    targets
}

fn deduplicate_non_empty(values: Vec<String>) -> Vec<String> {
    let mut deduped = Vec::new();
    for value in values {
        let trimmed = value.trim();
        if trimmed.is_empty() || deduped.iter().any(|existing| existing == trimmed) {
            continue;
        }
        deduped.push(trimmed.to_owned());
    }
    deduped
}

/// Remplace un terme selon le mode de matching choisi.
fn replace_term(
    text: &str,
    needle: &str,
    replacement: &str,
    match_mode: &str,
    case_sensitive: bool,
) -> (String, u64) {
    if match_mode == MATCH_MODE_PATTERN {
        return replace_pattern_term(text, needle, replacement, case_sensitive, NumberMode::Raw);
    }

    if needle.is_empty() {
        return (text.to_owned(), 0);
    }

    let mut output = String::with_capacity(text.len());
    let haystack = if case_sensitive {
        text.to_owned()
    } else {
        text.to_ascii_lowercase()
    };
    let normalized_needle = if case_sensitive {
        needle.to_owned()
    } else {
        needle.to_ascii_lowercase()
    };
    let mut cursor = 0usize;
    let mut hits = 0u64;

    while let Some(relative_index) = haystack[cursor..].find(&normalized_needle) {
        let start = cursor + relative_index;
        let end = start + normalized_needle.len();
        output.push_str(&text[cursor..start]);

        if accepts_match(text, start, end, match_mode) {
            output.push_str(replacement);
            hits += 1;
        } else {
            output.push_str(&text[start..end]);
        }

        cursor = end;
    }

    output.push_str(&text[cursor..]);
    (output, hits)
}

fn replace_spoken_term(text: &str, entry: &GlossaryEntry, spoken_term: &str) -> (String, u64) {
    if entry.match_mode == MATCH_MODE_PATTERN {
        return replace_pattern_term(
            text,
            &entry.translated_term,
            spoken_term,
            entry.case_sensitive,
            NumberMode::French,
        );
    }

    replace_term(
        text,
        &entry.translated_term,
        spoken_term,
        MATCH_MODE_WORD_BOUNDARY,
        false,
    )
}

#[derive(Debug, Clone, Copy)]
enum NumberMode {
    Raw,
    French,
}

#[derive(Debug, Clone, Copy)]
enum NumberRender {
    Default,
    Hour,
    Minute,
}

#[derive(Debug, Clone, Copy)]
enum ReplacementCaptureToken {
    Number(NumberRender),
    Text,
}

#[derive(Debug, Clone)]
enum PatternPart {
    Literal(String),
    Number,
    Text,
}

#[derive(Debug, Clone)]
enum PatternCapture {
    Number(String),
    Text(String),
}

fn replace_pattern_term(
    text: &str,
    pattern: &str,
    replacement: &str,
    case_sensitive: bool,
    number_mode: NumberMode,
) -> (String, u64) {
    let Some(parts) = parse_pattern_parts(pattern) else {
        return (text.to_owned(), 0);
    };

    let mut output = String::with_capacity(text.len());
    let mut cursor = 0usize;
    let mut hits = 0u64;

    while cursor < text.len() {
        let Some((start, end, captures)) =
            find_next_pattern_match(text, cursor, &parts, case_sensitive)
        else {
            output.push_str(&text[cursor..]);
            return (output, hits);
        };

        output.push_str(&text[cursor..start]);
        if let Some(rendered) = render_pattern_replacement(replacement, &captures, number_mode) {
            output.push_str(&rendered);
            hits += 1;
        } else {
            output.push_str(&text[start..end]);
        }
        cursor = end;
    }

    (output, hits)
}

fn protect_pattern_term(
    text: &str,
    pattern: &str,
    replacement: &str,
    case_sensitive: bool,
    first_placeholder_index: usize,
) -> (String, Vec<GlossaryProtection>) {
    let Some(parts) = parse_pattern_parts(pattern) else {
        return (text.to_owned(), Vec::new());
    };

    let mut output = String::with_capacity(text.len());
    let mut cursor = 0usize;
    let mut protections = Vec::new();

    while cursor < text.len() {
        let Some((start, end, captures)) =
            find_next_pattern_match(text, cursor, &parts, case_sensitive)
        else {
            output.push_str(&text[cursor..]);
            return (output, protections);
        };

        if text[start..end].contains("ZXQGLOSSARY") {
            output.push_str(&text[cursor..end]);
            cursor = end;
            continue;
        }

        let placeholder_index = first_placeholder_index + protections.len();
        let placeholder = format!("ZXQGLOSSARY{placeholder_index}QXZ");
        output.push_str(&text[cursor..start]);
        let Some(translated_term) =
            render_pattern_replacement(replacement, &captures, NumberMode::Raw)
        else {
            output.push_str(&text[start..end]);
            cursor = end;
            continue;
        };
        output.push_str(&placeholder);
        protections.push(GlossaryProtection {
            placeholder,
            translated_term,
        });
        cursor = end;
    }

    (output, protections)
}

fn parse_pattern_parts(pattern: &str) -> Option<Vec<PatternPart>> {
    if pattern.is_empty() {
        return None;
    }

    let mut parts = Vec::new();
    let mut cursor = 0usize;
    let mut has_capture = false;

    while let Some((relative_index, part, token_len)) =
        find_next_pattern_capture_token(&pattern[cursor..])
    {
        let start = cursor + relative_index;
        if start > cursor {
            parts.push(PatternPart::Literal(pattern[cursor..start].to_owned()));
        }
        parts.push(part);
        has_capture = true;
        cursor = start + token_len;
    }

    if cursor < pattern.len() {
        parts.push(PatternPart::Literal(pattern[cursor..].to_owned()));
    }

    has_capture.then_some(parts)
}

fn find_next_pattern_capture_token(value: &str) -> Option<(usize, PatternPart, usize)> {
    let number_match = value
        .find("<number>")
        .map(|index| (index, PatternPart::Number, "<number>".len()));
    let string_match = value
        .find("<string>")
        .map(|index| (index, PatternPart::Text, "<string>".len()));

    match (number_match, string_match) {
        (Some(number), Some(string)) => {
            if number.0 <= string.0 {
                Some(number)
            } else {
                Some(string)
            }
        }
        (Some(number), None) => Some(number),
        (None, Some(string)) => Some(string),
        (None, None) => None,
    }
}

fn find_next_pattern_match(
    text: &str,
    cursor: usize,
    parts: &[PatternPart],
    case_sensitive: bool,
) -> Option<(usize, usize, Vec<PatternCapture>)> {
    for (start, _) in text[cursor..].char_indices() {
        let start = cursor + start;
        if let Some((end, captures)) = match_pattern_at(text, start, parts, case_sensitive) {
            return Some((start, end, captures));
        }
    }

    match_pattern_at(text, text.len(), parts, case_sensitive)
        .map(|(end, captures)| (text.len(), end, captures))
}

fn match_pattern_at(
    text: &str,
    start: usize,
    parts: &[PatternPart],
    case_sensitive: bool,
) -> Option<(usize, Vec<PatternCapture>)> {
    let mut cursor = start;
    let mut captures = Vec::new();

    for part in parts {
        match part {
            PatternPart::Literal(literal) => {
                if literal.is_empty() {
                    continue;
                }
                let remaining = text.get(cursor..)?;
                let matches_literal = if case_sensitive {
                    remaining.starts_with(literal)
                } else {
                    remaining
                        .get(..literal.len())
                        .is_some_and(|candidate| candidate.eq_ignore_ascii_case(literal))
                };
                if !matches_literal {
                    return None;
                }
                cursor += literal.len();
            }
            PatternPart::Number => {
                let remaining = text.get(cursor..)?;
                let number_len = remaining
                    .char_indices()
                    .take_while(|(_, character)| character.is_ascii_digit())
                    .last()
                    .map(|(index, character)| index + character.len_utf8())?;
                captures.push(PatternCapture::Number(remaining[..number_len].to_owned()));
                cursor += number_len;
            }
            PatternPart::Text => {
                let remaining = text.get(cursor..)?;
                let text_len = remaining
                    .char_indices()
                    .take_while(|(_, character)| is_pattern_text_capture_char(*character))
                    .last()
                    .map(|(index, character)| index + character.len_utf8())?;
                captures.push(PatternCapture::Text(remaining[..text_len].to_owned()));
                cursor += text_len;
            }
        }
    }

    Some((cursor, captures))
}

fn is_pattern_text_capture_char(character: char) -> bool {
    character.is_alphanumeric()
        || matches!(
            character,
            '_' | '-' | '\'' | '’' | '.' | '$' | '%' | '/' | '&'
        )
}

fn render_pattern_replacement(
    replacement: &str,
    captures: &[PatternCapture],
    number_mode: NumberMode,
) -> Option<String> {
    let mut output = String::with_capacity(replacement.len());
    let mut capture_index = 0usize;
    let mut cursor = 0usize;

    while let Some((relative_index, token, placeholder, token_len)) =
        find_next_replacement_capture_token(&replacement[cursor..])
    {
        let start = cursor + relative_index;
        output.push_str(&replacement[cursor..start]);
        if let Some(capture) = captures.get(capture_index) {
            output.push_str(&render_capture(capture, token, number_mode)?);
        } else {
            output.push_str(placeholder);
        }
        capture_index += 1;
        cursor = start + token_len;
    }

    output.push_str(&replacement[cursor..]);
    Some(output)
}

fn render_capture(
    capture: &PatternCapture,
    token: ReplacementCaptureToken,
    number_mode: NumberMode,
) -> Option<String> {
    match (capture, token) {
        (PatternCapture::Number(value), ReplacementCaptureToken::Number(render)) => {
            render_number_capture(value, number_mode, render)
        }
        (PatternCapture::Text(value), ReplacementCaptureToken::Text) => Some(value.clone()),
        (PatternCapture::Text(value), ReplacementCaptureToken::Number(_)) => Some(value.clone()),
        (PatternCapture::Number(value), ReplacementCaptureToken::Text) => Some(value.clone()),
    }
}

fn render_number_capture(
    value: &str,
    number_mode: NumberMode,
    render: NumberRender,
) -> Option<String> {
    match number_mode {
        NumberMode::Raw => Some(value.to_owned()),
        NumberMode::French => match render {
            NumberRender::Default => Some(spoken_number_capture(value)),
            NumberRender::Hour => spoken_hour_capture(value),
            NumberRender::Minute => spoken_minute_capture(value),
        },
    }
}

fn find_next_replacement_capture_token(
    value: &str,
) -> Option<(usize, ReplacementCaptureToken, &'static str, usize)> {
    let mut best = None;
    for candidate in [
        replacement_token_candidate(
            value,
            "<number:hour>",
            ReplacementCaptureToken::Number(NumberRender::Hour),
        ),
        replacement_token_candidate(
            value,
            "<number:minute>",
            ReplacementCaptureToken::Number(NumberRender::Minute),
        ),
        replacement_token_candidate(
            value,
            "<number>",
            ReplacementCaptureToken::Number(NumberRender::Default),
        ),
        replacement_token_candidate(value, "<string>", ReplacementCaptureToken::Text),
    ] {
        if candidate.is_some_and(|candidate| {
            best.is_none_or(
                |current: (usize, ReplacementCaptureToken, &'static str, usize)| {
                    candidate.0 < current.0
                },
            )
        }) {
            best = candidate;
        }
    }

    best
}

fn replacement_token_candidate(
    value: &str,
    placeholder: &'static str,
    token: ReplacementCaptureToken,
) -> Option<(usize, ReplacementCaptureToken, &'static str, usize)> {
    value
        .find(placeholder)
        .map(|index| (index, token, placeholder, placeholder.len()))
}

fn spoken_number_capture(value: &str) -> String {
    if value.len() > 1 && value.starts_with('0') {
        return spell_out_digits(value);
    }

    value
        .parse::<u32>()
        .ok()
        .and_then(french_number_word)
        .unwrap_or_else(|| spell_out_digits(value))
}

fn spoken_hour_capture(value: &str) -> Option<String> {
    let hour = value.parse::<u32>().ok()?;
    if hour > 23 {
        return None;
    }

    match hour {
        0 => Some("zéro heure".to_owned()),
        1 => Some("une heure".to_owned()),
        _ => Some(format!("{} heures", french_number_word(hour)?)),
    }
}

fn spoken_minute_capture(value: &str) -> Option<String> {
    let minute = value.parse::<u32>().ok()?;
    if !(1..=59).contains(&minute) {
        return None;
    }

    if minute == 1 {
        Some("une".to_owned())
    } else {
        french_number_word(minute)
    }
}

fn spell_out_digits(value: &str) -> String {
    value
        .chars()
        .filter_map(|character| character.to_digit(10).and_then(french_number_word))
        .collect::<Vec<_>>()
        .join(" ")
}

fn french_number_word(value: u32) -> Option<String> {
    let label = match value {
        0 => "zéro",
        1 => "un",
        2 => "deux",
        3 => "trois",
        4 => "quatre",
        5 => "cinq",
        6 => "six",
        7 => "sept",
        8 => "huit",
        9 => "neuf",
        10 => "dix",
        11 => "onze",
        12 => "douze",
        13 => "treize",
        14 => "quatorze",
        15 => "quinze",
        16 => "seize",
        17 => "dix sept",
        18 => "dix huit",
        19 => "dix neuf",
        20 => "vingt",
        30 => "trente",
        40 => "quarante",
        50 => "cinquante",
        _ => return french_compound_number_word(value),
    };
    Some(label.to_owned())
}

fn french_compound_number_word(value: u32) -> Option<String> {
    let tens = (value / 10) * 10;
    let unit = value % 10;
    if !(20..=59).contains(&value) || unit == 0 {
        return None;
    }

    let tens_label = french_number_word(tens)?;
    let unit_label = french_number_word(unit)?;
    if unit == 1 {
        Some(format!("{tens_label} et {unit_label}"))
    } else {
        Some(format!("{tens_label} {unit_label}"))
    }
}

/// Verifie les limites de mot quand le mode le demande.
fn accepts_match(text: &str, start: usize, end: usize, match_mode: &str) -> bool {
    if match_mode == MATCH_MODE_EXACT {
        return true;
    }

    if match_mode != MATCH_MODE_WORD_BOUNDARY {
        return false;
    }

    let before = text[..start].chars().next_back();
    let after = text[end..].chars().next();
    !is_word_char(before) && !is_word_char(after)
}

/// Classe les caracteres qui prolongent un mot ou un ticker.
fn is_word_char(value: Option<char>) -> bool {
    value
        .map(|character| character.is_alphanumeric() || character == '_' || character == '-')
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(source: &str, translated: &str, spoken: Option<&str>, priority: i64) -> GlossaryEntry {
        GlossaryEntry {
            id: priority,
            source_term: source.to_owned(),
            translated_term: translated.to_owned(),
            spoken_term: spoken.map(str::to_owned),
            source_language: "en".to_owned(),
            target_language: "fr".to_owned(),
            match_mode: MATCH_MODE_WORD_BOUNDARY.to_owned(),
            case_sensitive: false,
            priority,
            enabled: true,
            scope: GLOSSARY_SCOPE_FULL_PIPELINE.to_owned(),
            notes: None,
            created_at: "2026-06-02T00:00:00Z".to_owned(),
            updated_at: "2026-06-02T00:00:00Z".to_owned(),
        }
    }

    fn pattern_entry(
        source: &str,
        translated: &str,
        spoken: Option<&str>,
        priority: i64,
    ) -> GlossaryEntry {
        GlossaryEntry {
            match_mode: MATCH_MODE_PATTERN.to_owned(),
            ..entry(source, translated, spoken, priority)
        }
    }

    #[test]
    fn source_glossary_protects_then_restores_display_term() {
        let entries = vec![entry("ECB", "BCE", Some("Banque Centrale Europeenne"), 10)];
        let application = apply_source_glossary("ECB signals no rush", &entries);
        let translated =
            application.restore_display_text("ZXQGLOSSARY0QXZ ne signale aucune urgence", &entries);

        assert_eq!(application.hits, 1);
        assert_eq!(translated, "BCE ne signale aucune urgence");
    }

    #[test]
    fn spoken_only_glossary_does_not_pollute_display_pipeline() {
        let entries = vec![GlossaryEntry {
            scope: GLOSSARY_SCOPE_SPOKEN_ONLY.to_owned(),
            ..entry("ECB", "BCE", Some("Banque Centrale Europeenne"), 10)
        }];
        let application = apply_source_glossary("ECB signals no rush", &entries);
        let displayed = apply_display_glossary("ECB signals no rush", &entries);
        let (spoken, hits) = apply_spoken_glossary("BCE signale une pause", &entries);

        assert_eq!(application.hits, 0);
        assert_eq!(application.text, "ECB signals no rush");
        assert_eq!(displayed, "ECB signals no rush");
        assert_eq!(hits, 1);
        assert_eq!(spoken, "Banque Centrale Europeenne signale une pause");
    }

    #[test]
    fn source_glossary_display_targets_render_pattern_captures() {
        let entries = vec![pattern_entry(
            "ECB's <string>",
            "<string> de la BCE",
            None,
            10,
        )];
        let targets =
            source_glossary_display_targets("ECB's Nagel says all options remain open.", &entries);

        assert_eq!(targets, vec!["Nagel de la BCE"]);
    }

    #[test]
    fn spoken_glossary_keeps_display_text_out_of_visual_fields() {
        let entries = vec![entry("ECB", "BCE", Some("Banque Centrale Europeenne"), 10)];
        let (spoken, hits) = apply_spoken_glossary("14:30. BCE attend.", &entries);

        assert_eq!(hits, 1);
        assert_eq!(spoken, "14:30. Banque Centrale Europeenne attend.");
    }

    #[test]
    fn pattern_glossary_expands_time_numbers_for_tts() {
        let entries = vec![pattern_entry(
            "<number>h<number>",
            "<number>h<number>",
            Some("<number> heure <number>"),
            10,
        )];
        let (spoken, hits) = apply_spoken_glossary("Decision attendue a 16h01.", &entries);

        assert_eq!(hits, 1);
        assert_eq!(spoken, "Decision attendue a seize heure zéro un.");
    }

    #[test]
    fn pattern_glossary_spells_number_groups_with_leading_zero() {
        let entries = vec![pattern_entry(
            "<number>h<number>",
            "<number>h<number>",
            Some("<number> heure <number>"),
            10,
        )];
        let (spoken, hits) = apply_spoken_glossary("Ouverture a 09h30.", &entries);

        assert_eq!(hits, 1);
        assert_eq!(spoken, "Ouverture a zéro neuf heure trente.");
    }

    #[test]
    fn pattern_glossary_time_modifiers_render_natural_spoken_hours() {
        let entries = vec![
            pattern_entry("<number>h00", "<number>h00", Some("<number:hour>"), 20),
            pattern_entry(
                "<number>h<number>",
                "<number>h<number>",
                Some("<number:hour> <number:minute>"),
                10,
            ),
        ];
        let (spoken, hits) =
            apply_spoken_glossary("12h01, 14h00, 09h30, 01h01, 00h00, 29h30, 10h88.", &entries);

        assert_eq!(hits, 5);
        assert_eq!(
            spoken,
            "douze heures une, quatorze heures, neuf heures trente, une heure une, zéro heure, 29h30, 10h88."
        );
    }

    #[test]
    fn pattern_glossary_can_preserve_display_text() {
        let entries = vec![pattern_entry(
            "<number>h<number>",
            "<number>h<number>",
            Some("<number> heure <number>"),
            10,
        )];

        assert_eq!(
            apply_display_glossary("Briefing 16h01.", &entries),
            "Briefing 16h01."
        );
    }

    #[test]
    fn pattern_glossary_captures_source_string_before_translation() {
        let entries = vec![pattern_entry(
            "ECB's <string>",
            "<string> de la BCE",
            None,
            10,
        )];
        let application = apply_source_glossary("ECB's Nagel says rates stay high.", &entries);
        let translated =
            application.restore_display_text("ZXQGLOSSARY0QXZ indique un statu quo.", &entries);

        assert_eq!(application.hits, 1);
        assert_eq!(application.text, "ZXQGLOSSARY0QXZ says rates stay high.");
        assert_eq!(translated, "Nagel de la BCE indique un statu quo.");
    }

    #[test]
    fn pattern_glossary_text_capture_leaves_trailing_colon_to_literal() {
        let entries = vec![pattern_entry("<string>:", "<string> :", None, 100)];

        assert_eq!(
            apply_display_glossary("NASDAQ: SpaceX remains pre-launch.", &entries),
            "NASDAQ : SpaceX remains pre-launch."
        );
    }

    #[test]
    fn pattern_glossary_combines_ecb_possessive_and_colon_spacing() {
        let entries = vec![
            pattern_entry("<string>:", "<string> :", None, 100),
            pattern_entry("ECB's <string>", "<string> de la BCE", None, 10),
        ];
        let application = apply_source_glossary("ECB's Nagel: all options remain open.", &entries);
        let translated = application.restore_display_text(
            "ZXQGLOSSARY0QXZ: toutes les options restent ouvertes.",
            &entries,
        );

        assert_eq!(application.hits, 2);
        assert_eq!(
            application.text,
            "ZXQGLOSSARY0QXZ: all options remain open."
        );
        assert_eq!(
            translated,
            "Nagel de la BCE : toutes les options restent ouvertes."
        );
    }

    #[test]
    fn pattern_glossary_rewrites_display_string_when_provider_keeps_source() {
        let entries = vec![pattern_entry(
            "ECB's <string>",
            "<string> de la BCE",
            None,
            10,
        )];

        assert_eq!(
            apply_display_glossary("ECB's Nagel juge la situation stable.", &entries),
            "Nagel de la BCE juge la situation stable."
        );
    }
}
