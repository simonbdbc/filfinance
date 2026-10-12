//! Fallback tolerant pour les payloads `GetPreviousNews` partiellement corrompus.

use super::super::item::FinancialJuiceApiNewsItem;

/// Extrait les objets d'un tableau JSON FJ et ignore uniquement ceux que `serde_json` refuse.
pub(super) fn parse_financial_juice_news_items_lenient(
    payload: &str,
) -> (Vec<FinancialJuiceApiNewsItem>, usize) {
    // Ces compteurs forment un petit scanner JSON suffisant pour isoler les objets top-level.
    let mut depth = 0_usize;
    let mut is_in_string = false;
    let mut is_escaped = false;
    let mut object_start = None;
    let mut items = Vec::new();
    let mut skipped_count = 0_usize;

    for (index, character) in payload.char_indices() {
        // Dans une chaîne JSON, seules l'échappement et la fermeture de chaîne changent l'état.
        if is_in_string {
            if is_escaped {
                is_escaped = false;
            } else if character == '\\' {
                is_escaped = true;
            } else if character == '"' {
                is_in_string = false;
            }
            continue;
        }

        match character {
            '"' => {
                is_in_string = true;
            }
            '{' => {
                // Un objet top-level commence au passage profondeur 0 -> 1.
                if depth == 0 {
                    object_start = Some(index);
                }
                depth += 1;
            }
            '}' => {
                if depth == 0 {
                    continue;
                }

                // La fermeture profondeur 1 -> 0 borne un objet complet candidat.
                if depth == 1
                    && let Some(start) = object_start.take()
                {
                    let end = index + character.len_utf8();
                    match serde_json::from_str::<FinancialJuiceApiNewsItem>(&payload[start..end]) {
                        Ok(item) => items.push(item),
                        Err(_) => skipped_count += 1,
                    }
                }

                depth -= 1;
            }
            _ => {}
        }
    }

    (items, skipped_count)
}
