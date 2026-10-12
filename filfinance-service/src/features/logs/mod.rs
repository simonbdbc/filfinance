//! Feature `logs`, responsable du viewer admin des logs backend.

// `domain` porte les DTO JSON exposes a l'admin.
pub(crate) mod domain;
// `http` contient les handlers Axum proteges par les routes operateur.
pub(crate) mod http;
// `service` isole la lecture disque et les controles anti path traversal.
pub(crate) mod service;
// `websocket` expose le tail live des logs backend aux moderators.
pub(crate) mod websocket;
// Les tests logs verrouillent retention J/J-1 et pagination.
#[cfg(test)]
mod tests;
