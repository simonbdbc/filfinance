//! Contrats internes de l'outbox durable de traduction.

use crate::{errors::AppError, features::news::domain::PersistedNewsEvent};

/// Requête interne pour créer un travail de traduction durable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NewTranslationWork {
    target_language: String,
}

impl NewTranslationWork {
    /// Normalise et valide la langue cible avant toute écriture SQLite.
    pub(crate) fn try_new(target_language: impl AsRef<str>) -> Result<Self, AppError> {
        let target_language = target_language.as_ref().trim().to_ascii_lowercase();
        let valid = !target_language.is_empty()
            && target_language.len() <= 32
            && target_language
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-');
        if !valid {
            return Err(AppError::Config(
                "NEWS_TRANSLATION_TARGET_LANGUAGE must be a non-empty language tag".to_owned(),
            ));
        }

        Ok(Self { target_language })
    }

    /// Retourne la langue cible normalisée, sans exposer de DTO HTTP.
    pub(crate) fn target_language(&self) -> &str {
        &self.target_language
    }
}

/// Etat fermé d'un travail de traduction durable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TranslationWorkStatus {
    /// Travail durablement admis, disponible à la consommation.
    Pending,
    /// Travail temporairement possédé par un worker doté d'un fencing token.
    Leased,
    /// Traduction persistée et travail acquitté dans la même transaction.
    Completed,
    /// Echec terminal, après le budget de retry borné.
    DeadLetter,
}

impl TranslationWorkStatus {
    /// Représentation SQLite stable du statut interne.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Leased => "leased",
            Self::Completed => "completed",
            Self::DeadLetter => "dead_letter",
        }
    }

    pub(crate) fn from_storage(value: &str) -> Result<Self, AppError> {
        match value {
            "pending" => Ok(Self::Pending),
            "leased" => Ok(Self::Leased),
            "completed" => Ok(Self::Completed),
            "dead_letter" => Ok(Self::DeadLetter),
            _ => Err(AppError::storage(
                "news_translation_outbox contains an unknown status",
            )),
        }
    }
}

/// Projection interne d'un travail durable persisté.
#[derive(Debug, Clone)]
pub(crate) struct PersistedTranslationWork {
    /// Identifiant SQLite technique du travail.
    pub(crate) id: i64,
    /// Evénement news canonique relu par le futur worker.
    pub(crate) news_event_id: i64,
    /// Langue cible de la variante demandée.
    pub(crate) target_language: String,
    /// Etat fermé de la machine à états de l'outbox.
    pub(crate) status: TranslationWorkStatus,
    /// Nombre de tentatives déjà démarrées.
    pub(crate) attempt_count: u32,
    /// Horodatage à partir duquel le travail redevient réclamable.
    pub(crate) available_at: String,
    /// Génération de fencing future, incrémentée à chaque lease.
    pub(crate) lease_generation: u64,
    /// Expiration de lease, présente uniquement à l'état `leased`.
    pub(crate) lease_expires_at: Option<String>,
    /// Code d'erreur fermé, jamais un message fournisseur brut.
    pub(crate) last_error_code: Option<String>,
    /// Horodatage de création du travail.
    pub(crate) created_at: String,
    /// Horodatage de dernière mutation du travail.
    pub(crate) updated_at: String,
    /// Horodatage d'acquittement terminal éventuel.
    pub(crate) completed_at: Option<String>,
}

/// Résultat typé de l'admission atomique événement + travail de traduction.
#[derive(Debug, Clone)]
pub(crate) enum NewsTranslationAdmission {
    /// Les deux lignes ont été committées ensemble.
    Inserted {
        event: PersistedNewsEvent,
        work: PersistedTranslationWork,
    },
    /// L'événement canonique existait déjà et porte désormais le travail demandé.
    ///
    /// `work_inserted` permet au futur consumer de distinguer la reprise d'une
    /// écriture legacy sans outbox d'une admission déjà durable, sans exposer ce
    /// détail aux contrats HTTP ou n8n.
    Existing {
        event: PersistedNewsEvent,
        work: PersistedTranslationWork,
        work_inserted: bool,
    },
}
