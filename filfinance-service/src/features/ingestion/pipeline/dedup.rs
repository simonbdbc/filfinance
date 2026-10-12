//! Deduplication memoire bornee du pipeline d'ingestion.

// Le cache de signatures est indexe par chaine et date par `Instant` monotone.
use std::collections::HashMap;
// `Duration` exprime le TTL sans conversion ad hoc dans le hot path.
use std::time::{Duration, Instant};

/// Purge les signatures trop anciennes pour garder un cache borne dans le temps.
pub(super) fn evict_expired(cache: &mut HashMap<String, Instant>, ttl: Duration) {
    // `retain` garde seulement les entrees qui satisfont la predicate, sans reconstruire une nouvelle map.
    cache.retain(|_, inserted_at| inserted_at.elapsed() < ttl);
}
