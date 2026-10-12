//! Modele canonique utilise par le pipeline d'ingestion news.

// Serialize et Deserialize permettent aux collecteurs et tests de manipuler le modele en JSON.
use serde::{Deserialize, Serialize};

use crate::shared::time::now_rfc3339;

use super::{EventKind, MacroMetadata};
use crate::features::news::domain::signature::signature_for;

/// Modele metier canonique manipule par le pipeline d'ingestion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewsEvent {
    /// Nom logique de la source d'origine.
    pub source: String,
    /// Famille metier de l'evenement.
    pub event_kind: EventKind,
    /// Titre canonique de la news.
    pub title: String,
    /// URL source de l'evenement.
    pub url: String,
    /// Date de publication source quand elle existe.
    pub published_at: Option<String>,
    /// Heure evenementielle plus precise quand la source la distingue.
    pub event_time: Option<String>,
    /// Horodatage d'ingestion locale dans le service.
    pub ingested_at: String,
    /// Corps de texte optionnel.
    pub body: Option<String>,
    /// Niveau brut de news fourni par la source, par exemple `active active-critical` chez FinancialJuice.
    pub news_level: Option<String>,
    /// Reference brute utile au debug et aux runs de test synthetiques.
    pub raw_payload_ref: Option<String>,
    /// Metadata macro optionnelles pour les evenements economiques.
    pub macro_metadata: Option<MacroMetadata>,
    /// Signature stable utilisee pour la dedup durable.
    pub signature: String,
}

impl NewsEvent {
    #[allow(dead_code)]
    /// Construit une headline simple en remplissant les champs canoniques par defaut.
    pub fn new_headline(
        // `impl Into<String>` rend l'appel souple: `&str` et `String` sont acceptes.
        source: impl Into<String>,
        title: impl Into<String>,
        url: impl Into<String>,
        published_at: Option<String>,
        body: Option<String>,
    ) -> Self {
        let source = source.into();
        // La signature est calculee apres construction pour centraliser la logique.
        let mut event = Self {
            source,
            event_kind: EventKind::Headline,
            title: title.into(),
            url: url.into(),
            published_at,
            event_time: None,
            ingested_at: now_rfc3339(),
            body,
            // Les sources qui ne publient pas encore de niveau gardent le contrat historique.
            news_level: None,
            raw_payload_ref: None,
            macro_metadata: None,
            signature: String::new(),
        };

        event.recompute_signature();
        event
    }

    /// Recalcule la signature quand un champ determinant a ete modifie.
    pub fn recompute_signature(&mut self) {
        self.signature = signature_for(self);
    }
}
