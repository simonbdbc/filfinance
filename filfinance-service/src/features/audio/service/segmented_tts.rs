//! Synthese TTS segmentee pour les corps longs.

use crate::{
    errors::AppError,
    features::audio::tts::provider::{SynthesizedAudio, TtsProvider, TtsRequest},
};

/// Audio genere en plusieurs passages Piper, puis recompresse en un seul WAV.
pub(in crate::features::audio::service) struct SegmentedSynthesizedAudio {
    /// WAV final compatible avec les appels HLS existants.
    pub audio: SynthesizedAudio,
    /// Nombre de segments TTS effectivement envoyes a Piper.
    pub segment_count: u64,
}

/// Synthétise un texte de corps en plusieurs segments de ponctuation.
pub(in crate::features::audio::service) async fn synthesize_punctuation_segmented(
    provider: &mut TtsProvider,
    text: &str,
) -> Result<SegmentedSynthesizedAudio, AppError> {
    let segments = split_text_on_tts_punctuation(text);
    crate::features::calendar_logs::event("tts.segments", serde_json::json!({"segments":segments}));
    let mut audios = Vec::with_capacity(segments.len());

    for segment in &segments {
        audios.push(
            provider
                .synthesize(&TtsRequest {
                    text: segment.clone(),
                })
                .await?,
        );
    }

    let segment_count = audios.len() as u64;
    let audio = concatenate_synthesized_wavs(audios)?;
    crate::features::calendar_logs::event(
        "tts.concatenated",
        serde_json::json!({"segment_count":segment_count,"bytes":audio.bytes.len(),"format":audio.audio_format}),
    );
    Ok(SegmentedSynthesizedAudio {
        audio,
        segment_count,
    })
}

/// Découpe uniquement sur les signes de ponctuation demandes.
fn split_text_on_tts_punctuation(text: &str) -> Vec<String> {
    let mut segments = Vec::new();
    let mut current = String::new();
    let characters = text.chars().collect::<Vec<_>>();

    for (index, character) in characters.iter().copied().enumerate() {
        current.push(character);
        if is_tts_segment_separator(
            character,
            index
                .checked_sub(1)
                .and_then(|previous| characters.get(previous)),
            characters.get(index + 1),
        ) {
            push_segment(&mut segments, &mut current);
        }
    }

    push_segment(&mut segments, &mut current);

    if segments.is_empty() {
        vec![text.trim().to_owned()]
    } else {
        segments
    }
}

fn push_segment(segments: &mut Vec<String>, current: &mut String) {
    let segment = current.trim();
    if segment_has_spoken_content(segment) {
        segments.push(segment.to_owned());
    }
    current.clear();
}

fn segment_has_spoken_content(segment: &str) -> bool {
    segment.chars().any(char::is_alphanumeric)
}

fn is_tts_segment_separator(character: char, previous: Option<&char>, next: Option<&char>) -> bool {
    match character {
        '.' | ',' => !matches!(
            (previous, next),
            (Some(previous), Some(next))
                if previous.is_ascii_digit() && next.is_ascii_digit()
        ),
        ';' | '?' | '!' => true,
        _ => false,
    }
}

fn concatenate_synthesized_wavs(
    mut audios: Vec<SynthesizedAudio>,
) -> Result<SynthesizedAudio, AppError> {
    if audios.is_empty() {
        return Err(AppError::downstream(
            "segmented tts did not produce any audio".to_owned(),
        ));
    }

    if audios.len() == 1 {
        return Ok(audios.remove(0));
    }

    let first = audios.remove(0);
    let first_wav = ParsedWav::parse(&first.bytes)?;
    let mut data = first_wav.data;
    let fmt = first_wav.fmt;

    for audio in audios {
        let parsed = ParsedWav::parse(&audio.bytes)?;
        if parsed.fmt != fmt {
            return Err(AppError::downstream(
                "cannot concatenate Piper WAV chunks with different formats".to_owned(),
            ));
        }
        data.extend(parsed.data);
    }

    Ok(SynthesizedAudio {
        bytes: build_wav(&fmt, &data)?,
        audio_format: first.audio_format,
        mime_type: first.mime_type,
        voice: first.voice,
    })
}

struct ParsedWav {
    fmt: Vec<u8>,
    data: Vec<u8>,
}

impl ParsedWav {
    fn parse(bytes: &[u8]) -> Result<Self, AppError> {
        if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
            return Err(AppError::downstream(
                "Piper returned a non-WAV payload".to_owned(),
            ));
        }

        let mut cursor = 12usize;
        let mut fmt = None;
        let mut data = Vec::new();

        while cursor + 8 <= bytes.len() {
            let chunk_id = &bytes[cursor..cursor + 4];
            let chunk_size = u32::from_le_bytes([
                bytes[cursor + 4],
                bytes[cursor + 5],
                bytes[cursor + 6],
                bytes[cursor + 7],
            ]) as usize;
            let chunk_start = cursor + 8;
            let chunk_end = chunk_start.saturating_add(chunk_size);
            if chunk_end > bytes.len() {
                return Err(AppError::downstream(
                    "Piper returned a truncated WAV chunk".to_owned(),
                ));
            }

            if chunk_id == b"fmt " {
                fmt = Some(bytes[chunk_start..chunk_end].to_vec());
            } else if chunk_id == b"data" {
                data.extend_from_slice(&bytes[chunk_start..chunk_end]);
            }

            cursor = chunk_end + (chunk_size % 2);
        }

