//! Estimation des durees WAV et segments HLS.

/// Estime la duree totale d'un WAV PCM a partir des chunks `fmt ` et `data`.
pub(in crate::features::audio) fn estimate_wav_duration_secs(bytes: &[u8]) -> Option<f64> {
    // Un WAV minimal porte `RIFF....WAVE`; sans cela on laisse le caller utiliser le fallback.
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return None;
    }

    let mut cursor = 12usize;
    let mut byte_rate = None;
    let mut data_size = None;

    while cursor + 8 <= bytes.len() {
        let chunk_id = &bytes[cursor..cursor + 4];
        let chunk_size = u32::from_le_bytes([
            bytes[cursor + 4],
            bytes[cursor + 5],
            bytes[cursor + 6],
            bytes[cursor + 7],
        ]) as usize;
        let chunk_start = cursor + 8;
        let chunk_end = chunk_start.saturating_add(chunk_size).min(bytes.len());

        if chunk_id == b"fmt " && chunk_start + 12 <= chunk_end {
            byte_rate = Some(u32::from_le_bytes([
                bytes[chunk_start + 8],
                bytes[chunk_start + 9],
                bytes[chunk_start + 10],
                bytes[chunk_start + 11],
            ]));
        }

        if chunk_id == b"data" {
            data_size = Some(chunk_size as u32);
        }

        cursor = chunk_start + chunk_size + (chunk_size % 2);
    }

    let byte_rate = byte_rate.filter(|value| *value > 0)?;
    let data_size = data_size?;
    Some(data_size as f64 / byte_rate as f64)
}

/// Estime la duree d'un segment produit par le segmenter HLS.
pub(in crate::features::audio::hls::origin) fn estimate_segment_duration(
    total_duration_secs: Option<f64>,
    segment_index: usize,
    segment_target_duration_secs: u32,
) -> f64 {
    // Sans duree WAV fiable, on annonce la duree cible pour rester lisible par le player.
    let Some(total_duration_secs) = total_duration_secs else {
        return segment_target_duration_secs.max(1) as f64;
    };
    let target = segment_target_duration_secs.max(1) as f64;
    let consumed_before = segment_index as f64 * target;
    let remaining = (total_duration_secs - consumed_before).max(0.0);

    if remaining <= 0.0 {
        target
    } else {
        remaining.min(target).max(0.1)
    }
}
