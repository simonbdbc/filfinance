//! Nettoyage texte des payloads FinancialJuice.

// `Html` transforme les fragments source en texte lisible sans regex fragile.
use scraper::Html;

/// Préfixe de marque encore présent dans certains titres FinancialJuice.
const FINANCIAL_JUICE_PREFIX: &str = "FinancialJuice:";

/// Nettoie un titre FJ en retirant le préfixe de marque et les espaces superflus.
pub(in crate::features::ingestion::collectors::financial_juice) fn normalize_title(
    raw_title: &str,
) -> String {
    let trimmed = collapse_whitespace(&decode_xml_entities(raw_title));

    if let Some(stripped) = trimmed.strip_prefix(FINANCIAL_JUICE_PREFIX) {
        return collapse_whitespace(stripped);
    }

    trimmed
}

/// Dé-HTMLise et compacte une description source.
pub(in crate::features::ingestion::collectors::financial_juice) fn normalize_description(
    raw_description: &str,
) -> Option<String> {
    let html = decode_xml_entities(raw_description.trim());

    if html.is_empty() {
        return None;
    }

    let text = html_fragment_to_text(&html);
    non_empty(&text)
}

/// Supprime le HTML très simple d'un fragment FinancialJuice.
fn html_fragment_to_text(fragment: &str) -> String {
    let document = Html::parse_fragment(fragment);
    let text = document.root_element().text().collect::<Vec<_>>().join(" ");
    collapse_whitespace(&text)
}

/// Décode quelques entités XML/HTML courantes présentes dans les payloads FJ.
pub(super) fn decode_xml_entities(value: &str) -> String {
    value
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
}

/// Compacte les espaces et retours à la ligne d'une chaîne source.
pub(in crate::features::ingestion::collectors::financial_juice) fn collapse_whitespace(
    value: &str,
) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Convertit une chaîne potentiellement vide en `Option<String>`.
pub(in crate::features::ingestion::collectors::financial_juice) fn non_empty(
    value: &str,
) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_owned())
    }
}
