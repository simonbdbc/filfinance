//! Etat runtime mutable des sources d'ingestion.

// `AtomicBool` porte le hot path; `Mutex` ne protège que le motif admin, lu beaucoup moins souvent.
use std::sync::{
    Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

// `Serialize` garde le contrat JSON local au runtime d'ingestion.
use serde::{Deserialize, Serialize};
// `OffsetDateTime` permet de detecter un heartbeat stale du collecteur navigateur.
use time::{Duration, OffsetDateTime, format_description::well_known::Rfc3339};

use crate::config::XConfig;
use crate::shared::time::now_rfc3339;

/// Etat runtime du collecteur FinancialJuice.
pub struct FinancialJuiceRuntimeState {
    /// Valeur issue de `NEWS_FINANCIAL_JUICE_ENABLED`, conservee comme fallback de redemarrage.
    configured_enabled: bool,
    /// Valeur effective en memoire, modifiable par l'admin sans reecrire `.env`.
    enabled: AtomicBool,
    /// Derniere raison systeme ayant coupe la source, affichee cote admin.
    disabled_reason: Mutex<Option<String>>,
    /// Nombre de reponses HTTP 429 consecutives recues par le collecteur primaire.
    consecutive_rate_limit_failures: AtomicUsize,
}

impl FinancialJuiceRuntimeState {
    /// Initialise l'etat effectif a partir de la configuration chargee au demarrage.
    pub fn new(configured_enabled: bool) -> Self {
        Self {
            configured_enabled,
            enabled: AtomicBool::new(configured_enabled),
            disabled_reason: Mutex::new(None),
            consecutive_rate_limit_failures: AtomicUsize::new(0),
        }
    }

    /// Indique si les workers FinancialJuice doivent faire du travail amont.
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    /// Applique un override admin en memoire pour le process courant.
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
        self.reset_rate_limit_failures();
        self.replace_disabled_reason(None);
    }

    /// Coupe la source apres challenge Cloudflare et indique si elle etait active juste avant.
    pub fn disable_after_cloudflare_managed_challenge(&self) -> bool {
        let was_enabled = self.enabled.swap(false, Ordering::Relaxed);
        self.reset_rate_limit_failures();
        self.replace_disabled_reason(Some("cloudflare_managed_challenge".to_owned()));
        was_enabled
    }

    /// Enregistre un 429 primaire et coupe la source si le seuil consecutif est atteint.
    pub fn record_rate_limit_failure(
        &self,
        disable_threshold: usize,
    ) -> FinancialJuiceRateLimitFailureStatus {
        let consecutive_failures = self
            .consecutive_rate_limit_failures
            .fetch_add(1, Ordering::Relaxed)
            + 1;
        let disabled = consecutive_failures >= disable_threshold;
        let was_enabled = if disabled {
            let was_enabled = self.enabled.swap(false, Ordering::Relaxed);
            self.replace_disabled_reason(Some("rate_limited".to_owned()));
            was_enabled
        } else {
            false
        };

        FinancialJuiceRateLimitFailureStatus {
            consecutive_failures,
            disable_threshold,
            disabled,
            was_enabled,
        }
    }

    /// Repart a zero des qu'une tentative primaire reussit ou echoue pour une autre raison.
    pub fn reset_rate_limit_failures(&self) {
        self.consecutive_rate_limit_failures
            .store(0, Ordering::Relaxed);
    }

    /// Retourne la vue HTTP stable du controle runtime.
    pub fn snapshot(&self) -> FinancialJuiceRuntimeStatus {
        FinancialJuiceRuntimeStatus {
            source: "financial_juice".to_owned(),
            configured_enabled: self.configured_enabled,
            enabled: self.is_enabled(),
            disabled_reason: self.disabled_reason(),
        }
    }

    fn disabled_reason(&self) -> Option<String> {
        self.disabled_reason
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    fn replace_disabled_reason(&self, reason: Option<String>) {
        *self
            .disabled_reason
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = reason;
    }
}

/// Resultat de l'enregistrement d'un 429 primaire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FinancialJuiceRateLimitFailureStatus {
    /// Nombre courant de 429 consecutifs.
    pub consecutive_failures: usize,
    /// Seuil a partir duquel le primaire est coupe.
    pub disable_threshold: usize,
    /// Indique si ce 429 place desormais le primaire en fallback.
    pub disabled: bool,
    /// Indique si le primaire etait actif juste avant la coupure.
    pub was_enabled: bool,
}

