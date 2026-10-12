//! Helpers de compteurs et percentiles du runtime TTS.

/// Nombre maximum de durees conservees pour les percentiles runtime TTS.
const RUNTIME_DURATION_SAMPLE_LIMIT: usize = 128;

/// Ajoute une duree recente tout en limitant la memoire conservee par le worker TTS.
pub(super) fn push_duration_sample(samples: &mut Vec<u64>, duration_ms: u64) {
    samples.push(duration_ms);
    if samples.len() > RUNTIME_DURATION_SAMPLE_LIMIT {
        // Les plus vieux samples sortent pour que les percentiles suivent les conditions recentes.
        let overflow = samples.len() - RUNTIME_DURATION_SAMPLE_LIMIT;
        samples.drain(0..overflow);
    }
}

/// Calcule un percentile nearest-rank a partir des durees observees recemment.
pub(super) fn percentile_duration_ms(samples: &[u64], percentile: u64) -> Option<u64> {
    if samples.is_empty() {
        return None;
    }

    // On trie une copie afin de garder l'historique interne dans l'ordre d'arrivee.
    let mut sorted_samples = samples.to_vec();
    sorted_samples.sort_unstable();
    // La valeur retournee reste une duree reelle observee, pratique pour comparer des deploys.
    let rank = ((percentile as f64 / 100.0) * sorted_samples.len() as f64).ceil() as usize;
    let index = rank.saturating_sub(1).min(sorted_samples.len() - 1);
    Some(sorted_samples[index])
}

/// Estime les items non termines a partir des compteurs monotones du worker.
pub(super) fn pending_count(
    received_total: u64,
    completed_total: u64,
    skipped_total: u64,
    failed_total: u64,
) -> u64 {
    received_total.saturating_sub(
        completed_total
            .saturating_add(skipped_total)
            .saturating_add(failed_total),
    )
}