        let fmt = fmt.ok_or_else(|| AppError::downstream("WAV fmt chunk missing".to_owned()))?;
        if data.is_empty() {
            return Err(AppError::downstream("WAV data chunk missing".to_owned()));
        }

        Ok(Self { fmt, data })
    }
}

fn build_wav(fmt: &[u8], data: &[u8]) -> Result<Vec<u8>, AppError> {
    let fmt_padding = fmt.len() % 2;
    let data_padding = data.len() % 2;
    let riff_size = 4usize
        .saturating_add(8 + fmt.len() + fmt_padding)
        .saturating_add(8 + data.len() + data_padding);
    let riff_size = u32::try_from(riff_size)
        .map_err(|_| AppError::downstream("concatenated WAV payload is too large".to_owned()))?;
    let data_len = u32::try_from(data.len())
        .map_err(|_| AppError::downstream("concatenated WAV data chunk is too large".to_owned()))?;
    let fmt_len = u32::try_from(fmt.len())
        .map_err(|_| AppError::downstream("WAV fmt chunk is too large".to_owned()))?;

    let mut output = Vec::with_capacity(riff_size as usize + 8);
    output.extend_from_slice(b"RIFF");
    output.extend_from_slice(&riff_size.to_le_bytes());
    output.extend_from_slice(b"WAVE");
    output.extend_from_slice(b"fmt ");
    output.extend_from_slice(&fmt_len.to_le_bytes());
    output.extend_from_slice(fmt);
    if fmt_padding == 1 {
        output.push(0);
    }
    output.extend_from_slice(b"data");
    output.extend_from_slice(&data_len.to_le_bytes());
    output.extend_from_slice(data);
    if data_padding == 1 {
        output.push(0);
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_text_uses_only_punctuation_boundaries() {
        let segments = split_text_on_tts_punctuation(
            "Premier segment, deuxieme segment sans limite dure ni espaces",
        );

        assert_eq!(
            segments,
            vec![
                "Premier segment,".to_owned(),
                "deuxieme segment sans limite dure ni espaces".to_owned()
            ]
        );
    }

    #[test]
    fn split_text_keeps_long_segment_without_space_or_hard_split() {
        let segments = split_text_on_tts_punctuation("un deux trois quatre cinq");

        assert_eq!(segments, vec!["un deux trois quatre cinq".to_owned()]);
    }

    #[test]
    fn split_text_drops_punctuation_only_segments() {
        let segments = split_text_on_tts_punctuation("... Marches en hausse... Fed stable.");

        assert_eq!(
            segments,
            vec!["Marches en hausse.".to_owned(), "Fed stable.".to_owned()]
        );
    }

    #[test]
    fn split_text_keeps_decimal_numbers_inside_segments() {
        let segments = split_text_on_tts_punctuation("CPI at 1.62%, puis 5,2 en Europe.");

        assert_eq!(
            segments,
            vec!["CPI at 1.62%,".to_owned(), "puis 5,2 en Europe.".to_owned()]
        );
    }

    #[test]
    fn split_text_never_uses_dashes_as_boundaries() {
        let segments = split_text_on_tts_punctuation("EUR-USD reste stable. Fed - BCE divergent.");

        assert_eq!(
            segments,
            vec![
                "EUR-USD reste stable.".to_owned(),
                "Fed - BCE divergent.".to_owned()
            ]
        );
    }

    #[test]
    fn split_text_keeps_iso_dates_inside_segments() {
        let segments = split_text_on_tts_punctuation("2026-06-09: marche calme.");

        assert_eq!(segments, vec!["2026-06-09: marche calme.".to_owned()]);
    }

    #[test]
    fn concatenate_wavs_preserves_a_single_wav_contract() {
        let first = fake_wav(&[1, 2, 3, 4]);
        let second = fake_wav(&[5, 6]);
        let audio = concatenate_synthesized_wavs(vec![
            fake_audio(first, "voice-a"),
            fake_audio(second, "voice-a"),
        ])
        .expect("compatible wav chunks should concatenate");
        let parsed = ParsedWav::parse(&audio.bytes).expect("result should be wav");

        assert_eq!(parsed.data, vec![1, 2, 3, 4, 5, 6]);
        assert_eq!(audio.voice, "voice-a");
    }

    fn fake_audio(bytes: Vec<u8>, voice: &str) -> SynthesizedAudio {
        SynthesizedAudio {
            bytes,
            audio_format: "wav".to_owned(),
            mime_type: "audio/wav".to_owned(),
            voice: voice.to_owned(),
        }
    }

    fn fake_wav(data: &[u8]) -> Vec<u8> {
        let fmt = vec![1, 0, 1, 0, 0x44, 0xac, 0, 0, 0x88, 0x58, 1, 0, 2, 0, 16, 0];
        build_wav(&fmt, data).expect("fake wav should build")
    }
}