/// Payload expose par `GET/PATCH /api/runtime/financial-juice`.
#[derive(Debug, Clone, Serialize)]
pub struct FinancialJuiceRuntimeStatus {
    /// Source controlee par ce runtime.
    pub source: String,
    /// Fallback env charge depuis `NEWS_FINANCIAL_JUICE_ENABLED`.
    pub configured_enabled: bool,
    /// Etat effectif courant utilise par les workers.
    pub enabled: bool,
    /// Raison technique de la derniere coupure automatique, si applicable.
    pub disabled_reason: Option<String>,
}

/// Etat runtime declare par le collecteur navigateur FinancialJuice separe.
pub struct FinancialJuiceBrowserRuntimeState {
    /// Valeur issue de `NEWS_FINANCIAL_JUICE_BROWSER_ENABLED`.
    configured_enabled: bool,
    /// Dernier statut connu du process Playwright externe.
    inner: Mutex<FinancialJuiceBrowserRuntimeInner>,
}

/// Donnees mutables declarees par le collecteur navigateur.
struct FinancialJuiceBrowserRuntimeInner {
    /// Etat operationnel courant annonce par le collecteur externe.
    status: BrowserCollectorStatus,
    /// Message court utile pour expliquer un blocage Cloudflare ou une degradation.
    message: Option<String>,
    /// Horodatage de la derniere mise a jour recue.
    updated_at: Option<String>,
}

impl FinancialJuiceBrowserRuntimeState {
    /// Initialise le statut a `stopped` jusqu'au premier heartbeat du collecteur externe.
    pub fn new(configured_enabled: bool) -> Self {
        Self {
            configured_enabled,
            inner: Mutex::new(FinancialJuiceBrowserRuntimeInner {
                status: BrowserCollectorStatus::Stopped,
                message: Some("collector status has not been reported".to_owned()),
                updated_at: None,
            }),
        }
    }

    /// Enregistre le dernier statut envoye par le process navigateur.
    pub fn update_status(&self, status: BrowserCollectorStatus, message: Option<String>) {
        *self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = FinancialJuiceBrowserRuntimeInner {
            status,
            message,
            updated_at: Some(now_rfc3339()),
        };
    }

    /// Construit la vue HTTP stable du runtime navigateur.
    pub fn snapshot(
        &self,
        ingest_token_configured: bool,
        primary_enabled: bool,
        stale_after_ms: u64,
    ) -> FinancialJuiceBrowserRuntimeStatus {
        let inner = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let effective = self.effective_status_from_inner(&inner, primary_enabled, stale_after_ms);

        FinancialJuiceBrowserRuntimeStatus {
            source: "financial_juice_browser".to_owned(),
            configured_enabled: self.configured_enabled,
            primary_enabled,
            fallback_active: effective.fallback_active,
            ingest_token_configured,
            status: inner.status.clone(),
            message: inner.message.clone(),
            updated_at: inner.updated_at.clone(),
            effective_enabled: effective.enabled,
            service_accessible: effective.service_accessible,
            main_feed_available: effective.main_feed_available,
            stale: effective.stale,
            effective_message: effective.message,
        }
    }

    /// Indique si la source navigateur est effectivement exploitable pour `/api/sources`.
    #[cfg(test)]
    pub fn effective_enabled(&self, primary_enabled: bool, stale_after_ms: u64) -> bool {
        self.effective_status(primary_enabled, stale_after_ms)
            .enabled
    }

