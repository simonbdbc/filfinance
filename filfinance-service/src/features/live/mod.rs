//! Feature `live`, responsable des WebSockets, du control plane edge et des runs E2E.

// `dto` reserve le futur emplacement des payloads live separes du domaine news.
pub(crate) mod dto;
// `edge` publie les metadata live vers Cloudflare Durable Objects.
pub(crate) mod edge;
// `fanout` porte Redis + WebSocket direct pour Bunny DEV, sans toucher au Durable Object prod.
pub(crate) mod fanout;
// `test_runs` prepare les evenements synthetiques E2E.
pub(crate) mod test_runs;
// `websocket` expose les flux temps reel backend.
pub(crate) mod websocket;
// Les tests live communs restent separes des tests websocket/edge.
#[cfg(test)]
mod tests;
