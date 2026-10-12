//! Cache JWKS Better Auth local au process.

// `OnceLock` initialise le cache JWKS au premier usage, `Duration` borne sa duree de vie et
// `Instant` permet de mesurer un age monotone sans dependre de l'horloge systeme.
use std::{
    future::Future,
    sync::OnceLock,
    time::{Duration, Instant},
};

use jsonwebtoken::jwk::JwkSet;
// Le `RwLock` protege l'etat court du cache; le `Mutex` coalesce les fetches reseau concurrents.
use tokio::sync::{Mutex, RwLock};

// La configuration auth porte l'URL JWKS et l'audience attendue.
use crate::config::AuthConfig;
// La taxonomie d'erreur commune garde le mapping HTTP stable.
use crate::errors::AppError;

/// Duree maximale de conservation locale du JWKS Better Auth.
// Une borne courte limite la duree de confiance locale pendant une rotation de cle.
const JWKS_CACHE_TTL: Duration = Duration::from_secs(10 * 60);

/// Intervalle minimal entre deux tentatives reseau provoquees par un `kid` inconnu.
// La fenetre reste courte pour rendre une rotation visible rapidement tout en bornant la charge
// qu'un flot de JWT aux `kid` arbitraires peut imposer a Better Auth.
const UNKNOWN_KID_REFRESH_COOLDOWN: Duration = Duration::from_secs(5);

/// Cache process-wide du dernier JWKS charge.
// Le cache est global au process car tous les routeurs partagent le meme service Better Auth.
static JWKS_CACHE: OnceLock<JwksCache> = OnceLock::new();

/// Etat et verrou singleflight du cache JWKS.
#[derive(Debug, Default)]
struct JwksCache {
    /// L'etat reste lisible pendant un fetch reseau en cours.
    state: RwLock<JwksCacheState>,
    /// Une seule tache a la fois peut rafraichir le JWKS; les suivantes reutilisent son resultat.
    refresh: Mutex<()>,
}

/// Donnees protegees du cache JWKS.
#[derive(Debug, Default)]
struct JwksCacheState {
    /// Dernier jeu de cles charge, s'il existe.
    cached: Option<CachedJwkSet>,
    /// Derniere tentative reseau terminee, y compris en erreur, pour freiner les `kid` inconnus.
    last_refresh_attempt: Option<RefreshAttempt>,
}

/// Horodatage d'une tentative de refresh pour une URL de configuration precise.
#[derive(Debug, Clone)]
struct RefreshAttempt {
    jwks_url: String,
    completed_at: Instant,
}

/// Entree de cache JWKS associee a l'URL qui l'a produite.
#[derive(Debug, Clone)]
struct CachedJwkSet {
    /// URL source pour eviter de reutiliser des cles si la config change en test ou en dev.
    jwks_url: String,
    /// Instant de fetch utilise pour appliquer `JWKS_CACHE_TTL`.
    fetched_at: Instant,
    /// Jeu de cles publiques Better Auth retourne par l'endpoint JWKS.
    keys: JwkSet,
}

/// Charge le JWKS Better Auth depuis le cache ou depuis le reseau.
pub(super) async fn load_jwks(
    config: &AuthConfig,
    force_refresh: bool,
) -> Result<JwkSet, AppError> {
    // La validation de config garantit cette URL quand auth est activee, mais on reste explicite.
    let jwks_url = config
        .jwks_url
        .as_deref()
        .ok_or_else(|| AppError::Config("Better Auth JWKS URL is missing".to_owned()))?;

    load_jwks_with(jwks_cache(), jwks_url, force_refresh, || {
        fetch_jwks(jwks_url)
    })
    .await
}

