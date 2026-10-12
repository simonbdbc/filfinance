//! Construction du préfixe canonique de narration news.

use super::super::event::PersistedNewsEvent;
use super::clock::format_local_clock_label;

/// Construit le prefixe de narration unique, responsable a la fois de l'heure et du libelle metier.
pub fn build_news_narration_prefix(item: &PersistedNewsEvent) -> String {
    // Le timestamp et le niveau source restent les deux seules entrees du prefixe metier.
    build_news_narration_prefix_from_timestamp(
        best_narration_timestamp(item).unwrap_or_default(),
        item.news_level.as_deref(),
    )
}

/// Construit le prefixe de narration depuis un timestamp live et le niveau source.
pub fn build_news_narration_prefix_from_timestamp(
    timestamp: &str,
    news_level: Option<&str>,
) -> String {
    // Le label horaire est toujours calcule au meme endroit pour eviter un drift entre traduction, WS et TTS.
    let clock_label = format_local_clock_label(timestamp).unwrap_or_else(|| "00:00".to_owned());

    // Le helper par parties concentre le choix d'ordre des marqueurs: `HH:MM. Nouvelle information`.
    build_news_narration_prefix_from_parts(&clock_label, news_level)
}

/// Construit le prefixe final a partir des deux marqueurs elementaires.
fn build_news_narration_prefix_from_parts(clock_label: &str, news_level: Option<&str>) -> String {
    if news_level_contains_active_word(news_level) {
        // Le libelle actif vient apres l'heure pour produire `HH:MM. Information importante. texte`.
        format!("{clock_label}. Information importante")
    } else {
        // Sans niveau actif, le live annonce explicitement une news standard avant le titre.
        format!("{clock_label}. Nouvelle information")
    }
}

/// Construit le marqueur horaire `HH:MM` utilise dans la narration utilisateur.
pub(super) fn build_news_clock_label(item: &PersistedNewsEvent) -> String {
    // La chaine `Option -> and_then -> unwrap_or_else` garde la logique compacte sans perdre le
    // contrat metier: meilleure date disponible, puis conversion locale, puis fallback stable.
    best_narration_timestamp(item)
        .and_then(format_local_clock_label)
        // Si un timestamp inattendu fuit jusque-la, on garde un format stable plutot qu'un libelle metier stale.
        .unwrap_or_else(|| "00:00".to_owned())
}

/// Choisit la meilleure date metier disponible pour construire la narration.
fn best_narration_timestamp(item: &PersistedNewsEvent) -> Option<&str> {
    item.published_at
        .as_deref()
        .or(item.event_time.as_deref())
        .or(Some(item.ingested_at.as_str()))
}

/// Detecte le mot `active` dans le niveau source sans confondre avec une sous-chaine comme `inactive`.
pub fn news_level_contains_active_word(news_level: Option<&str>) -> bool {
    news_level
        // `map(...)` evite d'allouer une chaine quand le niveau est absent.
        .map(|level| {
            level
                // On coupe sur les separateurs non alphanumeriques pour gerer `active-critical`.
                .split(|character: char| !character.is_ascii_alphanumeric())
                // `eq_ignore_ascii_case` accepte `Active` sans changer la valeur brute conservee ailleurs.
                .any(|token| token.eq_ignore_ascii_case("active"))
        })
        // Niveau absent: aucune importance explicite ne doit etre inventee.
        .unwrap_or(false)
}

/// Normalise l'importance FinancialJuice a partir du niveau source brut.
pub fn financial_juice_importance_from_news_level(news_level: Option<&str>) -> &'static str {
    match news_level.map(str::trim).filter(|level| !level.is_empty()) {
        None => "low",
        Some(level) if news_level_contains_active_word(Some(level)) => "high",
        Some(_) => "medium",
    }
}
