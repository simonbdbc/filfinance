//! Conversion des réponses de négociation SignalR en session de transport.

use crate::errors::AppError;

use super::super::super::{
    dto::{FinancialJuiceSignalrTransportNegotiateResponse, FinancialJuiceSignalrTransportSession},
    url::join_signalr_base,
};

/// Construit la session finale du cas redirigé Azure SignalR.
pub(super) fn redirected_transport_session(
    redirect_url: &str,
    access_token: &str,
    response: FinancialJuiceSignalrTransportNegotiateResponse,
    feedtoken: &str,
) -> FinancialJuiceSignalrTransportSession {
    // La base est normalisée avant d'appliquer l'éventuel sous-chemin renvoyé par Azure SignalR.
    let transport_base_url = redirect_url.trim_end_matches('/').to_owned();
    // Certains endpoints renvoient un sous-chemin relatif type `/aspnetclient`; on le recolle
    // proprement à la base négociée sans dupliquer de segments.
    let transport_base_url = if let Some(path) = response.url.as_deref() {
        join_signalr_base(&transport_base_url, path)
    } else {
        transport_base_url
    };

    FinancialJuiceSignalrTransportSession {
        transport_base_url,
        connection_token: response.connection_token,
        access_token: Some(access_token.to_owned()),
        feedtoken: feedtoken.to_owned(),
    }
}

/// Construit la session fallback quand l'application renvoie directement un `connectionToken`.
pub(super) fn direct_transport_session(
    main_url: &str,
    response_url: Option<String>,
    connection_token: String,
    feedtoken: &str,
) -> FinancialJuiceSignalrTransportSession {
    // Sans URL explicite, SignalR utilise le chemin standard `/signalr` sur l'application.
    let transport_base_url = if let Some(path) = response_url.as_deref() {
        join_signalr_base(main_url, path)
    } else {
        format!("{main_url}/signalr")
    };

    FinancialJuiceSignalrTransportSession {
        transport_base_url,
        connection_token,
        access_token: None,
        feedtoken: feedtoken.to_owned(),
    }
}

/// Erreur commune quand aucune variante de négociation n'est exploitable.
pub(super) fn unusable_transport_error(source_name: &str) -> AppError {
    // Le message reste stable pour ne pas changer les diagnostics existants.
    AppError::fetch(
        source_name.to_owned(),
        "signalr negotiate did not return a usable transport".to_owned(),
    )
}
