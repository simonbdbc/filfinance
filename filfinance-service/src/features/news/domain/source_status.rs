//! Projection publique de la configuration des sources de collecte.

// Serialize garde le contrat JSON expose par /api/sources et par les heartbeats publics.
use serde::{Deserialize, Serialize};

use crate::config::{FinancialJuiceSiteConfig, SourceConfig};

/// Vue resumee d'une source exposee par l'API.
#[derive(Debug, Clone, Serialize)]
pub struct SourceStatus {
    /// Nom logique de la source.
    pub name: String,
    /// Etat active/desactive.
    pub enabled: bool,
    /// Fallback charge depuis l'environnement avant override runtime eventuel.
    pub configured_enabled: bool,
    /// URL de polling.
    pub url: String,
    /// Frequence de polling.
    pub poll_interval_ms: u64,
    /// Timeout HTTP de cette source.
    pub request_timeout_ms: u64,
    /// Statut runtime optionnel, par exemple `ok`, `blocked`, `stale` ou `stopped`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// Message operationnel court associe au statut runtime.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Horodatage du dernier statut runtime connu.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    /// Indique si le service externe associe est joignable via heartbeat.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_accessible: Option<bool>,
    /// Indique si le DOM source attendu est disponible.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub main_feed_available: Option<bool>,
    /// Indique si le dernier heartbeat connu est trop ancien.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stale: Option<bool>,
}

/// Vue publique compacte d'une source diffusee dans les WebSockets live.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PublicSourceStatus {
    /// Nom logique stable de la source.
    pub source: String,
    /// Etat effectif public de la source.
    pub enabled: bool,
    /// Statut public stable: active, disabled, blocked, degraded ou stale.
    pub status: String,
    /// Horodatage de la projection ou du dernier statut runtime connu.
    pub updated_at: String,
}

impl SourceStatus {
    /// Construit la vue API a partir de la configuration interne d'une source.
    pub fn from_config(config: &SourceConfig) -> Self {
        Self {
            name: config.name.clone(),
            enabled: config.enabled,
            configured_enabled: config.enabled,
            url: config.url.clone(),
            poll_interval_ms: config.poll_interval_ms,
            request_timeout_ms: config.request_timeout_ms,
            status: None,
            message: None,
            updated_at: None,
            service_accessible: None,
            main_feed_available: None,
            stale: None,
        }
    }

    /// Variante dediee au collecteur FinancialJuice site, dont l'URL utile est `/home`.
    pub fn from_financial_juice_site(config: &FinancialJuiceSiteConfig) -> Self {
        Self {
            name: config.name.clone(),
            enabled: config.enabled,
            configured_enabled: config.enabled,
            url: config.home_url.clone(),
            poll_interval_ms: config.poll_interval_ms,
            request_timeout_ms: config.request_timeout_ms,
            status: None,
            message: None,
            updated_at: None,
            service_accessible: None,
            main_feed_available: None,
            stale: None,
        }
    }

    /// Ajoute la projection runtime d'une source qui expose son propre etat operationnel.
    pub fn set_runtime_status(
        &mut self,
        status: String,
        message: Option<String>,
        updated_at: Option<String>,
        service_accessible: bool,
        main_feed_available: bool,
        stale: bool,
    ) {
        self.status = Some(status);
        self.message = message;
        self.updated_at = updated_at;
        self.service_accessible = Some(service_accessible);
        self.main_feed_available = Some(main_feed_available);
        self.stale = Some(stale);
    }
}
