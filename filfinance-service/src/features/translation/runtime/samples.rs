//! Calculs bornes sur les samples runtime de traduction.

/// Nombre maximum de durees gardees pour calculer des percentiles runtime bornes.
const RUNTIME_DURATION_SAMPLE_LIMIT: usize = 128;

/// Ajoute une duree recente tout en gardant une memoire bornee pour le hot path.
pub(super) fn push_duration_sample(samples: &mut Vec<u64>, duration_ms: u64) {
    samples.push(duration_ms);
    if samples.len() > RUNTIME_DURATION_SAMPLE_LIMIT {
        // On retire les samples les plus anciens pour que les percentiles restent representatifs du recent.
        let overflow = samples.len() - RUNTIME_DURATION_SAMPLE_LIMIT;
        samples.drain(0..overflow);
    }
}

/// Calcule un percentile nearest-rank sur une copie triee des samples recents.
pub(super) fn percentile_duration_ms(samples: &[u64], percentile: u64) -> Option<u64> {
    if samples.is_empty() {
        return None;
    }

    // Le tri est fait sur une copie pour ne pas changer l'ordre temporel conserve en memoire.
    let mut sorted_samples = samples.to_vec();
    sorted_samples.sort_unstable();
    // La methode nearest-rank donne une valeur observee, plus simple a lire dans les logs runtime.
    let rank = ((percentile as f64 / 100.0) * sorted_samples.len() as f64).ceil() as usize;
    let index = rank.saturating_sub(1).min(sorted_samples.len() - 1);
    Some(sorted_samples[index])
}

/// Estime les items non termines a partir des compteurs monotones disponibles.
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
