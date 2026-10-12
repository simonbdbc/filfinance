//! Etats partagés du contrat edge live.

use serde::{Deserialize, Serialize};

use crate::features::{
    calendar_day::PublicCalendarDaySummaryStatus, news::domain::PublicSourceStatus,
};

/// Etats audio exposés sur le futur control plane edge.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeLiveAudioStatus {
    // Aucun flux live n'est actuellement en lecture.
    Idle,
    // Le player ou l'origin prépare le prochain segment.
    Buffering,
    // Le flux live est nominalement en cours de lecture.
    Playing,
    // Le flux existe mais le client ou l'origin signale un retard.
    Delayed,
    // Le flux audio est indisponible ou en erreur.
    Error,
}

/// Sévérité courte des notifications système diffusées par l'edge.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeSystemNoticeSeverity {
    // Information non bloquante, utile pour l'état live.
    Info,
    // Dégradation notable mais service encore exploitable.
    Warning,
    // Erreur qui demande une action ou une surveillance.
    Error,
}

/// Carte compacte de news destinée au fan-out edge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeNewsCard {
    /// Identifiant de l'événement source.
    pub news_event_id: i64,
    /// Identifiant de traduction si la carte est déjà rattachée à une variante FR.
    pub translation_id: Option<i64>,
    /// Source logique de la news.
    pub source: String,
    /// Type d'événement aplati pour les clients edge.
    pub event_kind: String,
    /// Titre source EN.
    pub source_title: String,
    /// Titre FR si la traduction est déjà disponible.
    pub translated_title: Option<String>,
    /// Niveau FinancialJuice ou équivalent source, conservé comme metadata.
    pub news_level: Option<String>,
    /// URL source si disponible.
    pub url: Option<String>,
    /// Timestamp métier retenu pour le tri et l'affichage.
    pub timestamp: String,
}

/// Metadata du contenu audio global actuellement annoncé comme "now playing".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeNowPlaying {
    /// Identifiant du stream audio s'il existe côté origin.
    pub stream_id: Option<String>,
    /// Traduction rattachée au segment courant si connue.
    pub translation_id: Option<i64>,
    /// Evénement source rattaché si connu.
    pub news_event_id: Option<i64>,
    /// Titre affiché pour l'item en cours.
    pub title: Option<String>,
    /// Niveau source rattaché au contenu en cours.
    pub news_level: Option<String>,
    /// Date à laquelle cet item est devenu actif.
    pub started_at: String,
    /// Timestamp métier de la news associée si disponible.
    pub timestamp: Option<String>,
}

/// Etat public du control plane audio, sans aucun octet audio.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeAudioState {
    /// Etat fonctionnel courant du live audio.
    pub status: EdgeLiveAudioStatus,
    /// URL publique du replay HLS du jour quand il est disponible.
    pub replay_today_playlist_url: Option<String>,
    /// Détail humain court en cas de dégradation.
    pub detail: Option<String>,
    /// Date de dernière mise à jour de cet état.
    pub updated_at: String,
}

/// Etat minimal renvoyé à la reconnexion edge avant les nouveaux événements.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeLiveSnapshot {
    /// Dernier identifiant d'événement metadata accepté par le Durable Object.
    pub latest_event_id: Option<String>,
    /// Item audio global en cours, si le live en a un.
    pub now_playing: Option<EdgeNowPlaying>,
    /// Etat audio courant annoncé aux clients.
    pub audio_state: Option<EdgeAudioState>,
    /// Derniers items traduits gardés pour restaurer l'historique visible après reconnexion edge.
    #[serde(default)]
    pub translated_news_items: Vec<serde_json::Value>,
    /// Derniers événements audio metadata gardés pour rattraper une reconnexion courte.
    #[serde(default)]
    pub audio_events: Vec<serde_json::Value>,
    /// Dernier etat public connu des sources de news.
    #[serde(default)]
    pub source_statuses: Vec<PublicSourceStatus>,
    /// Dernier statut public connu du résumé calendrier du jour.
    #[serde(default)]
    pub calendar_day_summary: Option<PublicCalendarDaySummaryStatus>,
}
