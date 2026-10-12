//! DTO du viewer de logs admin.

// `Serialize` produit les contrats JSON consommes par le frontend admin.
use serde::Serialize;

/// Fichier de logs disponible pour un jour conserve.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AdminLogDay {
    /// Jour UTC du fichier, au format `YYYY-MM-DD`.
    pub day: String,
    /// Taille disque courante du fichier.
    pub size_bytes: u64,
}

/// Reponse de listing des jours disponibles.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AdminLogDaysResponse {
    /// Jours disponibles, tries du plus recent au plus ancien.
    pub days: Vec<AdminLogDay>,
}

/// Ligne de log exposee par l'API admin.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AdminLogLine {
    /// Curseur de reprise, base sur l'index zero-based de la ligne dans le fichier.
    pub cursor: usize,
    /// Niveau detecte dans le rendu texte `tracing`, si disponible.
    pub level: Option<String>,
    /// Ligne brute sans retour chariot final.
    pub text: String,
}

/// Page bornee de logs pour un jour donne.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AdminLogPage {
    /// Jour UTC lu, au format `YYYY-MM-DD`.
    pub day: String,
    /// Lignes retenues apres pagination et filtres.
    pub items: Vec<AdminLogLine>,
    /// Indique qu'une page suivante existe avec les memes filtres.
    pub has_more: bool,
    /// Curseur a renvoyer tel quel pour lire la page suivante.
    pub next_cursor: Option<usize>,
}

/// Snapshot initial envoye au client WebSocket de tail.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AdminLogTailSnapshot {
    /// Jour UTC du fichier observe.
    pub day: String,
    /// Lignes initiales conservees par la fenetre de tail.
    pub items: Vec<AdminLogLine>,
    /// Curseur ligne a partir duquel les futurs append commencent.
    pub next_cursor: usize,
    /// Taille fichier connue apres le snapshot.
    pub file_size_bytes: u64,
}

/// Append incremental envoye quand le fichier de log grossit.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AdminLogTailAppend {
    /// Jour UTC du fichier observe.
    pub day: String,
    /// Nouvelles lignes detectees depuis la derniere lecture.
    pub items: Vec<AdminLogLine>,
    /// Curseur ligne suivant apres application de l'append.
    pub next_cursor: usize,
    /// Taille fichier connue apres l'append.
    pub file_size_bytes: u64,
}
