//! DTO SignalR FinancialJuice.

// `Deserialize` parse les DTO SignalR et `Value` porte les arguments de hub dynamiques.
use serde::Deserialize;
use serde_json::Value;

/// Négociation SignalR renvoyée par l'application FinancialJuice.
#[derive(Debug, Deserialize)]
pub(super) struct FinancialJuiceSignalrAppNegotiateResponse {
    /// Redirection Azure SignalR, présente dans le cas réel observé.
    #[serde(rename = "RedirectUrl")]
    pub(super) redirect_url: Option<String>,
    /// Access token à présenter au service Azure SignalR.
    #[serde(rename = "AccessToken")]
    pub(super) access_token: Option<String>,
    /// Cas alternatif d'un endpoint non redirigé.
    #[serde(rename = "ConnectionToken")]
    pub(super) connection_token: Option<String>,
    /// URL SignalR retournée quand il n'y a pas de redirection.
    #[serde(rename = "Url")]
    pub(super) url: Option<String>,
}

/// Seconde négociation SignalR après redirection Azure.
#[derive(Debug, Deserialize)]
pub(super) struct FinancialJuiceSignalrTransportNegotiateResponse {
    /// Token de connexion SignalR réellement utilisé pour `connect/start`.
    #[serde(rename = "ConnectionToken")]
    pub(super) connection_token: String,
    /// Sous-chemin SignalR éventuellement retourné par le transport.
    #[serde(rename = "Url")]
    pub(super) url: Option<String>,
}

/// Session de transport live complètement négociée.
#[derive(Debug, Clone)]
pub(in crate::features::ingestion::collectors::financial_juice) struct FinancialJuiceSignalrTransportSession
{
    /// Base finale de transport, par exemple `https://.../aspnetclient`.
    pub(super) transport_base_url: String,
    /// Token SignalR à placer dans `connect` et `start`.
    pub(super) connection_token: String,
    /// Access token Azure SignalR si le service redirige.
    pub(super) access_token: Option<String>,
    /// `feedtoken` applicatif à retransmettre jusque dans le live.
    pub(super) feedtoken: String,
}

/// Enveloppe JSON des frames SignalR côté serveur.
#[derive(Debug, Deserialize)]
pub(in crate::features::ingestion::collectors::financial_juice) struct FinancialJuiceSignalrEnvelope
{
    /// Curseur SignalR, utile au protocole mais sans intérêt métier ici.
    #[serde(rename = "C")]
    #[allow(dead_code)]
    cursor: Option<String>,
    /// Message d'erreur serveur éventuel.
    #[serde(rename = "E")]
    pub(in crate::features::ingestion::collectors::financial_juice) error: Option<String>,
    /// Invocations de hub embarquées dans la frame.
    #[serde(rename = "M", default)]
    pub(in crate::features::ingestion::collectors::financial_juice) messages:
        Vec<FinancialJuiceSignalrHubInvocation>,
}

/// Invocation d'un hub SignalR.
#[derive(Debug, Deserialize)]
pub(in crate::features::ingestion::collectors::financial_juice) struct FinancialJuiceSignalrHubInvocation
{
    /// Nom du hub ciblé.
    #[serde(rename = "H")]
    pub(in crate::features::ingestion::collectors::financial_juice) hub: Option<String>,
    /// Nom de la méthode appelée côté client.
    #[serde(rename = "M")]
    pub(in crate::features::ingestion::collectors::financial_juice) method: Option<String>,
    /// Arguments JSON de l'invocation.
    #[serde(rename = "A", default)]
    pub(in crate::features::ingestion::collectors::financial_juice) args: Vec<Value>,
}
