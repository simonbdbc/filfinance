//! Facade des collecteurs d'ingestion.

// Les sous-modules concrétisent les sources aujourd'hui conservées dans le service.
pub(crate) mod financial_juice;
pub(crate) mod x;

// `runner` porte le cycle de vie commun des collecteurs.
mod runner;
// `startup` porte seulement les tests historiques du filtrage J/J-1.
#[cfg(test)]
mod startup;

// Les collecteurs concrets conservent leurs imports transverses via cette façade.
pub(crate) use runner::{emit_items_to_pipeline, spawn};

// Les tests d'ingestion ciblent explicitement les helpers transverses sans les exposer en prod.
#[cfg(test)]
pub(in crate::features::ingestion) use startup::{
    event_business_date, filter_startup_backfill_items, parse_rfc3339_business_date,
};
#[cfg(test)]
pub(in crate::features::ingestion) use x::retry_after_delay_ms;