    /// Retourne la projection operationnelle utilisee par `/api/sources` et le monitoring.
    pub fn effective_status(
        &self,
        primary_enabled: bool,
        stale_after_ms: u64,
    ) -> BrowserCollectorEffectiveStatus {
        let inner = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        self.effective_status_from_inner(&inner, primary_enabled, stale_after_ms)
    }

    fn effective_status_from_inner(
        &self,
        inner: &FinancialJuiceBrowserRuntimeInner,
        primary_enabled: bool,
        stale_after_ms: u64,
    ) -> BrowserCollectorEffectiveStatus {
        let fallback_active = self.configured_enabled && !primary_enabled;
        let stale = fallback_active
            && inner.updated_at.is_some()
            && status_updated_at_is_stale(inner.updated_at.as_deref(), stale_after_ms);
        let status = if stale {
            "stale".to_owned()
        } else {
            inner.status.as_str().to_owned()
        };

        let message = effective_browser_status_message(
            self.configured_enabled,
            primary_enabled,
            &inner.status,
            inner.message.clone(),
            inner.updated_at.as_deref(),
            stale,
        );
        let service_accessible =
            fallback_active && inner.updated_at.is_some() && !stale && inner.status.is_service_up();
        let main_feed_available = service_accessible && inner.status == BrowserCollectorStatus::Ok;
        let enabled = fallback_active && service_accessible && main_feed_available;

        BrowserCollectorEffectiveStatus {
            fallback_active,
            enabled,
            status,
            message,
            updated_at: inner.updated_at.clone(),
            service_accessible,
            main_feed_available,
            stale,
        }
    }
}

/// Statuts explicites autorises pour le collecteur navigateur.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BrowserCollectorStatus {
    /// Le navigateur lit et publie les evenements attendus.
    Ok,
    /// Le navigateur est bloque par login, challenge, session ou acces amont.
    Blocked,
    /// Le navigateur tourne mais la qualite ou la fraicheur du flux est degradee.
    Degraded,
    /// Le collecteur est arrete ou volontairement desactive.
    Stopped,
}

impl BrowserCollectorStatus {
    /// Representation stable exposee par `/api/sources`.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Blocked => "blocked",
            Self::Degraded => "degraded",
            Self::Stopped => "stopped",
        }
    }

    fn is_service_up(&self) -> bool {
        matches!(self, Self::Ok | Self::Degraded | Self::Blocked)
    }
}

/// Projection operationnelle du fallback navigateur.
#[derive(Debug, Clone)]
pub struct BrowserCollectorEffectiveStatus {
    /// Indique si le fallback doit travailler maintenant.
    pub fallback_active: bool,
    /// Indique si le fallback est utilisable pour publier des news.
    pub enabled: bool,
    /// Statut expose a `/api/sources`, enrichi avec `stale` si le heartbeat expire.
    pub status: String,
    /// Message court exploitable par logs et frontend.
    pub message: Option<String>,
    /// Horodatage du dernier heartbeat connu.
    pub updated_at: Option<String>,
    /// Le process externe a-t-il emis un heartbeat recent.
    pub service_accessible: bool,
    /// Le DOM attendu `#mainFeed[data-feedid="mainfeed"]` est-il confirme disponible.
    pub main_feed_available: bool,
    /// Le dernier heartbeat est-il trop ancien.
    pub stale: bool,
}

