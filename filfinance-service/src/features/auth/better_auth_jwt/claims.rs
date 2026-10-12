//! Claims Better Auth utiles au contexte backend Fil Finance.

// `Deserialize` mappe les claims utiles du payload Better Auth vers une struct typee.
use serde::Deserialize;

/// Claims JWT emis par le plugin Better Auth.
#[derive(Debug, Clone, Deserialize)]
pub(super) struct BetterAuthClaims {
    /// `exp` borne aussi la duree des connexions WebSocket apres le handshake.
    pub(super) exp: u64,
    /// `sub` porte l'identifiant Better Auth stable.
    pub(super) sub: String,
    /// Better Auth connait toujours l'email d'un utilisateur.
    pub(super) email: String,
    /// Seuls les comptes dont Better Auth a verifie l'adresse peuvent acceder a l'API.
    pub(super) email_verified: bool,
    /// Le role vient du plugin Admin Better Auth.
    pub(super) role: String,
}
