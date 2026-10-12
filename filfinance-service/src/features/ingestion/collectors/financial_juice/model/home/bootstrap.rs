//! Donnees bootstrap authentifiees de FinancialJuice.

/// Variables utiles extraites de `/home` apres authentification.
#[derive(Debug, Clone)]
pub(in crate::features::ingestion::collectors::financial_juice) struct FinancialJuiceBootstrap {
    /// Base `live.financialjuice.com` utilisee par `FJService` et SignalR.
    pub(in crate::features::ingestion::collectors::financial_juice) main_url: String,
    /// Jeton opaque exige par `GetPreviousNews`.
    pub(in crate::features::ingestion::collectors::financial_juice) info: String,
    /// JWT/jeton de feed exige par le live SignalR.
    pub(in crate::features::ingestion::collectors::financial_juice) feedtoken: String,
    /// Liste des IDs de feeds autorises pour ce compte.
    pub(in crate::features::ingestion::collectors::financial_juice) feed_ids: Vec<i64>,
}
