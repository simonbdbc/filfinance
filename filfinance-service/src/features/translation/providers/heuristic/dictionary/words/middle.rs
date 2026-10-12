//! Traductions heuristiques des mots de milieu d'alphabet.

/// Traduit les mots de `gains` a `pricing`.
pub(super) fn translate_middle_word(core: &str) -> Option<&'static str> {
    match core {
        "gains" => Some("progresse"),
        "gain" => Some("progression"),
        "gold" => Some("or"),
        "growth" => Some("croissance"),
        "high" => Some("eleve"),
        "higher" => Some("plus haut"),
        "hold" => Some("maintien"),
        "holds" => Some("maintient"),
        "inflation" => Some("inflation"),
        "job" => Some("emploi"),
        "jobs" => Some("emplois"),
        "jumps" => Some("bondit"),
        "keeps" => Some("garde"),
        "lower" => Some("plus bas"),
        "loss" => Some("perte"),
        "losses" => Some("pertes"),
        "market" => Some("marche"),
        "markets" => Some("marches"),
        "misses" => Some("manque"),
        "oil" => Some("petrole"),
        "outlook" => Some("perspective"),
        "policy" => Some("politique"),
        "powell" => Some("Powell"),
        "prices" => Some("prix"),
        "pricing" => Some("valorisation"),
        _ => None,
    }
}
