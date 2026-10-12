//! Facade du parsing bootstrap `/home` FinancialJuice.

mod bootstrap;
mod extraction;
mod parser;

// Le module parent conserve le chemin historique des donnees de session authentifiees.
pub(in crate::features::ingestion::collectors::financial_juice) use bootstrap::FinancialJuiceBootstrap;
pub(in crate::features::ingestion) use parser::{
    FinancialJuiceHomePageState, parse_home_page_state,
};

// Ce helper reste expose uniquement aux tests de regression du parsing FinancialJuice.
#[cfg(test)]
pub(in crate::features::ingestion) use extraction::extract_js_number_list;
