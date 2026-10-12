//! Taxonomie d'erreurs commune au service et adaptation en reponses HTTP.

// `Json`, `StatusCode`, `IntoResponse` et `Response` viennent d'`axum`. Ils servent a traduire
// une erreur Rust interne en reponse HTTP exploitable par un client.
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
// `json!` construit facilement un objet JSON sans definir une struct dediee pour un payload tres simple.
use serde_json::json;
// `thiserror::Error` genere automatiquement les implementations d'erreur idiomatiques de Rust.
use thiserror::Error;

/// Enum centrale des erreurs remontees par le service.
// `#[derive(Debug, Error)]` donne a l'enum:
// - `Debug` pour l'affichage technique en debug,
// - `Error` pour l'integration avec l'ecosysteme standard des erreurs Rust.
#[derive(Debug, Error)]
pub enum AppError {
    // Variante simple contenant un message libre pour les erreurs de configuration.
    #[error("configuration error: {0}")]
    Config(String),
    // Variante dediee aux requetes sans session Better Auth valide.
    #[error("unauthorized: {0}")]
    Unauthorized(String),
    // Variante dediee aux sessions valides mais insuffisantes pour l'action demandee.
    #[error("forbidden: {0}")]
    Forbidden(String),
    // Variante pour les ressources absentes, utile surtout sur l'API HTTP.
    #[error("not found: {0}")]
    NotFound(String),
    // Variante pour les actions concurrentes deja en cours.
    #[error("conflict: {0}")]
    Conflict(String),
    // Variante structuree avec des champs nommes pour enrichir le contexte de fetch.
    #[error("fetch error for source {source_name}: {message}")]
    Fetch {
        source_name: String,
        message: String,
    },
    // Meme logique pour les erreurs de parsing issues d'une source externe.
    #[error("parse error for source {source_name}: {message}")]
    Parse {
        source_name: String,
        message: String,
    },
    // Variante applicative simple pour les erreurs de stockage non couvertes par SQLx.
    #[error("storage error: {0}")]
    Storage(String),
    // Variante reservee aux consumers lents ou distants comme Telegram.
    #[error("downstream error: {0}")]
    Downstream(String),
    // `#[from]` dit a `thiserror` de generer `From<sqlx::Error> for AppError`.
    #[error("database error: {0}")]
    Sqlx(#[from] sqlx::Error),
    // Meme mecanisme pour transformer automatiquement une erreur `reqwest` en `AppError`.
    #[error("http error: {0}")]
    Reqwest(#[from] reqwest::Error),
    // Meme mecanisme pour les erreurs d'I/O du systeme.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    // Meme mecanisme pour les erreurs JSON.
    #[error("json error: {0}")]
    SerdeJson(#[from] serde_json::Error),
}

impl AppError {
    #[allow(dead_code)]
    /// Construit une erreur de fetch en conservant la source concernee.
    pub fn fetch(source: impl Into<String>, message: impl Into<String>) -> Self {
        // `impl Into<String>` rend l'API souple: on peut passer un `&str` ou une `String`.
        Self::Fetch {
            // `.into()` convertit la valeur dans le type final attendu.
            source_name: source.into(),
            message: message.into(),
        }
    }

    #[allow(dead_code)]
    /// Construit une erreur de parsing rattachee a une source.
    pub fn parse(source: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Parse {
            source_name: source.into(),
            message: message.into(),
        }
    }

    #[allow(dead_code)]
    /// Construit une erreur de stockage purement applicative.
    pub fn storage(message: impl Into<String>) -> Self {
        // Ici l'enum compacte simplement le message dans sa variante associee.
        Self::Storage(message.into())
    }

    #[allow(dead_code)]
    /// Construit une erreur `401` quand la session Better Auth est absente ou invalide.
    pub fn unauthorized(message: impl Into<String>) -> Self {
        // Le message reste volontairement court pour ne pas exposer de details cryptographiques.
        Self::Unauthorized(message.into())
    }

    #[allow(dead_code)]
    /// Construit une erreur `403` quand l'utilisateur authentifie n'a pas le role requis.
    pub fn forbidden(message: impl Into<String>) -> Self {
        // La distinction avec `unauthorized` aide le client a differencier login et droits.
        Self::Forbidden(message.into())
    }

    #[allow(dead_code)]
    /// Construit une erreur `404` applicative pour les ressources de debug absentes.
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::NotFound(message.into())
    }

    #[allow(dead_code)]
    /// Construit une erreur `409` quand une action equivalente est deja en cours.
    pub fn conflict(message: impl Into<String>) -> Self {
        Self::Conflict(message.into())
    }

    #[allow(dead_code)]
    /// Construit une erreur de downstream pour les sorties lentes ou distantes.
    pub fn downstream(message: impl Into<String>) -> Self {
        Self::Downstream(message.into())
    }
}

// `IntoResponse` est le trait `axum` qui dit "ce type sait se convertir en reponse HTTP".
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        // Le mapping HTTP reste simple: configuration et payload invalide sont differencies,
        // tout le reste est traite comme une erreur serveur.
        // Le `match` est exhaustif: chaque variante de l'enum doit etre traitee explicitement.
        let status = match &self {
            Self::Config(_) => StatusCode::BAD_REQUEST,
            Self::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            Self::Forbidden(_) => StatusCode::FORBIDDEN,
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::Parse { .. } => StatusCode::UNPROCESSABLE_ENTITY,
            Self::Fetch { .. }
            | Self::Storage(_)
            | Self::Downstream(_)
            | Self::Sqlx(_)
            | Self::Reqwest(_)
            | Self::Io(_)
            | Self::SerdeJson(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        // Le corps JSON conserve le message exact pour faciliter le debug cote appelant.
        // `self.to_string()` reutilise le texte defini par les attributs `#[error(...)]` ci-dessus.
        let body = Json(json!({
            "error": self.to_string(),
        }));

        // Le tuple `(status, body)` implemente lui aussi `IntoResponse`, ce qui simplifie beaucoup
        // l'adaptation HTTP sans creer de constructeur manuel de `Response`.
        (status, body).into_response()
    }
}
