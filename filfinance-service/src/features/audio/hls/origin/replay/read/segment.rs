//! Lecture des segments replay HLS du jour.

// `fs` lit les segments replay immutables.
use tokio::fs;

use crate::errors::AppError;

use super::super::super::{
    HlsOriginState,
    helpers::{current_local_day, looks_like_local_day, parse_segment_sequence},
};

impl HlsOriginState {
    /// Lit un segment replay du jour par `{local_day}/{sequence}.ts`.
    pub(in crate::features::audio) async fn replay_today_segment_bytes(
        &self,
        local_day: &str,
        segment_name: &str,
    ) -> Result<Vec<u8>, AppError> {
        // HLS désactivé donne une erreur claire au lieu de tomber sur le disque.
        if !self.enabled {
            return Err(AppError::not_found("hls replay origin is disabled"));
        }

        // Le contrat replay ne sert que le jour courant; la veille doit disparaître à minuit.
        if local_day != current_local_day() {
            return Err(AppError::not_found("hls replay day is not current"));
        }

        // Le jour en URL reste strict pour éviter toute traversée de répertoire.
        if !looks_like_local_day(local_day) {
            return Err(AppError::not_found("hls replay day is invalid"));
        }

        // La séquence garde une validation stricte pour éviter toute URL ambiguë.
        let sequence = parse_segment_sequence(segment_name)?;
        // Le chemin est résolu depuis l'index mémoire, jamais depuis l'URL brute.
        let file_path = {
            let inner = self.inner.lock().expect("hls origin mutex poisoned");
            inner
                .segments
                .iter()
                .find(|segment| segment.local_day == local_day && segment.sequence == sequence)
                .map(|segment| segment.file_path.clone())
                .ok_or_else(|| {
                    AppError::not_found(format!("hls replay segment not found: {sequence}"))
                })?
        };

        // L'octet servi est un segment immutable du replay explicite.
        let bytes = fs::read(file_path).await?;
        Ok(bytes)
    }
}
