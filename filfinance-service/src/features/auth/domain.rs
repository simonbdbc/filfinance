//! Domaine auth: identite et role Better Auth verifies.

// La configuration auth porte la liste des roles moderateur autorises.
use crate::config::AuthConfig;

/// Utilisateur authentifie rendu disponible aux handlers via les extensions `axum`.
#[derive(Debug, Clone)]
pub struct CurrentUser {
    /// Identifiant Better Auth stable issu du claim `sub`.
    pub id: String,
    /// Email eventuel du compte, absent pour certains fournisseurs ou tokens.
    pub email: Option<String>,
    /// Role utilisateur Fil Finance signe par Better Auth.
    pub role: String,
    /// Expiration Unix du JWT ayant authentifie la requete, absente seulement quand auth est off.
    pub access_token_expires_at: Option<u64>,
}

/// Identite Better Auth verifiee cryptographiquement.
#[derive(Debug, Clone)]
pub struct AuthenticatedIdentity {
    /// Identifiant Better Auth stable issu du claim `sub`.
    pub id: String,
    /// Email eventuel du compte, absent pour certains fournisseurs ou tokens.
    pub email: Option<String>,
    /// Role signe dans le JWT Better Auth.
    pub role: String,
    /// Claim `exp` verifie, conserve pour borner les transports persistants.
    pub access_token_expires_at: Option<u64>,
}

impl CurrentUser {
    /// Construit le contexte utilise quand l'auth est explicitement desactivee en local.
    pub fn development() -> Self {
        // Le role `moderator` garde les routes de debug utilisables en developpement sans token.
        Self {
            id: "development".to_owned(),
            email: None,
            role: "moderator".to_owned(),
            access_token_expires_at: None,
        }
    }

    /// Construit le contexte courant a partir des claims Better Auth verifies.
    pub(in crate::features::auth) fn from_identity(identity: &AuthenticatedIdentity) -> Self {
        Self {
            id: identity.id.clone(),
            email: identity.email.clone(),
            role: identity.role.clone(),
            access_token_expires_at: identity.access_token_expires_at,
        }
    }

    /// Verifie si le role courant fait partie des roles moderateur autorises.
    pub fn can_access_moderator_routes(&self, config: &AuthConfig) -> bool {
        // La comparaison est exacte et trimmee pour eviter des promotions implicites de role.
        config
            .moderator_roles
            .iter()
            .any(|role| role.trim() == self.role)
    }

    /// Verifie si le role courant est le role admin applicatif.
    pub fn can_access_admin_routes(&self) -> bool {
        // Le role admin est volontairement strict.
        self.role == "admin"
    }
}

impl AuthenticatedIdentity {
    /// Construit l'identite de developpement quand l'auth serveur est desactivee.
    pub(super) fn development() -> Self {
        // Cette identite ne sert qu'aux chemins locaux qui veulent simuler un utilisateur.
        Self {
            id: "development".to_owned(),
            email: None,
            role: "moderator".to_owned(),
            access_token_expires_at: None,
        }
    }
}
