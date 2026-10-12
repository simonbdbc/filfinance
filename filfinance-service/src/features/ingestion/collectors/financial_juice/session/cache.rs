//! Cache de bootstrap authentifié FinancialJuice.

// `Duration` vérifie le TTL de session, `Instant` horodate le dernier login réussi.
use std::time::{Duration, Instant};

use crate::errors::AppError;

use super::super::{CachedBootstrap, FinancialJuiceSiteCollector, model::FinancialJuiceBootstrap};

impl FinancialJuiceSiteCollector {
    /// Retourne un bootstrap authentifié, en le rafraîchissant si le cache est absent ou expiré.
    pub(in crate::features::ingestion::collectors::financial_juice) async fn ensure_bootstrap(
        &self,
        force_refresh: bool,
    ) -> Result<FinancialJuiceBootstrap, AppError> {
        // Le cache n'est relu que si le caller n'a pas explicitement demandé un refresh.
        if !force_refresh {
            // `lock().await` suspend la tâche Tokio sans bloquer le thread OS pendant l'accès au cache.
            let guard = self.bootstrap_cache.lock().await;
            // `guard.as_ref()` convertit `&Option<_>` en `Option<&_>` pour emprunter le cache sans le déplacer.
            if let Some(cached) = guard.as_ref() {
                // `elapsed()` permet un cache TTL simple, lisible et suffisant pour ce MVP.
                if cached.refreshed_at.elapsed()
                    < Duration::from_millis(self.config.session_refresh_interval_ms)
                {
                    // On clone uniquement le petit bootstrap, pas tout le collecteur.
                    return Ok(cached.bootstrap.clone());
                }
            }
        }

        // Si le cache est absent, expiré ou forcé, on refait un login complet.
        let bootstrap = self.authenticate_and_bootstrap().await?;
        // On reprend ensuite le mutex en écriture pour publier la nouvelle session à tous les consommateurs.
        let mut guard = self.bootstrap_cache.lock().await;
        *guard = Some(CachedBootstrap {
            bootstrap: bootstrap.clone(),
            refreshed_at: Instant::now(),
        });

        Ok(bootstrap)
    }

    /// Vide explicitement le cache pour forcer un nouveau login au prochain cycle.
    pub(in crate::features::ingestion::collectors::financial_juice) async fn invalidate_bootstrap(
        &self,
    ) {
        // Réassigner `None` est plus clair qu'un `take()` ici: on ne cherche pas à récupérer l'ancienne valeur.
        let mut guard = self.bootstrap_cache.lock().await;
        *guard = None;
    }
}