/// Charge un JWKS en appliquant le cache, le cooldown des `kid` inconnus et le singleflight.
async fn load_jwks_with<F, Fut>(
    cache: &JwksCache,
    jwks_url: &str,
    force_refresh: bool,
    fetch: F,
) -> Result<JwkSet, AppError>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<JwkSet, AppError>>,
{
    // Le chemin chaud ne prend qu'un lock de lecture court.
    {
        let state = cache.state.read().await;
        if let Some(keys) = reusable_keys(&state, jwks_url, force_refresh) {
            return Ok(keys);
        }
    }

    // Le guard reste tenu pendant l'I/O: il ne bloque pas les lecteurs du JWKS courant, mais
    // garantit qu'une seule requete reseau represente toutes les demandes concurrentes.
    let _refresh_guard = cache.refresh.lock().await;

    // Une autre tache peut avoir rempli ou rafraichi le cache pendant notre attente du singleflight.
    {
        let state = cache.state.read().await;
        if let Some(keys) = reusable_keys(&state, jwks_url, force_refresh) {
            return Ok(keys);
        }
    }

    // Le fetch se fait hors lock d'etat pour laisser les lecteurs utiliser les cles precedentes.
    let result = fetch().await;
    let completed_at = Instant::now();

    // Toute tentative terminee active le cooldown. En cas d'echec d'un refresh force, le premier
    // appel conserve son erreur, puis les appels repetes echouent fermement sur le JWKS precedent
    // au lieu de marteler le service d'identite.
    let mut state = cache.state.write().await;
    state.last_refresh_attempt = Some(RefreshAttempt {
        jwks_url: jwks_url.to_owned(),
        completed_at,
    });
    if let Ok(keys) = result.as_ref() {
        state.cached = Some(CachedJwkSet {
            jwks_url: jwks_url.to_owned(),
            fetched_at: completed_at,
            keys: keys.clone(),
        });
    }
    drop(state);

    result
}

/// Retourne les cles reutilisables selon le motif de chargement.
fn reusable_keys(state: &JwksCacheState, jwks_url: &str, force_refresh: bool) -> Option<JwkSet> {
    let cached = state
        .cached
        .as_ref()
        .filter(|cached| cached.jwks_url == jwks_url)?;

    if !force_refresh && cached.fetched_at.elapsed() < JWKS_CACHE_TTL {
        return Some(cached.keys.clone());
    }

    // `force_refresh` signale un `kid` absent dans le verifier. Une tentative recente constitue
    // une cache negative globale et bornee: changer continuellement de `kid` ne contourne pas la
    // limitation, tandis qu'une vraie rotation sera retentee apres cinq secondes au plus.
    if force_refresh
        && state.last_refresh_attempt.as_ref().is_some_and(|attempt| {
            attempt.jwks_url == jwks_url
                && attempt.completed_at.elapsed() < UNKNOWN_KID_REFRESH_COOLDOWN
        })
    {
        return Some(cached.keys.clone());
    }

    None
}

/// Retourne la reference statique vers le cache JWKS initialise paresseusement.
fn jwks_cache() -> &'static JwksCache {
    // `get_or_init` evite une initialisation explicite dans `main` et garde le module autonome.
    JWKS_CACHE.get_or_init(JwksCache::default)
}

