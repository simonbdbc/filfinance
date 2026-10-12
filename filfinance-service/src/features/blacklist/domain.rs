//! Domaine blacklist news et audit des rejets/troncatures upstream.

use serde::{Deserialize, Serialize};

use crate::{errors::AppError, shared::text::normalize_whitespace};

/// Correspondance par sous-chaine normalisee.
pub(crate) const BLACKLIST_MATCH_MODE_PARTIAL: &str = "partial";
/// Correspondance de terme ou phrase complete avec bornes de mots.
pub(crate) const BLACKLIST_MATCH_MODE_EXACT: &str = "exact";
/// Filtre applique uniquement au titre source.
pub(crate) const BLACKLIST_TARGET_SCOPE_TITLE: &str = "title";
/// Filtre applique uniquement au contenu source.
pub(crate) const BLACKLIST_TARGET_SCOPE_CONTENT: &str = "content";
/// Filtre applique au titre puis au contenu source.
pub(crate) const BLACKLIST_TARGET_SCOPE_TITLE_CONTENT: &str = "title_content";
/// Action d'audit quand la news complete est rejetee.
pub(crate) const BLACKLIST_ACTION_REJECT_NEWS: &str = "reject_news";
/// Action d'audit quand seul le contenu est retire.
pub(crate) const BLACKLIST_ACTION_STRIP_CONTENT: &str = "strip_content";

/// Entree operateur de blacklist news.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub(crate) struct NewsBlacklistEntry {
    /// Identifiant technique SQLite.
    pub id: i64,
    /// Mot ou phrase source anglais a filtrer.
    pub term: String,
    /// Mode de correspondance applique au titre/contenu source.
    pub match_mode: String,
    /// Champ source cible par la regle.
    pub target_scope: String,
    /// Indique si la regle est active dans le hot path.
    pub enabled: bool,
    /// Note operateur optionnelle.
    pub notes: Option<String>,
    /// Date de creation RFC3339.
    pub created_at: String,
    /// Date de derniere mise a jour RFC3339.
    pub updated_at: String,
}

/// Audit d'une news rejetee par blacklist.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub(crate) struct NewsBlacklistHit {
    /// Identifiant technique SQLite.
    pub id: i64,
    /// Regle touchee, si elle existe encore.
    pub blacklist_entry_id: Option<i64>,
    /// Terme snapshot au moment du rejet.
    pub term: String,
    /// Mode snapshot au moment du rejet.
    pub match_mode: String,
    /// Portee snapshot au moment de l'action.
    pub target_scope: String,
    /// Action appliquee: rejet complet ou retrait du contenu.
    pub action: String,
    /// Source de la news entrante.
    pub source: String,
    /// Type metier de la news entrante.
    pub event_kind: String,
    /// Champ ayant matche: `title` ou `body`.
    pub matched_field: String,
    /// Extrait du champ matche.
    pub matched_text: String,
    /// Titre source complet ou tronque pour audit.
    pub source_title: String,
    /// Extrait du contenu source.
    pub source_body_excerpt: Option<String>,
    /// Reference brute si disponible.
    pub raw_payload_ref: Option<String>,
    /// Signature canonique entrante.
    pub signature: String,
    /// Date d'audit RFC3339.
    pub created_at: String,
}

/// Audit d'une troncature deja presente dans la source upstream.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub(crate) struct NewsTruncationAudit {
    /// Identifiant technique SQLite.
    pub id: i64,
    /// Source de la news entrante.
    pub source: String,
    /// Champ concerne, actuellement `body`.
    pub field_name: String,
    /// Marqueur detecte, par exemple `…` ou `...`.
    pub marker: String,
    /// Titre source complet ou tronque pour audit.
    pub source_title: String,
    /// Extrait du champ tronque.
    pub field_excerpt: String,
    /// Reference brute si disponible.
    pub raw_payload_ref: Option<String>,
    /// Signature canonique entrante.
    pub signature: String,
    /// Date d'audit RFC3339.
    pub created_at: String,
}

/// Query string de `GET /api/news-blacklist`.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct NewsBlacklistListQuery {
    /// Filtre optionnel sur les regles actives.
    pub enabled: Option<bool>,
    /// Recherche texte optionnelle.
    pub q: Option<String>,
    /// Limite optionnelle bornee cote serveur.
    pub limit: Option<i64>,
}

/// Query string des endpoints d'audit.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct NewsAuditListQuery {
    /// Limite optionnelle bornee cote serveur.
    pub limit: Option<i64>,
}

