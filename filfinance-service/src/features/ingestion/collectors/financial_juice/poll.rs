//! Polling et backfill news du collecteur FinancialJuice.

mod backlog;
mod page;
mod worker;

// Le module parent conserve le point d'entree historique du collecteur site et ses handles prives.
pub(crate) use worker::{FinancialJuiceWorkerHandles, spawn_site};
