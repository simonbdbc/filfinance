//! Acquisition `FinancialJuice` via session web, API interne et live SignalR.

// `Arc` partage le cache de session entre polling et live.
use std::{sync::Arc, time::Instant};
// `Client` est partagé par les sous-collecteurs HTTP et SignalR.
use reqwest::Client;
// `Mutex` garde un cache de session async cohérent.
use tokio::sync::Mutex;
// Le warning de coupure runtime doit etre visible sans spammer ensuite les boucles des workers.
use tracing::warn;

// On importe ici la configuration dédiée FinancialJuice et l'état partagé du collecteur.
use crate::{app_state::AppState, config::FinancialJuiceSiteConfig, errors::AppError};
// `constants` regroupe les valeurs stables du protocole FinancialJuice.
mod constants;
// `model` porte les DTO FinancialJuice et les conversions vers nos domaines internes.
pub(in crate::features::ingestion) mod model;
// `live` isole le décodage WebSocket SignalR et la boucle de reconnexion.
mod live;
// `poll` isole le backfill et le polling news FinancialJuice.
mod poll;
// `request` reserve l'encodage URL/formulaire commun aux endpoints WebForms et SignalR.
mod request;
// `session` isole le login WebForms et le cache de bootstrap authentifié.
mod session;
// `signalr` reserve les DTO et helpers du transport live FinancialJuice.
mod signalr;

pub(crate) use poll::spawn_site;

use constants::{
    FINANCIAL_JUICE_MAIN_FEED_ID, FINANCIAL_JUICE_OUTSIDE_WINDOW_PAGE_STREAK_LIMIT,
    FINANCIAL_JUICE_PERIODIC_CATCHUP_PAGE_LIMIT, FINANCIAL_JUICE_SIGNALR_CLIENT_PROTOCOL,
    FINANCIAL_JUICE_SIGNALR_CONNECTION_DATA, FINANCIAL_JUICE_SIGNALR_HUB_NAME,
};
use model::FinancialJuiceBootstrap;
use request::{
    build_url_with_query, encode_form_body, ensure_success_response,
    is_financial_juice_access_challenge, is_financial_juice_rate_limited,
};

const FINANCIAL_JUICE_RATE_LIMIT_AUTO_DISABLE_THRESHOLD: usize = 10;

/// Collecteur site authentifié `FinancialJuice`, partagé entre poll, backfill et live.
#[derive(Clone)]
pub struct FinancialJuiceSiteCollector {
    /// Configuration dédiée au chemin `/home -> FJService -> SignalR`.
    config: FinancialJuiceSiteConfig,
    /// Client HTTP partagé avec cookie-store actif.
    client: Client,
    /// Cache de bootstrap authentifié pour éviter de relogger à chaque appel.
    bootstrap_cache: Arc<Mutex<Option<CachedBootstrap>>>,
}

/// Etat mis en cache après un bootstrap FinancialJuice réussi.
struct CachedBootstrap {
    /// Variables de session et de bootstrap réellement exploitables.
    bootstrap: FinancialJuiceBootstrap,
    /// Horodatage local du refresh de session.
    refreshed_at: Instant,
}

impl FinancialJuiceSiteCollector {
    /// Construit une instance partagée entre bootstrap paginé, poll et live.
    pub fn new(config: FinancialJuiceSiteConfig, client: Client) -> Self {
        Self {
            // La config est possédée par la struct car le collecteur vit plus longtemps que l'appelant.
            config,
            // Le client `reqwest` est clonable et partage en interne son pool et son cookie store.
            client,
            // `Arc<Mutex<Option<_>>>` permet au poll HTTP et au live SignalR de réutiliser la même
            // session authentifiée sans recopier le bootstrap ni faire de login concurrent.
            bootstrap_cache: Arc::new(Mutex::new(None)),
        }
    }
}

/// Effet runtime applique apres une erreur du primaire FinancialJuice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FinancialJuiceRuntimeFailureAction {
    /// Erreur transitoire sans changement runtime.
    Retry,
    /// 429 observe, sans atteindre encore le seuil de coupure.
    RateLimited,
    /// Le primaire vient d'etre coupe et le fallback navigateur peut prendre le relais.
    Disabled,
}

impl FinancialJuiceRuntimeFailureAction {
    fn primary_disabled(self) -> bool {
        matches!(self, Self::Disabled)
    }

    fn use_rate_limit_backoff(self) -> bool {
        matches!(self, Self::RateLimited | Self::Disabled)
    }
}

/// Applique la politique runtime apres une erreur upstream du primaire FinancialJuice.
fn handle_runtime_upstream_error(
    state: &AppState,
    source_name: &str,
    operation: &str,
    error: &AppError,
) -> FinancialJuiceRuntimeFailureAction {
    if is_financial_juice_access_challenge(error) {
        let was_enabled = state
            .financial_juice_runtime
            .disable_after_cloudflare_managed_challenge();
        warn!(
            %error,
            source = %source_name,
            operation,
            was_enabled,
            "financial juice runtime disabled after cloudflare managed challenge"
        );
        return FinancialJuiceRuntimeFailureAction::Disabled;
    }

    if is_financial_juice_rate_limited(error) {
        let status = state
            .financial_juice_runtime
            .record_rate_limit_failure(FINANCIAL_JUICE_RATE_LIMIT_AUTO_DISABLE_THRESHOLD);
        warn!(
            %error,
            source = %source_name,
            operation,
            consecutive_429_failures = status.consecutive_failures,
            disable_threshold = status.disable_threshold,
            was_enabled = status.was_enabled,
            "financial juice primary rate limit failure recorded"
        );
        if status.disabled {
            warn!(
                source = %source_name,
                operation,
                consecutive_429_failures = status.consecutive_failures,
                disable_threshold = status.disable_threshold,
                was_enabled = status.was_enabled,
                "financial juice runtime disabled after consecutive 429 rate limits"
            );
            return FinancialJuiceRuntimeFailureAction::Disabled;
        }

        return FinancialJuiceRuntimeFailureAction::RateLimited;
    }

    state.financial_juice_runtime.reset_rate_limit_failures();
    FinancialJuiceRuntimeFailureAction::Retry
}

/// Remet a zero la serie de 429 des qu'une operation primaire aboutit.
fn reset_runtime_rate_limit_failures(state: &AppState) {
    state.financial_juice_runtime.reset_rate_limit_failures();
}