/// Parametres normalises de lecture blacklist.
#[derive(Debug, Clone)]
pub(crate) struct NewsBlacklistListParams {
    /// Filtre optionnel sur les regles actives.
    pub enabled: Option<bool>,
    /// Recherche texte deja trimmee.
    pub query: Option<String>,
    /// Limite bornee entre 1 et 500.
    pub limit: i64,
}

impl NewsBlacklistListParams {
    /// Normalise la query string avant SQL.
    pub(crate) fn from_query(query: NewsBlacklistListQuery) -> Self {
        let normalized_query = query.q.and_then(|value| {
            let normalized = normalize_whitespace(&value);
            if normalized.is_empty() {
                None
            } else {
                Some(normalized)
            }
        });

        Self {
            enabled: query.enabled,
            query: normalized_query,
            limit: query.limit.unwrap_or(100).clamp(1, 500),
        }
    }

    /// Parametres utilises par le pipeline chaud.
    pub(crate) fn active_for_pipeline() -> Self {
        Self {
            enabled: Some(true),
            query: None,
            limit: 1_000,
        }
    }
}

/// Parametres normalises de lecture audit.
#[derive(Debug, Clone)]
pub(crate) struct NewsAuditListParams {
    /// Limite bornee entre 1 et 500.
    pub limit: i64,
}

impl NewsAuditListParams {
    /// Normalise la query string audit.
    pub(crate) fn from_query(query: NewsAuditListQuery) -> Self {
        Self {
            limit: query.limit.unwrap_or(100).clamp(1, 500),
        }
    }
}

/// Payload de creation d'une regle blacklist.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct CreateNewsBlacklistEntryRequest {
    /// Mot ou phrase source anglais.
    pub term: String,
    /// Mode de correspondance.
    pub match_mode: String,
    /// Champ source cible; defaut serveur: titre + contenu.
    pub target_scope: Option<String>,
    /// Activation initiale.
    pub enabled: bool,
    /// Note optionnelle.
    pub notes: Option<String>,
}

/// Payload de mise a jour partielle d'une regle blacklist.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct UpdateNewsBlacklistEntryRequest {
    /// Mot ou phrase source anglais optionnel.
    pub term: Option<String>,
    /// Mode de correspondance optionnel.
    pub match_mode: Option<String>,
    /// Champ source cible optionnel.
    pub target_scope: Option<String>,
    /// Activation optionnelle.
    pub enabled: Option<bool>,
    /// Note optionnelle; `null` efface la valeur.
    pub notes: Option<Option<String>>,
}

/// Payload normalise pret a inserer.
#[derive(Debug, Clone)]
pub(crate) struct NormalizedCreateNewsBlacklistEntry {
    /// Mot ou phrase source non vide.
    pub term: String,
    /// Mode valide.
    pub match_mode: String,
    /// Champ source cible valide.
    pub target_scope: String,
    /// Activation initiale.
    pub enabled: bool,
    /// Note trimmee.
    pub notes: Option<String>,
}

/// Payload de mise a jour partielle normalise.
#[derive(Debug, Clone)]
pub(crate) struct NormalizedUpdateNewsBlacklistEntry {
    /// Mot ou phrase source normalise.
    pub term: Option<String>,
    /// Mode valide.
    pub match_mode: Option<String>,
    /// Champ source cible valide.
    pub target_scope: Option<String>,
    /// Etat actif.
    pub enabled: Option<bool>,
    /// Note tri-etat.
    pub notes: Option<Option<String>>,
}

/// Ligne d'audit blacklist a inserer.
#[derive(Debug, Clone)]
pub(crate) struct NewNewsBlacklistHit {
    /// Regle touchee.
    pub blacklist_entry_id: i64,
    /// Terme snapshot.
    pub term: String,
    /// Mode snapshot.
    pub match_mode: String,
    /// Portee snapshot.
    pub target_scope: String,
    /// Action appliquee.
    pub action: String,
    /// Source entrante.
    pub source: String,
    /// Type entrante.
    pub event_kind: String,
    /// Champ matche.
    pub matched_field: String,
    /// Extrait matche.
    pub matched_text: String,
    /// Titre source.
    pub source_title: String,
    /// Corps source extrait.
    pub source_body_excerpt: Option<String>,
    /// Reference brute.
    pub raw_payload_ref: Option<String>,
    /// Signature entrante.
    pub signature: String,
}

