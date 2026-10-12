//! Constantes du protocole et du polling FinancialJuice.

/// Feed fonctionnel principal demandé par l'EPIC.
pub(super) const FINANCIAL_JUICE_MAIN_FEED_ID: &str = "mainfeed";
/// Hub ASP.NET SignalR utilisé par les news live.
pub(super) const FINANCIAL_JUICE_SIGNALR_HUB_NAME: &str = "newsHub";
/// Protocole SignalR utilisé par le site.
pub(super) const FINANCIAL_JUICE_SIGNALR_CLIENT_PROTOCOL: &str = "1.5";
/// `connectionData` attendu par SignalR pour le hub news.
pub(super) const FINANCIAL_JUICE_SIGNALR_CONNECTION_DATA: &str = r#"[{"name":"newsHub"}]"#;
/// Nombre de pages recentes relues a chaque poll HTTP pour rattraper un trou live.
pub(super) const FINANCIAL_JUICE_PERIODIC_CATCHUP_PAGE_LIMIT: usize = 6;
/// Nombre de pages consecutives totalement hors fenetre avant d'arreter le backfill startup.
pub(super) const FINANCIAL_JUICE_OUTSIDE_WINDOW_PAGE_STREAK_LIMIT: usize = 2;
