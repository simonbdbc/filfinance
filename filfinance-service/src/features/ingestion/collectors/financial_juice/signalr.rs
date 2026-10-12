//! Facade SignalR FinancialJuice.

// `dto` porte les enveloppes JSON et la session négociée.
mod dto;
// `transport` porte `negotiate` et `start`.
mod transport;
// `url` porte la construction des URLs SignalR.
mod url;

pub(super) use dto::FinancialJuiceSignalrEnvelope;
pub(super) use url::build_signalr_connect_url;
