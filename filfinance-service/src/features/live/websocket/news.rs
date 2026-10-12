//! Facade du flux WebSocket user-facing des news traduites.

// `delivery` porte le replay initial et l'émission des items live.
mod delivery;
// `stream` porte la boucle WebSocket et le multiplexage des canaux.
mod stream;

pub(super) use stream::handle_news_socket;
