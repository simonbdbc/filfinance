//! Construction des URLs SignalR FinancialJuice.

// `Url` construit l'adresse WebSocket finale après négociation SignalR.
use reqwest::Url;

use crate::errors::AppError;

use super::dto::FinancialJuiceSignalrTransportSession;
use crate::features::ingestion::collectors::financial_juice::{
    FINANCIAL_JUICE_SIGNALR_CLIENT_PROTOCOL, FINANCIAL_JUICE_SIGNALR_CONNECTION_DATA,
};

/// Construit l'URL `wss://.../connect` complète pour Azure SignalR / ASP.NET SignalR.
pub(in crate::features::ingestion::collectors::financial_juice) fn build_signalr_connect_url(
    session: &FinancialJuiceSignalrTransportSession,
) -> Result<Url, AppError> {
    // Le transport HTTP négocié doit être converti en `wss://` pour `connect_async`.
    let base = session.transport_base_url.replace("https://", "wss://");
    let mut url = Url::parse(&format!("{base}/connect"))
        .map_err(|error| AppError::downstream(format!("invalid signalr websocket URL: {error}")))?;
    // `query_pairs_mut()` permet de muter la query string déjà encodée sans reconstruire l'URL à la main.
    url.query_pairs_mut()
        .append_pair("transport", "webSockets")
        .append_pair("clientProtocol", FINANCIAL_JUICE_SIGNALR_CLIENT_PROTOCOL)
        .append_pair("connectionToken", &session.connection_token)
        .append_pair("connectionData", FINANCIAL_JUICE_SIGNALR_CONNECTION_DATA)
        .append_pair("tid", "8")
        .append_pair("ftoken", &session.feedtoken);

    if let Some(access_token) = session.access_token.as_deref() {
        // `as_deref()` garde un `&str` emprunté plutôt qu'une nouvelle `String` clonée.
        url.query_pairs_mut()
            .append_pair("access_token", access_token);
    }

    Ok(url)
}

/// Recompose une base SignalR quand un sous-chemin relatif est renvoyé par `negotiate`.
pub(super) fn join_signalr_base(base: &str, path: &str) -> String {
    let base = base.trim_end_matches('/');
    let path = path.trim_start_matches('/');

    if path.is_empty() {
        return base.to_owned();
    }

    // Azure SignalR peut renvoyer `Url=/aspnetclient` alors que `RedirectUrl` contient déjà
    // `.../aspnetclient`. Dans ce cas, il ne faut surtout pas redoubler le segment.
    if base.ends_with(&format!("/{path}")) {
        return base.to_owned();
    }

    format!("{base}/{path}")
}
