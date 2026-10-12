//! Protection des tokens finance avant passage dans les providers de traduction.

use crate::shared::text::{cleanup_spacing, normalize_whitespace, split_token_parts};

/// Variante protegee d'un texte dans laquelle les tokens finance sont remplaces.
#[derive(Debug, Clone)]
pub(crate) struct ProtectedText {
    pub(crate) masked_text: String,
    pub(crate) placeholders: Vec<(String, String)>,
}

impl ProtectedText {
    /// Restaure les tokens proteges apres traduction.
    pub(crate) fn restore(&self, translated_text: &str) -> String {
        normalize_whitespace(&cleanup_spacing(&self.restore_exact(translated_text)))
    }

    /// Restaure les tokens d'un provider externe et rejette les placeholders deformes.
    pub(crate) fn restore_provider_output(&self, translated_text: &str) -> Result<String, String> {
        let restored = self.restore_exact(translated_text);
        if contains_placeholder_marker(&restored) {
            return Err(
                "translation provider returned a mangled protected finance placeholder".to_owned(),
            );
        }

        Ok(normalize_whitespace(&cleanup_spacing(&restored)))
    }

    /// Applique uniquement les remplacements exacts, sans normaliser ni valider.
    fn restore_exact(&self, translated_text: &str) -> String {
        // `fold` accumule la chaine restauree en reappliquant chaque placeholder dans l'ordre.
        self.placeholders.iter().fold(
            translated_text.to_owned(),
            |acc, (placeholder, original)| acc.replace(placeholder, original),
        )
    }
}

/// Remplace les tokens sensibles finance par des placeholders stables.
pub(crate) fn protect_finance_tokens(text: &str) -> ProtectedText {
    let mut placeholders = Vec::new();

    let masked_text = normalize_whitespace(text)
        .split_whitespace()
        .map(|token| {
            let (prefix, core, suffix) = split_token_parts(token);
            if core.is_empty() || !should_preserve_token(core) {
                return token.to_owned();
            }

            // Le placeholder volontairement bizarre reduit le risque de collision avec un vrai mot de marche.
            let placeholder = format!("ZXQTOKEN{}QXZ", placeholders.len());
            placeholders.push((placeholder.clone(), core.to_owned()));
            format!("{prefix}{placeholder}{suffix}")
        })
        .collect::<Vec<_>>()
        .join(" ");

    ProtectedText {
        masked_text,
        placeholders,
    }
}

/// Decide si un token doit etre preserve tel quel pendant la traduction.
fn should_preserve_token(core: &str) -> bool {
    if !core.is_ascii() {
        return false;
    }

    // On mesure separement "contient des lettres" et "toutes ces lettres sont majuscules"
    // pour reconnaitre les tickers courts sans sur-proteger n'importe quel mot.
    let alpha_len = core.chars().filter(|ch| ch.is_ascii_alphabetic()).count();
    let upper_len = core.chars().filter(|ch| ch.is_ascii_uppercase()).count();

    core.chars().any(|ch| ch.is_ascii_digit())
        || core.contains('%')
        || core.contains('$')
        || core.contains('/')
        || core.contains(':')
        || core.contains('=')
        || core.contains('+')
        || (alpha_len >= 2 && upper_len == alpha_len && alpha_len <= 8)
}

fn contains_placeholder_marker(text: &str) -> bool {
    text.contains("ZXQTOKEN")
}