/// Recupere le JWKS Better Auth et le deserialize dans le type officiel `jsonwebtoken`.
async fn fetch_jwks(jwks_url: &str) -> Result<JwkSet, AppError> {
    // Un client court-vivant suffit ici: le cache evite de construire un client par requete normale.
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;
    // Le status est controle avant la deserialisation pour produire une erreur downstream lisible.
    let response = client.get(jwks_url).send().await?;
    let status = response.status();
    if !status.is_success() {
        return Err(AppError::downstream(format!(
            "Better Auth JWKS endpoint returned HTTP {status}"
        )));
    }

    // `json::<JwkSet>()` valide que Better Auth renvoie une liste de cles publiques exploitable.
    Ok(response.json::<JwkSet>().await?)
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
        time::{Duration, Instant},
    };

    use jsonwebtoken::jwk::JwkSet;
    use serde_json::json;
    use tokio::sync::{Barrier, Notify};

    use super::{
        AppError, CachedJwkSet, JwksCache, RefreshAttempt, UNKNOWN_KID_REFRESH_COOLDOWN,
        load_jwks_with,
    };

    const TEST_JWKS_URL: &str = "https://api.filfinance.test/api/auth/jwks";

    /// Construit un JWKS minimal; ces tests exercent le cache et n'effectuent pas de cryptographie.
    fn jwks_with_kid(kid: &str) -> JwkSet {
        serde_json::from_value(json!({
            "keys": [{
                "kty": "RSA",
                "alg": "RS256",
                "kid": kid,
                "use": "sig",
                "n": "AQAB",
                "e": "AQAB"
            }]
        }))
        .expect("test JWKS should deserialize")
    }

    /// Injecte un cache dont le dernier refresh est assez ancien pour autoriser le suivant.
    async fn seed_refreshable_cache(cache: &JwksCache, keys: JwkSet) {
        let old_attempt = Instant::now()
            .checked_sub(UNKNOWN_KID_REFRESH_COOLDOWN + Duration::from_secs(1))
            .expect("test instant should support subtraction");
        let mut state = cache.state.write().await;
        state.cached = Some(CachedJwkSet {
            jwks_url: TEST_JWKS_URL.to_owned(),
            fetched_at: old_attempt,
            keys,
        });
        state.last_refresh_attempt = Some(RefreshAttempt {
            jwks_url: TEST_JWKS_URL.to_owned(),
            completed_at: old_attempt,
        });
    }

    #[tokio::test]
    async fn concurrent_unknown_kid_refreshes_are_singleflight() {
        const REQUESTS: usize = 16;

        let cache = Arc::new(JwksCache::default());
        seed_refreshable_cache(&cache, jwks_with_kid("old")).await;

        let fetches = Arc::new(AtomicUsize::new(0));
        let start = Arc::new(Barrier::new(REQUESTS + 1));
        let fetch_started = Arc::new(Notify::new());
        let release_fetch = Arc::new(Notify::new());
        let mut tasks = Vec::with_capacity(REQUESTS);

        for _ in 0..REQUESTS {
            let cache = Arc::clone(&cache);
            let fetches = Arc::clone(&fetches);
            let start = Arc::clone(&start);
            let fetch_started = Arc::clone(&fetch_started);
            let release_fetch = Arc::clone(&release_fetch);
            tasks.push(tokio::spawn(async move {
                start.wait().await;
                load_jwks_with(&cache, TEST_JWKS_URL, true, || async move {
                    fetches.fetch_add(1, Ordering::SeqCst);
                    fetch_started.notify_one();
                    release_fetch.notified().await;
                    Ok(jwks_with_kid("rotated"))
                })
                .await
            }));
        }

        start.wait().await;
        fetch_started.notified().await;
        release_fetch.notify_one();

        for task in tasks {
            let keys = task
                .await
                .expect("refresh task should join")
                .expect("coalesced refresh should succeed");
            assert!(keys.find("rotated").is_some());
        }
        assert_eq!(fetches.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn repeated_unknown_kids_are_throttled_then_rotation_is_visible() {
        let cache = JwksCache::default();
        seed_refreshable_cache(&cache, jwks_with_kid("old")).await;
        let fetches = AtomicUsize::new(0);

        // Le premier `kid` inconnu obtient un refresh, mais reste absent du resultat.
        let keys = load_jwks_with(&cache, TEST_JWKS_URL, true, || async {
            fetches.fetch_add(1, Ordering::SeqCst);
            Ok::<_, AppError>(jwks_with_kid("old"))
        })
        .await
        .expect("first unknown-kid refresh should succeed");
        assert!(keys.find("attacker-kid").is_none());

        // Meme avec un autre `kid` arbitraire, le cooldown global ne relance pas le reseau.
        let keys = load_jwks_with(&cache, TEST_JWKS_URL, true, || async {
            fetches.fetch_add(1, Ordering::SeqCst);
            Ok::<_, AppError>(jwks_with_kid("should-not-be-fetched"))
        })
        .await
        .expect("throttled lookup should reuse cached keys");
        assert!(keys.find("should-not-be-fetched").is_none());
        assert_eq!(fetches.load(Ordering::SeqCst), 1);

        // Une fois la courte fenetre expiree, une vraie rotation devient visible au prochain miss.
        {
            let mut state = cache.state.write().await;
            state
                .last_refresh_attempt
                .as_mut()
                .expect("refresh attempt should be recorded")
                .completed_at = Instant::now()
                .checked_sub(UNKNOWN_KID_REFRESH_COOLDOWN + Duration::from_secs(1))
                .expect("test instant should support subtraction");
        }
        let keys = load_jwks_with(&cache, TEST_JWKS_URL, true, || async {
            fetches.fetch_add(1, Ordering::SeqCst);
            Ok::<_, AppError>(jwks_with_kid("rotated"))
        })
        .await
        .expect("post-cooldown rotation refresh should succeed");

        assert!(keys.find("rotated").is_some());
        assert_eq!(fetches.load(Ordering::SeqCst), 2);
    }
}
