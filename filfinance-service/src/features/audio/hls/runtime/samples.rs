//! Fenetre bornee et percentiles des durees runtime HLS.

/// Nombre maximum de durees conservees pour calculer les percentiles runtime HLS.
const RUNTIME_DURATION_SAMPLE_LIMIT: usize = 128;

/// Ajoute une duree runtime tout en bornant la memoire utilisee par l'origin.
pub(in crate::features::audio::hls) fn push_duration_sample(
    samples: &mut Vec<u64>,
    duration_ms: u64,
) {
    samples.push(duration_ms);
    if samples.len() > RUNTIME_DURATION_SAMPLE_LIMIT {
        // Les samples les plus anciens sortent pour que les percentiles suivent les derniers deploys.
        let overflow = samples.len() - RUNTIME_DURATION_SAMPLE_LIMIT;
        samples.drain(0..overflow);
    }
}

/// Calcule un percentile nearest-rank a partir des durees observees recemment.
pub(in crate::features::audio::hls) fn percentile_duration_ms(
    samples: &[u64],
    percentile: u64,
) -> Option<u64> {
    if samples.is_empty() {
        return None;
    }

    // Le tri s'applique a une copie pour conserver l'ordre temporel dans l'etat runtime.
    let mut sorted_samples = samples.to_vec();
    sorted_samples.sort_unstable();
    // La valeur retournee est une duree effectivement mesuree, lisible dans les endpoints debug.
    let rank = ((percentile as f64 / 100.0) * sorted_samples.len() as f64).ceil() as usize;
    let index = rank.saturating_sub(1).min(sorted_samples.len() - 1);
    Some(sorted_samples[index])
}
