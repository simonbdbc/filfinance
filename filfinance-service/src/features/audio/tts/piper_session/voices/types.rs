//! Types publics du catalogue de voix Piper.

/// Representation canonique d'un choix de voix expose au client et reutilise par le helper Python.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::features::audio) struct PiperVoiceSelection {
    /// Identifiant public plat expose au client, par exemple `fr_FR-upmc-medium-jessica`.
    pub(in crate::features::audio) selection_id: String,
    /// Identifiant du modele Piper sous-jacent.
    pub(in crate::features::audio) voice_id: String,
}
