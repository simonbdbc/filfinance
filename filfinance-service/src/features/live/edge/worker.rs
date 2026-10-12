//! Facade du worker de publication metadata live vers Cloudflare Durable Objects.

// `lifecycle` garde la boucle de shutdown/broadcast, `publish` garde le mapping d'une traduction.
mod lifecycle;
mod publish;

pub use lifecycle::spawn;
