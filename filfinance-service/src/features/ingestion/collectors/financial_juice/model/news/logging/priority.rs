//! Observation des champs inconnus ressemblant a une priorite FinancialJuice.

mod key;
mod value;

// Les logs structurés documentent les champs de priorité encore non contractualisés.
use tracing::{debug, info};

use key::is_priority_like_unknown_key;
pub(in crate::features::ingestion) use value::is_priority_like_unknown_value_active;

use super::super::item::FinancialJuiceApiNewsItem;

/// Logue les champs inconnus qui ressemblent à un niveau de priorité ou de criticité.
pub(in crate::features::ingestion::collectors::financial_juice::model::news) fn log_priority_like_unknown_fields(
    source_name: &str,
    transport_name: &str,
    item: &FinancialJuiceApiNewsItem,
) {
    // Si aucun champ inconnu n'existe, `serde(flatten)` confirme que le contrat courant est couvert.
    if item.extra.is_empty() {
        return;
    }

    // Ce log debug donne le contrat complet observé quand on lance le service avec `RUST_LOG=debug`.
    debug!(
        source = %source_name,
        transport = %transport_name,
        news_id = item.news_id,
        unmapped_keys = ?item.extra.keys().collect::<Vec<_>>(),
        // Les valeurs sont loguées en `debug` uniquement pour inspecter le contrat source sans bruit en production.
        unmapped_values = ?item.extra,
        "financial juice news item exposes unmapped fields"
    );

    // On garde seulement les clés qui ressemblent à un signal exploitable pour ne pas spammer le live.
    let matching_fields: Vec<_> = item
        // `extra` ne contient que les champs JSON que le DTO ne mappe pas explicitement.
        .extra
        // `.iter()` emprunte les paires clé/valeur sans consommer le DTO, car on logue seulement.
        .iter()
        // `filter` conserve les champs dont la clé ressemble à une criticité et dont la valeur est active.
        .filter(|(key, value)| {
            is_priority_like_unknown_key(key) && is_priority_like_unknown_value_active(value)
        })
        // `collect()` matérialise les références retenues pour les réutiliser dans le log structuré.
        .collect();

    // Les autres champs inconnus restent ignorés pour l'instant: l'objectif est de trouver `critical`.
    if matching_fields.is_empty() {
        return;
    }

    // `info!` rend le signal visible avec le niveau de log par défaut du service.
    info!(
        source = %source_name,
        transport = %transport_name,
        news_id = item.news_id,
        iid = ?item.iid,
        title = %item.title,
        candidate_fields = ?matching_fields,
        "financial juice news item exposes priority-like unmapped fields"
    );
}
