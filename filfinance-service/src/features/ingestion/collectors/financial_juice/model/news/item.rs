//! DTO d'une news FinancialJuice et conversion vers NewsEvent canonique.

// HashMap capture les champs inconnus exposés par serde(flatten).
use std::collections::HashMap;

use serde::Deserialize;
use serde_json::Value;

use crate::features::news::domain::NewsEvent;

use super::{
    super::common::{
        absolutize_url_from_site, non_empty, normalize_description, normalize_timestamp,
        normalize_title,
    },
    level::normalize_news_level,
};

/// Ligne news brute renvoyée par `GetPreviousNews` et par `sendUpdates`.
#[derive(Debug, Clone, Deserialize)]
pub(in crate::features::ingestion) struct FinancialJuiceApiNewsItem {
    /// Identifiant numérique stable de la news.
    #[serde(rename = "NewsID")]
    pub(in crate::features::ingestion) news_id: i64,
    /// Titre brut.
    #[serde(rename = "Title", default)]
    pub(in crate::features::ingestion) title: String,
    /// Description brute.
    #[serde(rename = "Description", default)]
    pub(in crate::features::ingestion) description: String,
    /// Date de publication au format source.
    #[serde(rename = "DatePublished")]
    pub(in crate::features::ingestion) date_published: Option<String>,
    /// URL FinancialJuice si présente.
    #[serde(rename = "EURL", default)]
    pub(in crate::features::ingestion) url: String,
    /// Identifiant métier auxiliaire fourni par FJ.
    #[serde(rename = "IID")]
    pub(in crate::features::ingestion) iid: Option<String>,
    /// Niveau d'affichage FinancialJuice, par exemple `active active-critical` pour les news critiques.
    #[serde(rename = "Level")]
    pub(in crate::features::ingestion) level: Option<String>,
    /// Champs non encore mappés, utiles pour repérer un éventuel signal `critical`/`breaking`.
    #[serde(flatten)]
    pub(in crate::features::ingestion) extra: HashMap<String, Value>,
}

impl FinancialJuiceApiNewsItem {
    /// Convertit une ligne API FinancialJuice en `NewsEvent` canonique.
    pub(in crate::features::ingestion) fn into_news_event(
        self,
        source_name: &str,
    ) -> Option<NewsEvent> {
        self.into_news_event_with_published_at(source_name, None)
    }

    /// Variante utilitaire qui permet au backlog de remplacer la date source par une version reparée.
    pub(in crate::features::ingestion::collectors::financial_juice::model::news) fn into_news_event_with_published_at(
        self,
        source_name: &str,
        published_at_override: Option<String>,
    ) -> Option<NewsEvent> {
        let title = normalize_title(&self.title);
        let url = absolutize_url_from_site(Some(self.url.as_str()))?;

        if title.is_empty() {
            return None;
        }

        let mut event = NewsEvent::new_headline(
            source_name.to_owned(),
            title,
            url.clone(),
            published_at_override.or_else(|| {
                normalize_timestamp(self.date_published.as_deref().unwrap_or_default())
            }),
            normalize_description(&self.description),
        );
        event.news_level = normalize_news_level(self.level.as_deref());
        event.raw_payload_ref = self
            .iid
            .as_deref()
            .and_then(non_empty)
            .or_else(|| Some(format!("financial_juice_news_id:{}", self.news_id)));
        event.recompute_signature();
        Some(event)
    }
}
