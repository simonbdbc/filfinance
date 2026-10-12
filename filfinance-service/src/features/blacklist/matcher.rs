//! Matching operateur blacklist applique aux textes source anglais.

use crate::{
    features::{
        blacklist::domain::{
            BLACKLIST_ACTION_REJECT_NEWS, BLACKLIST_ACTION_STRIP_CONTENT,
            BLACKLIST_MATCH_MODE_EXACT, BLACKLIST_MATCH_MODE_PARTIAL,
            BLACKLIST_TARGET_SCOPE_CONTENT, BLACKLIST_TARGET_SCOPE_TITLE,
            BLACKLIST_TARGET_SCOPE_TITLE_CONTENT, NewNewsBlacklistHit, NewsBlacklistEntry,
        },
        news::domain::NewsEvent,
    },
    shared::text::{normalize_whitespace, trim_chars},
};

/// Resultat de matching d'une news entrante.
#[derive(Debug, Clone)]
pub(crate) struct NewsBlacklistMatch {
    /// Regle touchee.
    pub entry: NewsBlacklistEntry,
    /// Ligne d'audit prete a inserer.
    pub hit: NewNewsBlacklistHit,
}

/// Cherche la premiere regle active touchant le titre ou contenu source.
pub(crate) fn find_news_blacklist_match(
    event: &NewsEvent,
    entries: &[NewsBlacklistEntry],
) -> Option<NewsBlacklistMatch> {
    for entry in entries {
        if !entry.enabled {
            continue;
        }

        match entry.target_scope.as_str() {
            BLACKLIST_TARGET_SCOPE_TITLE => {
                if let Some(matched_text) = matches_field(&event.title, entry) {
                    return Some(build_match(
                        event,
                        entry,
                        "title",
                        matched_text,
                        BLACKLIST_ACTION_REJECT_NEWS,
                    ));
                }
            }
            BLACKLIST_TARGET_SCOPE_CONTENT => {
                if let Some(body) = event.body.as_deref() {
                    if let Some(matched_text) = matches_field(body, entry) {
                        return Some(build_match(
                            event,
                            entry,
                            "body",
                            matched_text,
                            BLACKLIST_ACTION_STRIP_CONTENT,
                        ));
                    }
                }
            }
            BLACKLIST_TARGET_SCOPE_TITLE_CONTENT => {
                if let Some(matched_text) = matches_field(&event.title, entry) {
                    return Some(build_match(
                        event,
                        entry,
                        "title",
                        matched_text,
                        BLACKLIST_ACTION_REJECT_NEWS,
                    ));
                }

                if let Some(body) = event.body.as_deref() {
                    if let Some(matched_text) = matches_field(body, entry) {
                        return Some(build_match(
                            event,
                            entry,
                            "body",
                            matched_text,
                            BLACKLIST_ACTION_REJECT_NEWS,
                        ));
                    }
                }
            }
            _ => {
                if let Some(matched_text) = matches_field(&event.title, entry) {
                    return Some(build_match(
                        event,
                        entry,
                        "title",
                        matched_text,
                        BLACKLIST_ACTION_REJECT_NEWS,
                    ));
                }

                if let Some(body) = event.body.as_deref() {
                    if let Some(matched_text) = matches_field(body, entry) {
                        return Some(build_match(
                            event,
                            entry,
                            "body",
                            matched_text,
                            BLACKLIST_ACTION_REJECT_NEWS,
                        ));
                    }
                }
            }
        }
    }

    None
}

fn build_match(
    event: &NewsEvent,
    entry: &NewsBlacklistEntry,
    matched_field: &str,
    matched_text: String,
    action: &str,
) -> NewsBlacklistMatch {
    NewsBlacklistMatch {
        entry: entry.clone(),
        hit: NewNewsBlacklistHit {
            blacklist_entry_id: entry.id,
            term: entry.term.clone(),
            match_mode: entry.match_mode.clone(),
            target_scope: entry.target_scope.clone(),
            action: action.to_owned(),
            source: event.source.clone(),
            event_kind: event.event_kind.as_str().to_owned(),
            matched_field: matched_field.to_owned(),
            matched_text,
            source_title: trim_chars(&event.title, 500),
            source_body_excerpt: event.body.as_deref().map(|body| trim_chars(body, 500)),
            raw_payload_ref: event.raw_payload_ref.clone(),
            signature: event.signature.clone(),
        },
    }
}

fn matches_field(value: &str, entry: &NewsBlacklistEntry) -> Option<String> {
    let normalized_value = normalize_for_match(value);
    let normalized_term = normalize_for_match(&entry.term);

    if normalized_term.is_empty() {
        return None;
    }

    match entry.match_mode.as_str() {
        BLACKLIST_MATCH_MODE_PARTIAL => normalized_value
            .contains(&normalized_term)
            .then(|| trim_chars(value, 500)),
        BLACKLIST_MATCH_MODE_EXACT => {
            exact_phrase_match(&normalized_value, &normalized_term).then(|| trim_chars(value, 500))
        }
        _ => None,
    }
}

