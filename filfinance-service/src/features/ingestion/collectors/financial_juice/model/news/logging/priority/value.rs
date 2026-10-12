//! Interpretation des valeurs de priorite inconnues FinancialJuice.

use serde_json::Value;

/// Interprete une valeur inconnue comme signal actif uniquement si elle porte un vrai "oui".
pub(in crate::features::ingestion) fn is_priority_like_unknown_value_active(value: &Value) -> bool {
    match value {
        // `Breaking: false` est frequent dans le backlog et ne doit pas remonter en info production.
        Value::Bool(flag) => *flag,
        // Un entier ou flottant non nul peut representer une priorite numerique cote source.
        Value::Number(number) => number.as_f64().is_some_and(|value| value != 0.0),
        // Les chaines vides ou explicitement negatives sont des metadonnees, pas des alertes.
        Value::String(text) => {
            let normalized = text.trim().to_ascii_lowercase();
            !normalized.is_empty()
                && !matches!(
                    normalized.as_str(),
                    "0" | "false" | "no" | "none" | "null" | "normal"
                )
        }
        // Les objets/listes restent visibles en debug complet mais ne sont pas un signal exploitable ici.
        Value::Array(_) | Value::Object(_) | Value::Null => false,
    }
}
