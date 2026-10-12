//! Lecture typee des variables d'environnement `NEWS_*`.

// Le module standard donne acces a l'environnement du process courant.
use std::env;

// Les erreurs de parsing remontent comme erreurs de configuration applicative.
use crate::errors::AppError;

/// Lit une variable texte avec valeur par defaut.
pub(super) fn env_or(name: &str, default: &str) -> String {
    // `unwrap_or_else` evite de construire le defaut si la variable existe deja.
    env::var(name).unwrap_or_else(|_| default.to_owned())
}

/// Lit une variable optionnelle en normalisant la chaine vide vers `None`.
pub(super) fn env_optional(name: &str) -> Option<String> {
    // `ok()` convertit `Result<String, VarError>` en `Option<String>` en oubliant seulement le detail
    // d'absence/erreur, ce qui est acceptable ici pour un helper optionnel.
    env::var(name).ok().and_then(|value| {
        // On normalise les espaces avant de decider si la valeur est exploitable.
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_owned())
        }
    })
}

/// Convertit une liste CSV d'environnement en vecteur propre.
pub(super) fn env_csv(name: &str) -> Vec<String> {
    // `unwrap_or_default()` retourne une chaine vide si la variable est absente.
    env::var(name)
        .unwrap_or_default()
        // `split(',')` decoupe la liste brute.
        .split(',')
        // `map(str::trim)` nettoie les espaces inutiles.
        .map(str::trim)
        // On ignore les elements vides pour eviter des IDs de chat ou des args fantomes.
        .filter(|value| !value.is_empty())
        // `ToOwned::to_owned` produit des `String` possedees a partir de `&str`.
        .map(ToOwned::to_owned)
        .collect()
}

/// Parse un bool applicatif avec un vocabulaire explicite.
pub(super) fn env_bool(name: &str, default: bool) -> Result<bool, AppError> {
    // `match env::var(name)` permet de distinguer:
    // - variable presente
    // - variable absente
    // - autre erreur de lecture
    match env::var(name) {
        // Si la variable existe, on accepte plusieurs synonymes usuels pour reduire la friction locale.
        Ok(value) => match value.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Ok(true),
            "0" | "false" | "no" | "off" => Ok(false),
            // Toute autre valeur est refusee explicitement pour eviter les faux positifs silencieux.
            other => Err(AppError::Config(format!(
                "invalid boolean value for {name}: {other}"
            ))),
        },
        // `NotPresent` signifie "utilise le defaut" et non "erreur fatale".
        Err(env::VarError::NotPresent) => Ok(default),
        // Les autres erreurs, par exemple Unicode invalide, sont remontees comme vraies erreurs de config.
        Err(error) => Err(AppError::Config(format!("failed to read {name}: {error}"))),
    }
}

/// Parse une taille en `usize` pour les buffers et limites memoire.
pub(super) fn env_usize(name: &str, default: usize) -> Result<usize, AppError> {
    match env::var(name) {
        // `parse::<usize>()` force ici un entier positif adapte aux tailles/index memoire en Rust.
        Ok(value) => value
            .trim()
            .parse::<usize>()
            .map_err(|error| AppError::Config(format!("invalid usize for {name}: {error}"))),
        Err(env::VarError::NotPresent) => Ok(default),
        Err(error) => Err(AppError::Config(format!("failed to read {name}: {error}"))),
    }
}

/// Parse une duree ou un compteur exprime en `u64`.
pub(super) fn env_u64(name: &str, default: u64) -> Result<u64, AppError> {
    match env::var(name) {
        // `u64` convient bien aux millisecondes, timestamps et compteurs non signes.
        Ok(value) => value
            .trim()
            .parse::<u64>()
            .map_err(|error| AppError::Config(format!("invalid u64 for {name}: {error}"))),
        Err(env::VarError::NotPresent) => Ok(default),
        Err(error) => Err(AppError::Config(format!("failed to read {name}: {error}"))),
    }
}

/// Parse un entier `u32` pour les limites de pool.
pub(super) fn env_u32(name: &str, default: u32) -> Result<u32, AppError> {
    match env::var(name) {
        // `u32` suffit pour les tailles de pool, frequences et compteurs modestes.
        Ok(value) => value
            .trim()
            .parse::<u32>()
            .map_err(|error| AppError::Config(format!("invalid u32 for {name}: {error}"))),
        Err(env::VarError::NotPresent) => Ok(default),
        Err(error) => Err(AppError::Config(format!("failed to read {name}: {error}"))),
    }
}

/// Parse un entier signe `i32`, utile pour les offsets de fuseau horaire en minutes.
pub(super) fn env_i32(name: &str, default: i32) -> Result<i32, AppError> {
    match env::var(name) {
        Ok(value) => value
            .trim()
            .parse::<i32>()
            .map_err(|error| AppError::Config(format!("invalid i32 for {name}: {error}"))),
        Err(env::VarError::NotPresent) => Ok(default),
        Err(error) => Err(AppError::Config(format!("failed to read {name}: {error}"))),
    }
}