fn normalize_for_match(value: &str) -> String {
    normalize_whitespace(value).to_lowercase()
}

fn exact_phrase_match(value: &str, term: &str) -> bool {
    let mut start = 0usize;
    while let Some(relative_index) = value[start..].find(term) {
        let index = start + relative_index;
        let before = value[..index].chars().next_back();
        let after = value[index + term.len()..].chars().next();

        if is_boundary(before) && is_boundary(after) {
            return true;
        }

        start = index + term.len();
    }

    false
}

fn is_boundary(character: Option<char>) -> bool {
    character.is_none_or(|character| !character.is_alphanumeric())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::blacklist::domain::NewsBlacklistEntry;

    fn entry(term: &str, match_mode: &str) -> NewsBlacklistEntry {
        NewsBlacklistEntry {
            id: 1,
            term: term.to_owned(),
            match_mode: match_mode.to_owned(),
            target_scope: BLACKLIST_TARGET_SCOPE_TITLE_CONTENT.to_owned(),
            enabled: true,
            notes: None,
            created_at: "2026-06-10T00:00:00Z".to_owned(),
            updated_at: "2026-06-10T00:00:00Z".to_owned(),
        }
    }

    #[test]
    fn partial_match_is_case_insensitive_substring() {
        let entry = entry("AI rebound", BLACKLIST_MATCH_MODE_PARTIAL);

        assert!(matches_field("As day two of the ai rebound begins", &entry).is_some());
    }

    #[test]
    fn exact_match_uses_word_boundaries() {
        let entry = entry("Fed", BLACKLIST_MATCH_MODE_EXACT);

        assert!(matches_field("Fed signals new stance", &entry).is_some());
        assert!(matches_field("The Fed, again", &entry).is_some());
        assert!(matches_field("FedEx rises", &entry).is_none());
    }

    #[test]
    fn title_scope_ignores_body_match() {
        let mut entry = entry("blocked phrase", BLACKLIST_MATCH_MODE_PARTIAL);
        entry.target_scope = BLACKLIST_TARGET_SCOPE_TITLE.to_owned();
        let event = NewsEvent::new_headline(
            "financial_juice",
            "Market update",
            "https://example.test/news",
            Some("2026-06-10T10:00:00Z".to_owned()),
            Some("Detail with blocked phrase".to_owned()),
        );

        assert!(find_news_blacklist_match(&event, &[entry]).is_none());
    }

    #[test]
    fn content_scope_strips_content_and_ignores_title_match() {
        let mut title_entry = entry("Market update", BLACKLIST_MATCH_MODE_PARTIAL);
        title_entry.target_scope = BLACKLIST_TARGET_SCOPE_CONTENT.to_owned();
        let event = NewsEvent::new_headline(
            "financial_juice",
            "Market update",
            "https://example.test/news",
            Some("2026-06-10T10:00:00Z".to_owned()),
            Some("Detail with blocked phrase".to_owned()),
        );

        assert!(find_news_blacklist_match(&event, &[title_entry]).is_none());

        let mut body_entry = entry("blocked phrase", BLACKLIST_MATCH_MODE_PARTIAL);
        body_entry.target_scope = BLACKLIST_TARGET_SCOPE_CONTENT.to_owned();
        let matched = find_news_blacklist_match(&event, &[body_entry])
            .expect("body should match content scope");

        assert_eq!(matched.hit.matched_field, "body");
        assert_eq!(matched.hit.action, BLACKLIST_ACTION_STRIP_CONTENT);
        assert_eq!(matched.hit.target_scope, BLACKLIST_TARGET_SCOPE_CONTENT);
    }

    #[test]
    fn title_content_scope_rejects_body_match() {
        let event = NewsEvent::new_headline(
            "financial_juice",
            "Market update",
            "https://example.test/news",
            Some("2026-06-10T10:00:00Z".to_owned()),
            Some("Detail with blocked phrase".to_owned()),
        );
        let matched = find_news_blacklist_match(
            &event,
            &[entry("blocked phrase", BLACKLIST_MATCH_MODE_PARTIAL)],
        )
        .expect("body should match title_content scope");

        assert_eq!(matched.hit.matched_field, "body");
        assert_eq!(matched.hit.action, BLACKLIST_ACTION_REJECT_NEWS);
        assert_eq!(
            matched.hit.target_scope,
            BLACKLIST_TARGET_SCOPE_TITLE_CONTENT
        );
    }
}