/// Payload expose par `GET /api/runtime/financial-juice-browser`.
#[derive(Debug, Clone, Serialize)]
pub struct FinancialJuiceBrowserRuntimeStatus {
    /// Source controlee par ce runtime externe.
    pub source: String,
    /// Fallback env charge depuis `NEWS_FINANCIAL_JUICE_BROWSER_ENABLED`.
    pub configured_enabled: bool,
    /// Etat effectif courant du collecteur Rust `financial_juice`, prioritaire sur le navigateur.
    pub primary_enabled: bool,
    /// Indique si le collecteur navigateur doit travailler maintenant.
    pub fallback_active: bool,
    /// Indique si le token d'ingestion machine-to-machine est configure.
    pub ingest_token_configured: bool,
    /// Dernier statut declare par le collecteur navigateur.
    pub status: BrowserCollectorStatus,
    /// Message operationnel court associe au statut.
    pub message: Option<String>,
    /// Horodatage de la derniere declaration de statut.
    pub updated_at: Option<String>,
    /// Etat effectif utilise par `/api/sources`.
    pub effective_enabled: bool,
    /// Indique si le service externe a emis un heartbeat recent.
    pub service_accessible: bool,
    /// Indique si le collecteur a confirme `#mainFeed[data-feedid="mainfeed"]`.
    pub main_feed_available: bool,
    /// Indique si le dernier heartbeat est trop ancien.
    pub stale: bool,
    /// Message operationnel calcule pour le frontend et les logs.
    pub effective_message: Option<String>,
}

/// Delai de fraicheur du heartbeat browser deduit de la cadence de polling.
pub fn browser_collector_status_stale_after_ms(poll_interval_ms: u64) -> u64 {
    poll_interval_ms.saturating_mul(3).max(30_000)
}

fn status_updated_at_is_stale(updated_at: Option<&str>, stale_after_ms: u64) -> bool {
    let Some(updated_at) = updated_at else {
        return false;
    };
    let Ok(parsed) = OffsetDateTime::parse(updated_at, &Rfc3339) else {
        return true;
    };

    OffsetDateTime::now_utc() - parsed > Duration::milliseconds(stale_after_ms as i64)
}

fn effective_browser_status_message(
    configured_enabled: bool,
    primary_enabled: bool,
    status: &BrowserCollectorStatus,
    message: Option<String>,
    updated_at: Option<&str>,
    stale: bool,
) -> Option<String> {
    if !configured_enabled {
        return Some("fallback disabled by NEWS_FINANCIAL_JUICE_BROWSER_ENABLED=false".to_owned());
    }
    if primary_enabled {
        return Some("fallback idle: financial_juice primary is active".to_owned());
    }
    if updated_at.is_none() {
        return Some(
            "financial_juice_browser collector has not reported status; start the collector service"
                .to_owned(),
        );
    }
    if stale {
        return Some(format!(
            "financial_juice_browser collector heartbeat is stale; last status at {}",
            updated_at.unwrap_or("unknown")
        ));
    }
    if matches!(status, BrowserCollectorStatus::Stopped) {
        return Some(
            "financial_juice_browser collector is stopped while fallback is active".to_owned(),
        );
    }

    message.or_else(|| {
        Some(match status {
            BrowserCollectorStatus::Ok => {
                "FinancialJuice browser collector is healthy and #mainFeed is available"
            }
            BrowserCollectorStatus::Blocked => {
                "FinancialJuice browser collector is blocked; #mainFeed is not available"
            }
            BrowserCollectorStatus::Degraded => {
                "FinancialJuice browser collector is degraded; #mainFeed availability is not confirmed"
            }
            BrowserCollectorStatus::Stopped => "FinancialJuice browser collector is stopped",
        }
        .to_owned())
    })
}

/// Etat runtime du collecteur X.
pub struct XRuntimeState {
    /// Valeur issue de `NEWS_X_ENABLED`, conservee comme fallback de redemarrage.
    configured_enabled: bool,
    /// Etat effectif modifiable par l'admin et relu par le worker X.
    inner: Mutex<XRuntimeInner>,
}

/// Donnees mutables du collecteur X.
struct XRuntimeInner {
    /// Active ou non la collecte X.
    enabled: bool,
    /// Derniere raison systeme ayant coupe la source X, affichee cote admin.
    disabled_reason: Option<String>,
    /// Comptes X surveilles, sans prefixe `@`.
    account_usernames: Vec<String>,
    /// Mots cles optionnels ajoutes a la requete.
    keywords: Vec<String>,
    /// Horodatage de la derniere persistence SQLite connue.
    updated_at: Option<String>,
}

