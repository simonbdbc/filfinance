//! Type metier d'un evenement news canonique.

// fmt sert a implementer Display sur EventKind.
use std::fmt;

// Serialize et Deserialize permettent au type d'entrer/sortir en JSON.
use serde::{Deserialize, Serialize};

/// Distingue les familles d'evenements supportees par le service.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    // Headline classique provenant du flux news.
    Headline,
    // Evenement de calendrier macroeconomique, plus riche en metadata.
    MacroCalendar,
}

impl EventKind {
    /// Retourne la representation stable utilisee en persistence et en sortie.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Headline => "headline",
            Self::MacroCalendar => "macro_calendar",
        }
    }
}

impl fmt::Display for EventKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