/// Ligne d'audit troncature upstream a inserer.
#[derive(Debug, Clone)]
pub(crate) struct NewNewsTruncationAudit {
    /// Source entrante.
    pub source: String,
    /// Champ concerne.
    pub field_name: String,
    /// Marqueur detecte.
    pub marker: String,
    /// Titre source.
    pub source_title: String,
    /// Extrait du champ.
    pub field_excerpt: String,
    /// Reference brute.
    pub raw_payload_ref: Option<String>,
    /// Signature entrante.
    pub signature: String,
}

impl CreateNewsBlacklistEntryRequest {
    /// Valide et normalise un payload de creation.
    pub(crate) fn normalize(self) -> Result<NormalizedCreateNewsBlacklistEntry, AppError> {
        Ok(NormalizedCreateNewsBlacklistEntry {
            term: normalize_required_text("term", self.term)?,
            match_mode: normalize_match_mode(self.match_mode)?,
            target_scope: normalize_target_scope(self.target_scope)?,
            enabled: self.enabled,
            notes: normalize_optional_text(self.notes),
        })
    }
}

impl UpdateNewsBlacklistEntryRequest {
    /// Valide et normalise un payload de mise a jour.
    pub(crate) fn normalize(self) -> Result<NormalizedUpdateNewsBlacklistEntry, AppError> {
        Ok(NormalizedUpdateNewsBlacklistEntry {
            term: normalize_optional_required_text("term", self.term)?,
            match_mode: self.match_mode.map(normalize_match_mode).transpose()?,
            target_scope: self
                .target_scope
                .map(|target_scope| normalize_target_scope(Some(target_scope)))
                .transpose()?,
            enabled: self.enabled,
            notes: self.notes.map(normalize_optional_text),
        })
    }
}

fn normalize_required_text(field: &str, value: String) -> Result<String, AppError> {
    let normalized = normalize_whitespace(&value);
    if normalized.is_empty() {
        return Err(AppError::Config(format!("{field} cannot be empty")));
    }
    Ok(normalized)
}

fn normalize_optional_required_text(
    field: &str,
    value: Option<String>,
) -> Result<Option<String>, AppError> {
    value
        .map(|candidate| normalize_required_text(field, candidate))
        .transpose()
}

fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value.and_then(|candidate| {
        let normalized = normalize_whitespace(&candidate);
        if normalized.is_empty() {
            None
        } else {
            Some(normalized)
        }
    })
}

fn normalize_match_mode(value: String) -> Result<String, AppError> {
    let normalized = value.trim().to_ascii_lowercase();
    match normalized.as_str() {
        BLACKLIST_MATCH_MODE_EXACT | BLACKLIST_MATCH_MODE_PARTIAL => Ok(normalized),
        _ => Err(AppError::Config(
            "match_mode must be exact or partial".to_owned(),
        )),
    }
}

fn normalize_target_scope(value: Option<String>) -> Result<String, AppError> {
    let Some(value) = value else {
        return Ok(BLACKLIST_TARGET_SCOPE_TITLE_CONTENT.to_owned());
    };
    let normalized = value.trim().to_ascii_lowercase();
    match normalized.as_str() {
        BLACKLIST_TARGET_SCOPE_TITLE
        | BLACKLIST_TARGET_SCOPE_CONTENT
        | BLACKLIST_TARGET_SCOPE_TITLE_CONTENT => Ok(normalized),
        _ => Err(AppError::Config(
            "target_scope must be title, content or title_content".to_owned(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_blacklist_defaults_target_scope_to_title_content() {
        let payload = CreateNewsBlacklistEntryRequest {
            term: "blocked".to_owned(),
            match_mode: BLACKLIST_MATCH_MODE_PARTIAL.to_owned(),
            target_scope: None,
            enabled: true,
            notes: None,
        };

        let normalized = payload.normalize().expect("payload should normalize");

        assert_eq!(
            normalized.target_scope,
            BLACKLIST_TARGET_SCOPE_TITLE_CONTENT
        );
    }

    #[test]
    fn create_blacklist_rejects_unknown_target_scope() {
        let payload = CreateNewsBlacklistEntryRequest {
            term: "blocked".to_owned(),
            match_mode: BLACKLIST_MATCH_MODE_PARTIAL.to_owned(),
            target_scope: Some("summary".to_owned()),
            enabled: true,
            notes: None,
        };

        let error = payload
            .normalize()
            .expect_err("unknown target_scope should fail");

        assert!(error.to_string().contains("target_scope"));
    }
}
