//! Contrat JSON partage par les providers locaux one-shot et session.

use serde::{Deserialize, Serialize};

/// Payload JSON envoye sur `stdin` a une commande locale de traduction.
#[derive(Debug, Serialize)]
pub(in crate::features::translation::providers) struct LocalCommandPayload<'a> {
    // Ici on emprunte en `&str` pour serialiser sans recopier tout le texte une seconde fois.
    pub(in crate::features::translation::providers) source_language: &'a str,
    pub(in crate::features::translation::providers) target_language: &'a str,
    pub(in crate::features::translation::providers) title: &'a str,
    pub(in crate::features::translation::providers) body: Option<&'a str>,
}

/// Reponse JSON attendue sur `stdout` en retour d'une commande locale.
#[derive(Debug, Deserialize)]
pub(in crate::features::translation::providers) struct LocalCommandResponse {
    pub(in crate::features::translation::providers) translated_title: Option<String>,
    pub(in crate::features::translation::providers) translated_body: Option<String>,
    pub(in crate::features::translation::providers) translation_mode: Option<String>,
    pub(in crate::features::translation::providers) title: Option<String>,
    pub(in crate::features::translation::providers) body: Option<String>,
    pub(in crate::features::translation::providers) error: Option<String>,
}