impl XRuntimeState {
    /// Initialise l'etat runtime depuis l'environnement.
    pub fn new(config: &XConfig) -> Self {
        Self {
            configured_enabled: config.source.enabled,
            inner: Mutex::new(XRuntimeInner {
                enabled: config.source.enabled,
                disabled_reason: None,
                account_usernames: config.account_usernames.clone(),
                keywords: config.keywords.clone(),
                updated_at: None,
            }),
        }
    }

    /// Retourne une copie stable des parametres de polling X.
    pub fn settings(&self) -> XRuntimeSettings {
        let inner = self.inner();

        XRuntimeSettings {
            enabled: inner.enabled,
            disabled_reason: inner.disabled_reason,
            account_usernames: inner.account_usernames,
            keywords: inner.keywords,
            updated_at: inner.updated_at,
        }
    }

    /// Remplace l'etat runtime apres chargement ou persistence SQLite.
    pub fn replace_settings(&self, settings: XRuntimeSettings) {
        *self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = XRuntimeInner {
            enabled: settings.enabled,
            disabled_reason: None,
            account_usernames: settings.account_usernames,
            keywords: settings.keywords,
            updated_at: settings.updated_at,
        };
    }

    /// Coupe la source apres un refus billing X et indique si elle etait active juste avant.
    pub fn disable_after_payment_required(&self) -> bool {
        let mut inner = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let was_enabled = inner.enabled;
        inner.enabled = false;
        inner.disabled_reason = Some("x_payment_required".to_owned());
        was_enabled
    }

    /// Construit la vue HTTP stable du runtime X.
    pub fn snapshot(
        &self,
        bearer_token_configured: bool,
        persistence_available: bool,
    ) -> XRuntimeStatus {
        let settings = self.settings();

        XRuntimeStatus {
            source: "x".to_owned(),
            configured_enabled: self.configured_enabled,
            enabled: settings.enabled,
            disabled_reason: settings.disabled_reason,
            bearer_token_configured,
            persistence_available,
            account_usernames: settings.account_usernames,
            keywords: settings.keywords,
            updated_at: settings.updated_at,
        }
    }

    fn inner(&self) -> XRuntimeInner {
        let inner = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        XRuntimeInner {
            enabled: inner.enabled,
            disabled_reason: inner.disabled_reason.clone(),
            account_usernames: inner.account_usernames.clone(),
            keywords: inner.keywords.clone(),
            updated_at: inner.updated_at.clone(),
        }
    }
}

/// Parametres persistables de la source X.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XRuntimeSettings {
    /// Active ou non la collecte X.
    pub enabled: bool,
    /// Derniere raison systeme ayant coupe la source X, si presente.
    pub disabled_reason: Option<String>,
    /// Comptes X surveilles, sans prefixe `@`.
    pub account_usernames: Vec<String>,
    /// Mots cles optionnels ajoutes a la requete.
    pub keywords: Vec<String>,
    /// Horodatage de la derniere persistence SQLite connue.
    pub updated_at: Option<String>,
}

/// Payload expose par `GET/PATCH /api/runtime/x`.
#[derive(Debug, Clone, Serialize)]
pub struct XRuntimeStatus {
    /// Source controlee par ce runtime.
    pub source: String,
    /// Fallback env charge depuis `NEWS_X_ENABLED`.
    pub configured_enabled: bool,
    /// Etat effectif courant utilise par le worker X.
    pub enabled: bool,
    /// Raison technique de la derniere coupure automatique, si applicable.
    pub disabled_reason: Option<String>,
    /// Indique si le bearer token serveur est configure.
    pub bearer_token_configured: bool,
    /// Indique si la persistence SQLite est disponible pour les changements admin.
    pub persistence_available: bool,
    /// Comptes X surveilles, sans prefixe `@`.
    pub account_usernames: Vec<String>,
    /// Mots cles optionnels ajoutes a la requete.
    pub keywords: Vec<String>,
    /// Horodatage SQLite de la derniere mise a jour connue.
    pub updated_at: Option<String>,
}
