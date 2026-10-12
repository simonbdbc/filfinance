//! Metadonnees macroeconomiques optionnelles d'un evenement news.

// Serialize et Deserialize gardent la projection JSON identique a l'ancien modele.
use serde::{Deserialize, Serialize};

/// Metadonnees macro optionnelles pour les sources de type calendrier economique.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MacroMetadata {
    /// Niveau d'importance de l'evenement macro.
    pub importance: Option<String>,
    /// Pays concerne si la source le fournit.
    pub country_code: Option<String>,
    /// Devise de reference si la source la fournit.
    pub currency: Option<String>,
    /// Valeur publiee effectivement.
    pub actual_value: Option<String>,
    /// Valeur attendue / consensus.
    pub forecast_value: Option<String>,
    /// Valeur precedente.
    pub previous_value: Option<String>,
}
