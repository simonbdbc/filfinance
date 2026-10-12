//! File SQLite durable et isolee du calendrier natif.
//!
//! La file ne transporte volontairement aucun payload metier (ni JSON, ni texte
//! source, ni reponse IA). Un consumer reclame seulement un `kind`, un
//! `local_day` et une cle technique d'idempotence, puis relit les entrees
//! canoniques depuis les stores appropries. Cela garde cette table independante
//! de toute outbox et permet de reprendre les travaux apres un redemarrage.

use std::time::Duration;

use sqlx::{Sqlite, SqlitePool, Transaction};
use time::{
    Date, Duration as TimeDuration, Month, OffsetDateTime, UtcOffset,
    format_description::well_known::Rfc3339,
};

use crate::errors::AppError;

/// Nom de table local a la file calendrier; il ne partage aucun contrat outbox.
const CALENDAR_WORK_TABLE: &str = "calendar_native_work";
/// Une reclamation reste bornee, meme si un futur worker transmet une valeur trop grande.
const MAX_CLAIM_BATCH_SIZE: usize = 64;
/// Les ecrans de supervision peuvent lire plus d'items sans devenir une lecture non bornee.
const MAX_PENDING_QUERY_SIZE: usize = 256;
/// Une lease anormalement longue rendrait la reprise trop lente apres une panne.
const MAX_LEASE_DURATION: Duration = Duration::from_secs(24 * 60 * 60);
/// Une intention calendrier ne peut jamais etre executee indefiniment apres
/// une succession de crashes, de leases abandonnees ou de retries.
///
/// Le runtime et les workers audio appliquent deja le meme budget avant de
/// replanifier un echec connu. Cette borne est aussi imposee dans SQLite afin
/// qu'un crash entre le claim et ce garde-fou ne puisse pas contourner la
/// limite.
const CALENDAR_WORK_MAX_ATTEMPTS: u32 = 3;
/// Code ferme, sans detail fournisseur ni identifiant de worker, pour une
/// intention terminee par le garde-fou de tentatives.
const CALENDAR_WORK_ATTEMPTS_EXHAUSTED_CODE: &str = "calendar_work_attempts_exhausted";
/// Les identifiants techniques restent courts et ne peuvent pas devenir un payload implicite.
const MAX_IDEMPOTENCY_KEY_BYTES: usize = 160;
const MAX_WORKER_ID_BYTES: usize = 128;
const MAX_ERROR_CODE_BYTES: usize = 96;
/// Un stream HLS est une reference technique compacte, pas le contenu audio lui-meme.
const MAX_RESERVED_STREAM_ID_BYTES: usize = 160;
/// Code durable qui interdit toute nouvelle collecte pour le jour concerne.
///
/// Il ne transporte ni nom de fournisseur libre, ni contenu source : il sert
/// seulement de verrou fail-closed lorsqu'un refus d'acces a ete observe.
// Scope a fail-closed audit-write sentinel to the active endpoint set. A historical
// HTML refusal must not ban newly authorised official exports on different endpoints.
pub(crate) const CALENDAR_SOURCE_REFUSAL_TERMINAL_ERROR: &str = "calendar_native_feeds_v1_refused";
/// Nombre maximal d'intentions encore ouvertes dans la file durable.
///
/// Les travaux termines sont purges par la retention existante; cette borne ne
/// concerne donc que ce qui peut consommer des ressources au prochain reveil.
const DEFAULT_MAX_OPEN_WORK: usize = 64;
const MAX_OPEN_WORK: usize = 256;

/// Bootstrap additif du schema propre au calendrier.
///
/// Cette chaine n'est pas rattachee au schema global news: son appel explicite
/// est le seul point d'integration requis au demarrage du futur worker natif.
pub(crate) const CALENDAR_WORK_SCHEMA: &[&str] = &[
    r#"
    CREATE TABLE IF NOT EXISTS calendar_native_work (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        kind TEXT NOT NULL CHECK (kind IN ('ingestion', 'summary', 'item_audio', 'alert_audio', 'summary_audio')),
        local_day TEXT NOT NULL,
        idempotency_key TEXT NOT NULL,
        status TEXT NOT NULL CHECK (status IN ('pending', 'leased', 'completed', 'dead_letter')),
        attempt_count INTEGER NOT NULL DEFAULT 0 CHECK (attempt_count >= 0),
        available_at TEXT NOT NULL,
        lease_generation INTEGER NOT NULL DEFAULT 0 CHECK (lease_generation >= 0),
        lease_owner TEXT,
        lease_expires_at TEXT,
        last_error_code TEXT,
        reserved_stream_id TEXT,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        finished_at TEXT,
        UNIQUE (kind, idempotency_key),
        CHECK (
            (status = 'pending'
                AND lease_owner IS NULL
                AND lease_expires_at IS NULL
                AND finished_at IS NULL)
            OR (status = 'leased'
                AND lease_owner IS NOT NULL
                AND lease_expires_at IS NOT NULL
                AND finished_at IS NULL)
            OR (status = 'completed'
                AND lease_owner IS NULL
                AND lease_expires_at IS NULL
                AND finished_at IS NOT NULL)
            OR (status = 'dead_letter'
                AND lease_owner IS NULL
                AND lease_expires_at IS NULL
                AND finished_at IS NOT NULL)
        )
    );
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS calendar_native_work_claim_ready_idx
        ON calendar_native_work (status, available_at, id);
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS calendar_native_work_pending_kind_idx
        ON calendar_native_work (status, kind, available_at, id);
    "#,
];

/// Colonnes explicites reutilisees par les relectures et les `RETURNING` de la file.
const CALENDAR_WORK_COLUMNS: &str = r#"
    id,
    kind,
    local_day,
    idempotency_key,
    status,
    attempt_count,
    available_at,
    lease_generation,
    lease_owner,
    lease_expires_at,
    last_error_code,
    reserved_stream_id,
    created_at,
    updated_at,
    finished_at
"#;

/// Projection SELECT complete partagee par les requetes de lecture.
const CALENDAR_WORK_SELECT: &str = r#"
    id,
    kind,
    local_day,
    idempotency_key,
    status,
    attempt_count,
    available_at,
    lease_generation,
    lease_owner,
    lease_expires_at,
    last_error_code,
    reserved_stream_id,
    created_at,
    updated_at,
    finished_at
FROM calendar_native_work
"#;

/// Applique le schema additif de la file calendrier sur le pool fourni.
///
/// Le caller choisit le pool SQLite durable. La file reste donc utilisable avec
/// un fichier dedie ou avec un pool applicatif deja ouvert, sans modifier la
/// migration globale existante.
pub(crate) async fn apply_schema(pool: &SqlitePool) -> Result<(), AppError> {
    crate::features::calendar_logs::operation("apply_schema", serde_json::json!({}), async {
        for statement in CALENDAR_WORK_SCHEMA {
            sqlx::query(statement).execute(pool).await?;
        }

        Ok(())
    })
    .await
}

/// Type ferme d'une etape executable par le chemin calendrier natif.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum CalendarWorkKind {
    /// Qualification des evenements source pour un jour local.
    Ingestion,
    /// Construction du resume depuis les items calendrier canoniques.
    Summary,
    /// Clip HLS descriptif d'un item calendrier important.
    ItemAudio,
    /// Clip HLS d'alerte avant un item calendrier important.
    AlertAudio,
    /// Clip HLS du resume global du calendrier.
    SummaryAudio,
}

impl CalendarWorkKind {
    /// Libelle metier de l'etape d'entree du plan d'orchestration.
    ///
    /// Le stockage conserve `ingestion` pour rester compatible avec les lignes
    /// deja creees pendant le developpement, mais les appelants peuvent nommer
    /// explicitement le travail `CollectAndIngest` dans leurs contrats.
    #[allow(dead_code, non_upper_case_globals)]
    pub(crate) const CollectAndIngest: Self = Self::Ingestion;

    /// Libelle metier de l'etape de synthese du plan d'orchestration.
    #[allow(dead_code, non_upper_case_globals)]
    pub(crate) const Summarize: Self = Self::Summary;

    /// Representation SQLite stable, fermee et sans valeur libre.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Ingestion => "ingestion",
            Self::Summary => "summary",
            Self::ItemAudio => "item_audio",
            Self::AlertAudio => "alert_audio",
            Self::SummaryAudio => "summary_audio",
        }
    }

    fn from_storage(value: &str) -> Result<Self, AppError> {
        match value {
            "ingestion" => Ok(Self::Ingestion),
            "summary" => Ok(Self::Summary),
            "item_audio" => Ok(Self::ItemAudio),
            "alert_audio" => Ok(Self::AlertAudio),
            "summary_audio" => Ok(Self::SummaryAudio),
            _ => Err(AppError::storage(
                "calendar_native_work has an unknown kind",
            )),
        }
    }
}

/// Etat ferme de la machine a etats durable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CalendarWorkState {
    /// Travail admis, eventuellement planifie pour un retry futur.
    Pending,
    /// Travail temporairement possede par une lease et un fencing token.
    Leased,
    /// Travail acquitte avec succes par le possesseur de la lease courante.
    Completed,
    /// Echec terminal explicitement choisi par le consumer.
    DeadLetter,
}

impl CalendarWorkState {
    fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Leased => "leased",
            Self::Completed => "completed",
            Self::DeadLetter => "dead_letter",
        }
    }

    fn from_storage(value: &str) -> Result<Self, AppError> {
        match value {
            "pending" => Ok(Self::Pending),
            "leased" => Ok(Self::Leased),
            "completed" => Ok(Self::Completed),
            "dead_letter" => Ok(Self::DeadLetter),
            _ => Err(AppError::storage(
                "calendar_native_work has an unknown status",
            )),
        }
    }
}

/// Demande validee d'admission durable, sans contenu metier embarque.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CalendarWorkRequest {
    kind: CalendarWorkKind,
    local_day: String,
    idempotency_key: String,
}

impl CalendarWorkRequest {
    /// Construit une demande bornee dont la cle peut etre rejouee sans dupliquer le travail.
    pub(crate) fn try_new(
        kind: CalendarWorkKind,
        local_day: impl AsRef<str> + Sync,
        idempotency_key: impl AsRef<str> + Sync,
    ) -> Result<Self, AppError> {
        Ok(Self {
            kind,
            local_day: normalize_local_day(local_day.as_ref())?,
            idempotency_key: normalize_technical_identifier(
                idempotency_key.as_ref(),
                MAX_IDEMPOTENCY_KEY_BYTES,
                "calendar_work_invalid_idempotency_key",
            )?,
        })
    }

    /// Etape native fermee a executer.
    pub(crate) fn kind(&self) -> CalendarWorkKind {
        self.kind
    }

    /// Jour local canonique auquel le worker doit relire ses donnees.
    pub(crate) fn local_day(&self) -> &str {
        &self.local_day
    }

    /// Cle technique unique par kind, sans jamais contenir de payload metier.
    pub(crate) fn idempotency_key(&self) -> &str {
        &self.idempotency_key
    }
}

/// Code d'erreur ferme, conserve pour diagnostic sans enregistrer un message fournisseur libre.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CalendarWorkErrorCode(String);

impl CalendarWorkErrorCode {
    /// Normalise un code stable tel que `provider_unavailable` ou `invalid_input`.
    pub(crate) fn try_new(value: impl AsRef<str> + Sync) -> Result<Self, AppError> {
        let value = value.as_ref().trim().to_ascii_lowercase();
        let valid = !value.is_empty()
            && value.len() <= MAX_ERROR_CODE_BYTES
            && value.bytes().all(|byte| {
                byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_' || byte == b'-'
            });
        if !valid {
            return Err(AppError::Config(
                "calendar_work_invalid_error_code".to_owned(),
            ));
        }

        Ok(Self(value))
    }

    /// Code ferme utilisable dans les statuts calendrier sans detail fournisseur.
    pub(crate) fn as_public_code(&self) -> &str {
        &self.0
    }
}

/// Projection d'une ligne durable, sans contenu source ou IA.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CalendarWork {
    /// Identifiant SQLite technique.
    pub(crate) id: i64,
    /// Etape native demandee.
    pub(crate) kind: CalendarWorkKind,
    /// Jour local a relire dans les stores canoniques.
    pub(crate) local_day: String,
    /// Cle de deduplication durable de l'admission.
    pub(crate) idempotency_key: String,
    /// Etat courant de la file.
    pub(crate) state: CalendarWorkState,
    /// Nombre de leases demarrees, donc de tentatives effectivement commencees.
    pub(crate) attempt_count: u32,
    /// Instant a partir duquel un pending peut etre reclame.
    pub(crate) available_at: String,
    /// Token monotone incremente a chaque claim.
    pub(crate) lease_generation: u64,
    /// Worker possesseur uniquement dans l'etat leased.
    pub(crate) lease_owner: Option<String>,
    /// Borne de validite de lease uniquement dans l'etat leased.
    pub(crate) lease_expires_at: Option<String>,
    /// Code d'echec ferme du dernier retry ou echec terminal.
    pub(crate) last_error_code: Option<String>,
    /// Stream HLS fixe une seule fois avant dispatch afin de rendre l'audio recuperable.
    pub(crate) reserved_stream_id: Option<String>,
    /// Horodatage de creation.
    pub(crate) created_at: String,
    /// Horodatage de derniere mutation.
    pub(crate) updated_at: String,
    /// Horodatage terminal pour completed ou dead_letter.
    pub(crate) finished_at: Option<String>,
}

/// Capacite de mutation liee a un seul owner et une seule generation de lease.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CalendarWorkLease {
    work_id: i64,
    generation: u64,
    owner: String,
}

impl CalendarWorkLease {
    /// Identifiant du travail protege par ce token.
    pub(crate) fn work_id(&self) -> i64 {
        self.work_id
    }

    /// Generation de fencing qui doit correspondre exactement a la ligne courante.
    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }

    /// Identite du worker qui a reclame le travail.
    pub(crate) fn owner(&self) -> &str {
        &self.owner
    }
}

/// Travail retourne au consumer avec le token requis pour toute mutation terminale ou retry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ClaimedCalendarWork {
    pub(crate) work: CalendarWork,
    pub(crate) lease: CalendarWorkLease,
}

/// Contexte de reprise remis au worker audio apres la reservation HLS.
///
/// Le job audio ne transporte pas de donnees source ni IA : ce token suffit
/// pour acquitter ou replanifier exactement l'intention SQLite qui l'a cree.
/// Sa copie garde le fencing token d'origine, de sorte qu'un worker expire ne
/// puisse jamais clore le travail repris par un autre processus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CalendarAudioWorkContext {
    lease: CalendarWorkLease,
    attempt_count: u32,
}

impl CalendarAudioWorkContext {
    /// Extrait le token durable du travail actuellement reclame.
    pub(crate) fn from_claimed(claimed: &ClaimedCalendarWork) -> Self {
        Self {
            lease: claimed.lease.clone(),
            attempt_count: claimed.work.attempt_count,
        }
    }

    /// Token a presenter pour toute mutation de la file.
    pub(crate) fn lease(&self) -> &CalendarWorkLease {
        &self.lease
    }

    /// Nombre borne de tentatives effectivement commencees.
    pub(crate) fn attempt_count(&self) -> u32 {
        self.attempt_count
    }
}

/// Verdict idempotent de l'admission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CalendarWorkAdmission {
    /// Une nouvelle ligne durable vient d'etre creee.
    Inserted(CalendarWork),
    /// La cle etait deja durable; aucun etat n'a ete reinitialise.
    Existing(CalendarWork),
    /// Un travail audio termine a ete rendu reclamable a nouveau pour une
    /// reparation HLS, sans perdre son stream reserve.
    Reactivated(CalendarWork),
}

impl CalendarWorkAdmission {
    /// Indique si cette demande a cree la ligne plutot que de relire son admission precedente.
    pub(crate) fn inserted(&self) -> bool {
        matches!(self, Self::Inserted(_))
    }

    /// Retourne la ligne persistante sans la cloner.
    pub(crate) fn work(&self) -> &CalendarWork {
        match self {
            Self::Inserted(work) | Self::Existing(work) | Self::Reactivated(work) => work,
        }
    }

    /// Distingue une reparation HLS d'une admission initiale dans les traces
    /// internes, sans transformer ce detail en contrat HTTP.
    pub(crate) fn reactivated(&self) -> bool {
        matches!(self, Self::Reactivated(_))
    }
}

/// Resultat d'une mutation protegee par fencing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CalendarWorkLeaseMutation {
    /// La lease courante a ete modifiee et la ligne retournee est le nouvel etat durable.
    Applied(CalendarWork),
    /// Le travail n'est plus possede par ce token (lease expiree, recuperee ou terminee).
    FencedOut,
}

/// Store clonable de la file SQLite calendrier.
#[derive(Clone)]
pub(crate) struct CalendarWorkStore {
    pool: SqlitePool,
    max_open_work: usize,
}

impl CalendarWorkStore {
    /// Construit le store autour d'un pool SQLite durable deja ouvert par le bootstrap appelant.
    pub(crate) fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            max_open_work: DEFAULT_MAX_OPEN_WORK,
        }
    }

    /// Construit une file avec une borne explicite des intentions ouvertes.
    ///
    /// La configuration est refusee au bootstrap; ce constructeur reste
    /// defensif pour les tests et les appels internes.
    pub(crate) fn with_max_open_work(
        pool: SqlitePool,
        max_open_work: usize,
    ) -> Result<Self, AppError> {
        if !(1..=MAX_OPEN_WORK).contains(&max_open_work) {
            return Err(AppError::Config(
                "calendar_work_invalid_open_work_limit".to_owned(),
            ));
        }
        Ok(Self {
            pool,
            max_open_work,
        })
    }

    /// Applique explicitement le schema propre a ce store, sans modifier le schema global.
    pub(crate) async fn apply_schema(&self) -> Result<(), AppError> {
        apply_schema(&self.pool).await
    }

    /// Admet un travail de maniere atomique et idempotente par `(kind, idempotency_key)`.
    ///
    /// Une cle rejouee avec un autre jour est refusee: accepter ce conflit creerait un
    /// second sens pour la meme intention idempotente.
    pub(crate) async fn admit(
        &self,
        request: &CalendarWorkRequest,
        now: OffsetDateTime,
    ) -> Result<CalendarWorkAdmission, AppError> {
        crate::features::calendar_logs::operation(
            "admit",
            serde_json::json!({"request": format!("{:?}", request), "now": format!("{:?}", now)}),
            async {
                let now = storage_timestamp(now)?;
                admit_with_pool(&self.pool, self.max_open_work, request, &now).await
            },
        )
        .await
    }

    /// Variante transactionnelle utilisee par le commit calendrier canonique.
    ///
    /// Elle pose les intentions audio/summary dans la meme transaction SQLite
    /// que les items, alertes et changement d'etat. Ainsi aucun item ne peut
    /// devenir visible sans travail aval durable correspondant.
    pub(crate) async fn admit_in_tx(
        &self,
        tx: &mut Transaction<'_, Sqlite>,
        request: &CalendarWorkRequest,
        now: OffsetDateTime,
    ) -> Result<CalendarWorkAdmission, AppError> {
        crate::features::calendar_logs::operation(
            "admit_in_tx",
            serde_json::json!({"request": format!("{:?}", request), "now": format!("{:?}", now)}),
            async {
                let now = storage_timestamp(now)?;
                admit_with_tx(tx, self.max_open_work, request, &now).await
            },
        )
        .await
    }

    /// Cree ou rend a nouveau executable un travail audio de reparation.
    ///
    /// Un travail `pending` ou `leased` est deja l'intention durable voulue et
    /// reste donc intact. En revanche, une intention `completed` ou
    /// `dead_letter` doit pouvoir reconstituer le meme manifeste HLS : elle
    /// redevient `pending`, repart avec son budget de tentatives et conserve
    /// son `reserved_stream_id`. La borne des travaux ouverts est verifiee
    /// dans la meme transaction avant cette reactivation.
    pub(crate) async fn reactivate_audio_for_repair_in_tx(
        &self,
        tx: &mut Transaction<'_, Sqlite>,
        request: &CalendarWorkRequest,
        preferred_stream_id: Option<&str>,
        now: OffsetDateTime,
    ) -> Result<CalendarWorkAdmission, AppError> {
        crate::features::calendar_logs::operation("reactivate_audio_for_repair_in_tx", serde_json::json!({"request": format!("{:?}", request), "preferred_stream_id": format!("{:?}", preferred_stream_id), "now": format!("{:?}", now)}), async {

        if !matches!(
            request.kind,
            CalendarWorkKind::ItemAudio
                | CalendarWorkKind::AlertAudio
                | CalendarWorkKind::SummaryAudio
        ) {
            return Err(AppError::Config(
                "calendar_work_reactivation_requires_audio_kind".to_owned(),
            ));
        }

        let preferred_stream_id = valid_reserved_stream_id(preferred_stream_id);
        let now = storage_timestamp(now)?;
        let select = format!(
            "SELECT {CALENDAR_WORK_SELECT} WHERE kind = ?1 AND idempotency_key = ?2 LIMIT 1"
        );
        let existing = sqlx::query_as::<_, CalendarWorkRow>(&select)
            .bind(request.kind().as_str())
            .bind(request.idempotency_key())
            .fetch_optional(&mut **tx)
            .await?;

        let Some(existing) = existing else {
            let admission = admit_with_tx(tx, self.max_open_work, request, &now).await?;
            if !admission.inserted() || preferred_stream_id.is_none() {
                return Ok(admission);
            }
            let work = reserve_pending_stream_in_tx(
                tx,
                admission.work().id,
                preferred_stream_id.as_deref(),
                &now,
            )
            .await?;
            return Ok(CalendarWorkAdmission::Inserted(work));
        };

        let existing = existing_admission(existing, request)?;
        match existing.work().state {
            CalendarWorkState::Pending | CalendarWorkState::Leased => Ok(existing),
            CalendarWorkState::Completed | CalendarWorkState::DeadLetter => {
                let max_open_work = i64::try_from(self.max_open_work)
                    .map_err(|_| AppError::storage("calendar_work_open_work_limit_overflow"))?;
                let statement = format!(
                    r#"
                    UPDATE {CALENDAR_WORK_TABLE}
                    SET
                        status = ?1,
                        attempt_count = 0,
                        available_at = ?2,
                        lease_owner = NULL,
                        lease_expires_at = NULL,
                        last_error_code = NULL,
                        reserved_stream_id = COALESCE(reserved_stream_id, ?3),
                        updated_at = ?2,
                        finished_at = NULL
                    WHERE id = ?4
                      AND status IN (?5, ?6)
                      AND (
                          SELECT COUNT(*)
                          FROM {CALENDAR_WORK_TABLE}
                          WHERE status IN ('pending', 'leased')
                      ) < ?7
                    RETURNING {CALENDAR_WORK_COLUMNS}
                    "#,
                );
                let row = sqlx::query_as::<_, CalendarWorkRow>(&statement)
                    .bind(CalendarWorkState::Pending.as_str())
                    .bind(&now)
                    .bind(preferred_stream_id.as_deref())
                    .bind(existing.work().id)
                    .bind(CalendarWorkState::Completed.as_str())
                    .bind(CalendarWorkState::DeadLetter.as_str())
                    .bind(max_open_work)
                    .fetch_optional(&mut **tx)
                    .await?;
                match row {
                    Some(row) => Ok(CalendarWorkAdmission::Reactivated(row.try_into()?)),
                    None => {
                        let open_count: i64 = sqlx::query_scalar(&format!(
                            "SELECT COUNT(*) FROM {CALENDAR_WORK_TABLE} WHERE status IN ('pending', 'leased')"
                        ))
                        .fetch_one(&mut **tx)
                        .await?;
                        if open_count >= max_open_work {
                            Err(AppError::conflict("calendar_native_work_queue_full"))
                        } else {
                            Err(AppError::conflict("calendar_work_reactivation_conflict"))
                        }
                    }
                }
            }
        }

}).await
    }

    /// Reclamation atomique des travaux pending devenus disponibles.
    ///
    /// La meme ecriture traite aussi les lignes laissees par un ancien
    /// processus: une lease expiree repasse `pending` si son budget le permet,
    /// sinon elle devient directement `dead_letter`. Les intentions `pending`
    /// deja epuisees sont egalement fermees avant toute nouvelle execution.
    /// La requete `UPDATE ... RETURNING` incremente le fencing token dans la
    /// meme instruction que le passage a `leased`; deux workers ne peuvent donc
    /// pas recevoir la meme generation ni depasser le budget apres une panne.
    pub(crate) async fn claim(
        &self,
        worker_id: impl AsRef<str> + Sync,
        now: OffsetDateTime,
        lease_for: Duration,
        limit: usize,
    ) -> Result<Vec<ClaimedCalendarWork>, AppError> {
        crate::features::calendar_logs::operation("claim", serde_json::json!({"worker_id": worker_id.as_ref(), "now": format!("{:?}", now), "lease_for": format!("{:?}", lease_for), "limit": format!("{:?}", limit)}), async {

        let worker_id = normalize_technical_identifier(
            worker_id.as_ref(),
            MAX_WORKER_ID_BYTES,
            "calendar_work_invalid_worker_id",
        )?;
        let limit = checked_limit(
            limit,
            MAX_CLAIM_BATCH_SIZE,
            "calendar_work_invalid_claim_limit",
        )?;
        let lease_expires_at = storage_timestamp(lease_expires_at(now, lease_for)?)?;
        let now = storage_timestamp(now)?;

        let max_attempts = i64::from(CALENDAR_WORK_MAX_ATTEMPTS);
        let statement = format!(
            r#"
            WITH claimable AS (
                SELECT id
                FROM {CALENDAR_WORK_TABLE}
                WHERE status = ?8
                  AND available_at <= ?7
                  AND attempt_count < ?1
                ORDER BY available_at ASC, id ASC
                LIMIT ?9
            )
            UPDATE {CALENDAR_WORK_TABLE}
            SET
                status = CASE
                    WHEN status = ?3
                         AND lease_expires_at <= ?7
                         AND attempt_count >= ?1 THEN ?2
                    WHEN status = ?3
                         AND lease_expires_at <= ?7 THEN ?8
                    WHEN attempt_count >= ?1 THEN ?2
                    ELSE ?3
                END,
                attempt_count = CASE
                    WHEN status = ?8 AND id IN (SELECT id FROM claimable)
                        THEN attempt_count + 1
                    ELSE attempt_count
                END,
                lease_generation = CASE
                    WHEN status = ?8 AND id IN (SELECT id FROM claimable)
                        THEN lease_generation + 1
                    ELSE lease_generation
                END,
                lease_owner = CASE
                    WHEN status = ?8 AND id IN (SELECT id FROM claimable) THEN ?4
                    ELSE NULL
                END,
                lease_expires_at = CASE
                    WHEN status = ?8 AND id IN (SELECT id FROM claimable) THEN ?5
                    ELSE NULL
                END,
                last_error_code = CASE
                    WHEN (status = ?3
                          AND lease_expires_at <= ?7
                          AND attempt_count >= ?1)
                      OR (status = ?8 AND attempt_count >= ?1) THEN ?6
                    WHEN status = ?3 AND lease_expires_at <= ?7 THEN 'lease_expired'
                    ELSE NULL
                END,
                available_at = CASE
                    WHEN status = ?3
                         AND lease_expires_at <= ?7
                         AND attempt_count < ?1 THEN ?7
                    ELSE available_at
                END,
                updated_at = ?7,
                finished_at = CASE
                    WHEN (status = ?3
                          AND lease_expires_at <= ?7
                          AND attempt_count >= ?1)
                      OR (status = ?8 AND attempt_count >= ?1) THEN ?7
                    ELSE NULL
                END
            WHERE (status = ?3 AND lease_expires_at <= ?7)
               OR (status = ?8 AND attempt_count >= ?1)
               OR (status = ?8 AND id IN (SELECT id FROM claimable))
            RETURNING {CALENDAR_WORK_COLUMNS}
            "#,
        );
        let rows = sqlx::query_as::<_, CalendarWorkRow>(&statement)
            .bind(max_attempts)
            .bind(CalendarWorkState::DeadLetter.as_str())
            .bind(CalendarWorkState::Leased.as_str())
            .bind(&worker_id)
            .bind(&lease_expires_at)
            .bind(CALENDAR_WORK_ATTEMPTS_EXHAUSTED_CODE)
            .bind(&now)
            .bind(CalendarWorkState::Pending.as_str())
            .bind(limit)
            .fetch_all(&self.pool)
            .await?;

        let mut claimed = Vec::with_capacity(rows.len());
        for row in rows {
            let work: CalendarWork = row.try_into()?;
            match work.state {
                CalendarWorkState::Leased if work.lease_owner.as_deref() == Some(&worker_id) => {
                    let lease = CalendarWorkLease {
                        work_id: work.id,
                        generation: work.lease_generation,
                        owner: worker_id.clone(),
                    };
                    claimed.push(ClaimedCalendarWork { work, lease });
                }
                // La requete peut avoir assaini une lease expiree ou une
                // intention deja epuisee. Ces lignes ne sont jamais remises
                // au consumer et ne portent donc aucun nouveau token.
                CalendarWorkState::Pending | CalendarWorkState::DeadLetter => {}
                CalendarWorkState::Leased => {
                    return Err(AppError::storage(
                        "calendar_native_work returned an invalid claimed row",
                    ));
                }
                CalendarWorkState::Completed => {
                    return Err(AppError::storage(
                        "calendar_native_work returned an invalid claim transition",
                    ));
                }
            }
        }
        // SQLite ne promet pas l'ordre de RETURNING; le consumer recoit donc un ordre stable.
        claimed.sort_by(|left, right| {
            left.work
                .available_at
                .cmp(&right.work.available_at)
                .then_with(|| left.work.id.cmp(&right.work.id))
        });

        Ok(claimed)

}).await
    }

    /// Fixe le stream HLS d'un travail audio sans jamais remplacer une reservation existante.
    ///
    /// L'operation est protegee par la lease courante: un worker expire ne peut
    /// pas ecraser une reservation faite par son successeur. Si le stream etait
    /// deja reserve, la ligne retournee porte cette valeur stable a reutiliser.
    pub(crate) async fn reserve_stream_id(
        &self,
        lease: &CalendarWorkLease,
        reserved_stream_id: impl AsRef<str> + Sync,
        now: OffsetDateTime,
    ) -> Result<CalendarWorkLeaseMutation, AppError> {
        crate::features::calendar_logs::operation("reserve_stream_id", serde_json::json!({"lease": format!("{:?}", lease), "reserved_stream_id": reserved_stream_id.as_ref(), "now": format!("{:?}", now)}), async {

        let reserved_stream_id = normalize_technical_identifier(
            reserved_stream_id.as_ref(),
            MAX_RESERVED_STREAM_ID_BYTES,
            "calendar_work_invalid_reserved_stream_id",
        )?;
        let now = storage_timestamp(now)?;
        let generation = storage_generation(lease.generation())?;
        let statement = format!(
            r#"
            UPDATE {CALENDAR_WORK_TABLE}
            SET
                reserved_stream_id = COALESCE(reserved_stream_id, ?1),
                updated_at = ?2
            WHERE id = ?3
              AND status = ?4
              AND lease_generation = ?5
              AND lease_owner = ?6
              AND lease_expires_at > ?2
            RETURNING {CALENDAR_WORK_COLUMNS}
            "#,
        );
        let row = sqlx::query_as::<_, CalendarWorkRow>(&statement)
            .bind(&reserved_stream_id)
            .bind(&now)
            .bind(lease.work_id())
            .bind(CalendarWorkState::Leased.as_str())
            .bind(generation)
            .bind(lease.owner())
            .fetch_optional(&self.pool)
            .await?;

        mutation_from_optional_row(row)

}).await
    }

    /// Acquitte un travail si et seulement si le consumer porte encore la lease courante.
    pub(crate) async fn complete(
        &self,
        lease: &CalendarWorkLease,
        now: OffsetDateTime,
    ) -> Result<CalendarWorkLeaseMutation, AppError> {
        crate::features::calendar_logs::operation(
            "complete",
            serde_json::json!({"lease": format!("{:?}", lease), "now": format!("{:?}", now)}),
            async {
                let now = storage_timestamp(now)?;
                let generation = storage_generation(lease.generation())?;
                let statement = format!(
                    r#"
            UPDATE {CALENDAR_WORK_TABLE}
            SET
                status = ?1,
                lease_owner = NULL,
                lease_expires_at = NULL,
                last_error_code = NULL,
                updated_at = ?2,
                finished_at = ?2
            WHERE id = ?3
              AND status = ?4
              AND lease_generation = ?5
              AND lease_owner = ?6
              AND lease_expires_at > ?2
            RETURNING {CALENDAR_WORK_COLUMNS}
            "#,
                );
                let row = sqlx::query_as::<_, CalendarWorkRow>(&statement)
                    .bind(CalendarWorkState::Completed.as_str())
                    .bind(&now)
                    .bind(lease.work_id())
                    .bind(CalendarWorkState::Leased.as_str())
                    .bind(generation)
                    .bind(lease.owner())
                    .fetch_optional(&self.pool)
                    .await?;

                mutation_from_optional_row(row)
            },
        )
        .await
    }

    /// Acquitte une lease dans une transaction appelante, pour joindre le
    /// changement d'etat metier au travail durable sans fenetre de crash.
    pub(crate) async fn complete_in_tx(
        &self,
        tx: &mut Transaction<'_, Sqlite>,
        lease: &CalendarWorkLease,
        now: OffsetDateTime,
    ) -> Result<CalendarWorkLeaseMutation, AppError> {
        crate::features::calendar_logs::operation(
            "complete_in_tx",
            serde_json::json!({"lease": format!("{:?}", lease), "now": format!("{:?}", now)}),
            async {
                let now = storage_timestamp(now)?;
                let generation = storage_generation(lease.generation())?;
                let statement = format!(
                    r#"
            UPDATE {CALENDAR_WORK_TABLE}
            SET
                status = ?1,
                lease_owner = NULL,
                lease_expires_at = NULL,
                last_error_code = NULL,
                updated_at = ?2,
                finished_at = ?2
            WHERE id = ?3
              AND status = ?4
              AND lease_generation = ?5
              AND lease_owner = ?6
              AND lease_expires_at > ?2
            RETURNING {CALENDAR_WORK_COLUMNS}
            "#,
                );
                let row = sqlx::query_as::<_, CalendarWorkRow>(&statement)
                    .bind(CalendarWorkState::Completed.as_str())
                    .bind(&now)
                    .bind(lease.work_id())
                    .bind(CalendarWorkState::Leased.as_str())
                    .bind(generation)
                    .bind(lease.owner())
                    .fetch_optional(&mut **tx)
                    .await?;

                mutation_from_optional_row(row)
            },
        )
        .await
    }

    /// Replanifie un travail apres un echec transitoire, sous le meme controle de fencing.
    pub(crate) async fn retry(
        &self,
        lease: &CalendarWorkLease,
        error_code: &CalendarWorkErrorCode,
        retry_at: OffsetDateTime,
        now: OffsetDateTime,
    ) -> Result<CalendarWorkLeaseMutation, AppError> {
        crate::features::calendar_logs::operation("retry", serde_json::json!({"lease": format!("{:?}", lease), "error_code": format!("{:?}", error_code), "retry_at": format!("{:?}", retry_at), "now": format!("{:?}", now)}), async {

        if retry_at < now {
            return Err(AppError::Config(
                "calendar_work_retry_at_must_not_precede_now".to_owned(),
            ));
        }

        let retry_at = storage_timestamp(retry_at)?;
        let now = storage_timestamp(now)?;
        let generation = storage_generation(lease.generation())?;
        let max_attempts = i64::from(CALENDAR_WORK_MAX_ATTEMPTS);
        let statement = format!(
            r#"
            UPDATE {CALENDAR_WORK_TABLE}
            SET
                status = CASE WHEN attempt_count >= ?1 THEN ?2 ELSE ?3 END,
                available_at = CASE
                    WHEN attempt_count >= ?1 THEN available_at
                    ELSE ?4
                END,
                lease_owner = NULL,
                lease_expires_at = NULL,
                last_error_code = CASE
                    WHEN attempt_count >= ?1 THEN ?5
                    ELSE ?6
                END,
                updated_at = ?7,
                finished_at = CASE
                    WHEN attempt_count >= ?1 THEN ?7
                    ELSE NULL
                END
            WHERE id = ?8
              AND status = ?9
              AND lease_generation = ?10
              AND lease_owner = ?11
              AND lease_expires_at > ?7
            RETURNING {CALENDAR_WORK_COLUMNS}
            "#,
        );
        let row = sqlx::query_as::<_, CalendarWorkRow>(&statement)
            .bind(max_attempts)
            .bind(CalendarWorkState::DeadLetter.as_str())
            .bind(CalendarWorkState::Pending.as_str())
            .bind(&retry_at)
            .bind(CALENDAR_WORK_ATTEMPTS_EXHAUSTED_CODE)
            .bind(error_code.as_public_code())
            .bind(&now)
            .bind(lease.work_id())
            .bind(CalendarWorkState::Leased.as_str())
            .bind(generation)
            .bind(lease.owner())
            .fetch_optional(&self.pool)
            .await?;

        mutation_from_optional_row(row)

}).await
    }

    /// Termine explicitement un travail non recuperable, toujours sous fencing.
    pub(crate) async fn fail(
        &self,
        lease: &CalendarWorkLease,
        error_code: &CalendarWorkErrorCode,
        now: OffsetDateTime,
    ) -> Result<CalendarWorkLeaseMutation, AppError> {
        crate::features::calendar_logs::operation("fail", serde_json::json!({"lease": format!("{:?}", lease), "error_code": format!("{:?}", error_code), "now": format!("{:?}", now)}), async {

        let now = storage_timestamp(now)?;
        let generation = storage_generation(lease.generation())?;
        let statement = format!(
            r#"
            UPDATE {CALENDAR_WORK_TABLE}
            SET
                status = ?1,
                lease_owner = NULL,
                lease_expires_at = NULL,
                last_error_code = ?2,
                updated_at = ?3,
                finished_at = ?3
            WHERE id = ?4
              AND status = ?5
              AND lease_generation = ?6
              AND lease_owner = ?7
              AND lease_expires_at > ?3
            RETURNING {CALENDAR_WORK_COLUMNS}
            "#,
        );
        let row = sqlx::query_as::<_, CalendarWorkRow>(&statement)
            .bind(CalendarWorkState::DeadLetter.as_str())
            .bind(error_code.as_public_code())
            .bind(&now)
            .bind(lease.work_id())
            .bind(CalendarWorkState::Leased.as_str())
            .bind(generation)
            .bind(lease.owner())
            .fetch_optional(&self.pool)
            .await?;

        mutation_from_optional_row(row)

}).await
    }

    /// Termine une lease dans une transaction metier lorsque l'echec est
    /// terminal. Aucun autre worker ne peut alors reprendre cette intention.
    pub(crate) async fn fail_in_tx(
        &self,
        tx: &mut Transaction<'_, Sqlite>,
        lease: &CalendarWorkLease,
        error_code: &CalendarWorkErrorCode,
        now: OffsetDateTime,
    ) -> Result<CalendarWorkLeaseMutation, AppError> {
        crate::features::calendar_logs::operation("fail_in_tx", serde_json::json!({"lease": format!("{:?}", lease), "error_code": format!("{:?}", error_code), "now": format!("{:?}", now)}), async {

        let now = storage_timestamp(now)?;
        let generation = storage_generation(lease.generation())?;
        let statement = format!(
            r#"
            UPDATE {CALENDAR_WORK_TABLE}
            SET
                status = ?1,
                lease_owner = NULL,
                lease_expires_at = NULL,
                last_error_code = ?2,
                updated_at = ?3,
                finished_at = ?3
            WHERE id = ?4
              AND status = ?5
              AND lease_generation = ?6
              AND lease_owner = ?7
              AND lease_expires_at > ?3
            RETURNING {CALENDAR_WORK_COLUMNS}
            "#,
        );
        let row = sqlx::query_as::<_, CalendarWorkRow>(&statement)
            .bind(CalendarWorkState::DeadLetter.as_str())
            .bind(error_code.as_public_code())
            .bind(&now)
            .bind(lease.work_id())
            .bind(CalendarWorkState::Leased.as_str())
            .bind(generation)
            .bind(lease.owner())
            .fetch_optional(&mut **tx)
            .await?;

        mutation_from_optional_row(row)

}).await
    }

    /// Verifie qu'une lease porte encore le fencing token courant avant un
    /// effet de bord externe. Le caller peut abandonner proprement un job
    /// expire au lieu d'ecrire Piper/HLS avec un ancien possesseur.
    pub(crate) async fn lease_is_current(
        &self,
        lease: &CalendarWorkLease,
        now: OffsetDateTime,
    ) -> Result<bool, AppError> {
        crate::features::calendar_logs::operation(
            "lease_is_current",
            serde_json::json!({"lease": format!("{:?}", lease), "now": format!("{:?}", now)}),
            async {
                let now = storage_timestamp(now)?;
                let generation = storage_generation(lease.generation())?;
                let current = sqlx::query_scalar::<_, i64>(
                    r#"
            SELECT EXISTS (
                SELECT 1
                FROM calendar_native_work
                WHERE id = ?1
                  AND status = 'leased'
                  AND lease_generation = ?2
                  AND lease_owner = ?3
                  AND lease_expires_at > ?4
            )
            "#,
                )
                .bind(lease.work_id())
                .bind(generation)
                .bind(lease.owner())
                .bind(&now)
                .fetch_one(&self.pool)
                .await?;
                Ok(current != 0)
            },
        )
        .await
    }

    /// Variante transactionnelle de la verification de fencing. Elle est
    /// appelee avant le premier changement metier du commit calendrier.
    pub(crate) async fn lease_is_current_in_tx(
        &self,
        tx: &mut Transaction<'_, Sqlite>,
        lease: &CalendarWorkLease,
        now: OffsetDateTime,
    ) -> Result<bool, AppError> {
        crate::features::calendar_logs::operation(
            "lease_is_current_in_tx",
            serde_json::json!({"lease": format!("{:?}", lease), "now": format!("{:?}", now)}),
            async {
                let now = storage_timestamp(now)?;
                let generation = storage_generation(lease.generation())?;
                let current = sqlx::query_scalar::<_, i64>(
                    r#"
            SELECT EXISTS (
                SELECT 1
                FROM calendar_native_work
                WHERE id = ?1
                  AND status = 'leased'
                  AND lease_generation = ?2
                  AND lease_owner = ?3
                  AND lease_expires_at > ?4
            )
            "#,
                )
                .bind(lease.work_id())
                .bind(generation)
                .bind(lease.owner())
                .bind(&now)
                .fetch_one(&mut **tx)
                .await?;
                Ok(current != 0)
            },
        )
        .await
    }

    /// Recupere les leases expirees apres crash ou abandon de worker.
    ///
    /// La mutation unique remet une lease expiree `pending` seulement tant que
    /// son nombre de claims reste inferieur au budget. Une lease au dernier
    /// essai devient `dead_letter` avec un code redacte, et une ligne `pending`
    /// legacy deja epuisee est fermee de la meme maniere. La generation n'est
    /// pas modifiee pendant une reprise: l'ancien token est invalide par le
    /// changement d'etat, puis le claim suivant incremente son fencing token.
    pub(crate) async fn recover_expired_leases(
        &self,
        now: OffsetDateTime,
    ) -> Result<u64, AppError> {
        crate::features::calendar_logs::operation(
            "recover_expired_leases",
            serde_json::json!({"now": format!("{:?}", now)}),
            async {
                let now = storage_timestamp(now)?;
                let max_attempts = i64::from(CALENDAR_WORK_MAX_ATTEMPTS);
                let statement = format!(
                    r#"
            UPDATE {CALENDAR_WORK_TABLE}
            SET
                status = CASE WHEN attempt_count >= ?1 THEN ?2 ELSE ?3 END,
                available_at = CASE
                    WHEN attempt_count >= ?1 THEN available_at
                    ELSE ?5
                END,
                lease_owner = NULL,
                lease_expires_at = NULL,
                last_error_code = CASE
                    WHEN attempt_count >= ?1 THEN ?4
                    ELSE 'lease_expired'
                END,
                updated_at = ?5,
                finished_at = CASE
                    WHEN attempt_count >= ?1 THEN ?5
                    ELSE NULL
                END
            WHERE (status = ?6 AND lease_expires_at <= ?5)
               OR (status = ?3 AND attempt_count >= ?1)
            "#,
                );
                let result = sqlx::query(&statement)
                    .bind(max_attempts)
                    .bind(CalendarWorkState::DeadLetter.as_str())
                    .bind(CalendarWorkState::Pending.as_str())
                    .bind(CALENDAR_WORK_ATTEMPTS_EXHAUSTED_CODE)
                    .bind(&now)
                    .bind(CalendarWorkState::Leased.as_str())
                    .execute(&self.pool)
                    .await?;

                Ok(result.rows_affected())
            },
        )
        .await
    }

    /// Liste les travaux a l'etat pending, y compris ceux dont le retry est planifie dans le futur.
    pub(crate) async fn pending(
        &self,
        kind: Option<CalendarWorkKind>,
        limit: usize,
    ) -> Result<Vec<CalendarWork>, AppError> {
        crate::features::calendar_logs::operation("pending", serde_json::json!({"kind": format!("{:?}", kind), "limit": format!("{:?}", limit)}), async {

        let limit = checked_limit(
            limit,
            MAX_PENDING_QUERY_SIZE,
            "calendar_work_invalid_pending_limit",
        )?;
        let rows = match kind {
            Some(kind) => {
                let select = format!(
                    "SELECT {CALENDAR_WORK_SELECT} WHERE status = ?1 AND kind = ?2 ORDER BY available_at ASC, id ASC LIMIT ?3"
                );
                sqlx::query_as::<_, CalendarWorkRow>(&select)
                    .bind(CalendarWorkState::Pending.as_str())
                    .bind(kind.as_str())
                    .bind(limit)
                    .fetch_all(&self.pool)
                    .await?
            }
            None => {
                let select = format!(
                    "SELECT {CALENDAR_WORK_SELECT} WHERE status = ?1 ORDER BY available_at ASC, id ASC LIMIT ?2"
                );
                sqlx::query_as::<_, CalendarWorkRow>(&select)
                    .bind(CalendarWorkState::Pending.as_str())
                    .bind(limit)
                    .fetch_all(&self.pool)
                    .await?
            }
        };

        rows.into_iter().map(TryInto::try_into).collect()

}).await
    }

    /// Indique qu'une ingestion du jour a ete arretee apres un refus source.
    ///
    /// Ce verrou est distinct des audits detailes : il reste utilisable si la
    /// table d'audit ou son artifact ne peut momentanement pas etre ecrit.
    /// L'admission le consulte avant de creer un nouveau run, ce qui empeche
    /// une re-collecte apres redemarrage sans conserver de contenu source.
    pub(crate) async fn has_terminal_ingestion_source_refusal(
        &self,
        local_day: &str,
    ) -> Result<bool, AppError> {
        crate::features::calendar_logs::operation(
            "has_terminal_ingestion_source_refusal",
            serde_json::json!({"local_day": format!("{:?}", local_day)}),
            async {
                let exists: i64 = sqlx::query_scalar(&format!(
                    r#"
            SELECT EXISTS (
                SELECT 1
                FROM {CALENDAR_WORK_TABLE}
                WHERE kind = ?1
                  AND local_day = ?2
                  AND status = ?3
                  AND last_error_code = ?4
            )
            "#,
                ))
                .bind(CalendarWorkKind::Ingestion.as_str())
                .bind(local_day)
                .bind(CalendarWorkState::DeadLetter.as_str())
                .bind(CALENDAR_SOURCE_REFUSAL_TERMINAL_ERROR)
                .fetch_one(&self.pool)
                .await?;
                Ok(exists != 0)
            },
        )
        .await
    }

    /// Liste uniquement les pending immediatement reclamables a l'instant fourni.
    pub(crate) async fn pending_ready(
        &self,
        kind: Option<CalendarWorkKind>,
        now: OffsetDateTime,
        limit: usize,
    ) -> Result<Vec<CalendarWork>, AppError> {
        crate::features::calendar_logs::operation("pending_ready", serde_json::json!({"kind": format!("{:?}", kind), "now": format!("{:?}", now), "limit": format!("{:?}", limit)}), async {

        let limit = checked_limit(
            limit,
            MAX_PENDING_QUERY_SIZE,
            "calendar_work_invalid_pending_limit",
        )?;
        let now = storage_timestamp(now)?;
        let rows = match kind {
            Some(kind) => {
                let select = format!(
                    "SELECT {CALENDAR_WORK_SELECT} WHERE status = ?1 AND kind = ?2 AND available_at <= ?3 ORDER BY available_at ASC, id ASC LIMIT ?4"
                );
                sqlx::query_as::<_, CalendarWorkRow>(&select)
                    .bind(CalendarWorkState::Pending.as_str())
                    .bind(kind.as_str())
                    .bind(&now)
                    .bind(limit)
                    .fetch_all(&self.pool)
                    .await?
            }
            None => {
                let select = format!(
                    "SELECT {CALENDAR_WORK_SELECT} WHERE status = ?1 AND available_at <= ?2 ORDER BY available_at ASC, id ASC LIMIT ?3"
                );
                sqlx::query_as::<_, CalendarWorkRow>(&select)
                    .bind(CalendarWorkState::Pending.as_str())
                    .bind(&now)
                    .bind(limit)
                    .fetch_all(&self.pool)
                    .await?
            }
        };

        rows.into_iter().map(TryInto::try_into).collect()

}).await
    }
}

/// Admission sur pool autonome, avec une borne atomique des intentions encore
/// ouvertes. Une cle existante est toujours relue avant le plafond afin que le
/// replay idempotent reste possible lorsque la file est pleine.
async fn admit_with_pool(
    pool: &SqlitePool,
    max_open_work: usize,
    request: &CalendarWorkRequest,
    now: &str,
) -> Result<CalendarWorkAdmission, AppError> {
    let select =
        format!("SELECT {CALENDAR_WORK_SELECT} WHERE kind = ?1 AND idempotency_key = ?2 LIMIT 1");
    if let Some(existing) = sqlx::query_as::<_, CalendarWorkRow>(&select)
        .bind(request.kind().as_str())
        .bind(request.idempotency_key())
        .fetch_optional(pool)
        .await?
    {
        return existing_admission(existing, request);
    }

    let max_open_work = i64::try_from(max_open_work)
        .map_err(|_| AppError::storage("calendar_work_open_work_limit_overflow"))?;
    let inserted = sqlx::query(
        r#"
        INSERT INTO calendar_native_work (
            kind,
            local_day,
            idempotency_key,
            status,
            attempt_count,
            available_at,
            lease_generation,
            lease_owner,
            lease_expires_at,
            last_error_code,
            reserved_stream_id,
            created_at,
            updated_at,
            finished_at
        )
        SELECT ?1, ?2, ?3, ?4, 0, ?5, 0, NULL, NULL, NULL, NULL, ?5, ?5, NULL
        WHERE (
            SELECT COUNT(*)
            FROM calendar_native_work
            WHERE status IN ('pending', 'leased')
        ) < ?6
        ON CONFLICT (kind, idempotency_key) DO NOTHING;
        "#,
    )
    .bind(request.kind().as_str())
    .bind(request.local_day())
    .bind(request.idempotency_key())
    .bind(CalendarWorkState::Pending.as_str())
    .bind(now)
    .bind(max_open_work)
    .execute(pool)
    .await?;

    let work = sqlx::query_as::<_, CalendarWorkRow>(&select)
        .bind(request.kind().as_str())
        .bind(request.idempotency_key())
        .fetch_optional(pool)
        .await?;
    match work {
        Some(work) if inserted.rows_affected() == 1 => {
            Ok(CalendarWorkAdmission::Inserted(work.try_into()?))
        }
        Some(work) => existing_admission(work, request),
        None => Err(AppError::conflict("calendar_native_work_queue_full")),
    }
}

/// Meme admission, mais sur la transaction qui porte le commit calendrier.
async fn admit_with_tx(
    tx: &mut Transaction<'_, Sqlite>,
    max_open_work: usize,
    request: &CalendarWorkRequest,
    now: &str,
) -> Result<CalendarWorkAdmission, AppError> {
    let select =
        format!("SELECT {CALENDAR_WORK_SELECT} WHERE kind = ?1 AND idempotency_key = ?2 LIMIT 1");
    if let Some(existing) = sqlx::query_as::<_, CalendarWorkRow>(&select)
        .bind(request.kind().as_str())
        .bind(request.idempotency_key())
        .fetch_optional(&mut **tx)
        .await?
    {
        return existing_admission(existing, request);
    }

    let max_open_work = i64::try_from(max_open_work)
        .map_err(|_| AppError::storage("calendar_work_open_work_limit_overflow"))?;
    let inserted = sqlx::query(
        r#"
        INSERT INTO calendar_native_work (
            kind,
            local_day,
            idempotency_key,
            status,
            attempt_count,
            available_at,
            lease_generation,
            lease_owner,
            lease_expires_at,
            last_error_code,
            reserved_stream_id,
            created_at,
            updated_at,
            finished_at
        )
        SELECT ?1, ?2, ?3, ?4, 0, ?5, 0, NULL, NULL, NULL, NULL, ?5, ?5, NULL
        WHERE (
            SELECT COUNT(*)
            FROM calendar_native_work
            WHERE status IN ('pending', 'leased')
        ) < ?6
        ON CONFLICT (kind, idempotency_key) DO NOTHING;
        "#,
    )
    .bind(request.kind().as_str())
    .bind(request.local_day())
    .bind(request.idempotency_key())
    .bind(CalendarWorkState::Pending.as_str())
    .bind(now)
    .bind(max_open_work)
    .execute(&mut **tx)
    .await?;

    let work = sqlx::query_as::<_, CalendarWorkRow>(&select)
        .bind(request.kind().as_str())
        .bind(request.idempotency_key())
        .fetch_optional(&mut **tx)
        .await?;
    match work {
        Some(work) if inserted.rows_affected() == 1 => {
            Ok(CalendarWorkAdmission::Inserted(work.try_into()?))
        }
        Some(work) => existing_admission(work, request),
        None => Err(AppError::conflict("calendar_native_work_queue_full")),
    }
}

/// Attache une reservation deja connue a une nouvelle intention audio avant
/// son premier claim. Cette ecriture reste dans la transaction d'admission :
/// aucun worker ne peut observer le travail sans le stream historique a
/// reutiliser.
async fn reserve_pending_stream_in_tx(
    tx: &mut Transaction<'_, Sqlite>,
    work_id: i64,
    preferred_stream_id: Option<&str>,
    now: &str,
) -> Result<CalendarWork, AppError> {
    let statement = format!(
        r#"
        UPDATE {CALENDAR_WORK_TABLE}
        SET reserved_stream_id = COALESCE(reserved_stream_id, ?1),
            updated_at = ?2
        WHERE id = ?3
          AND status = ?4
        RETURNING {CALENDAR_WORK_COLUMNS}
        "#,
    );
    let row = sqlx::query_as::<_, CalendarWorkRow>(&statement)
        .bind(preferred_stream_id)
        .bind(now)
        .bind(work_id)
        .bind(CalendarWorkState::Pending.as_str())
        .fetch_optional(&mut **tx)
        .await?;
    row.map(TryInto::try_into)
        .transpose()?
        .ok_or_else(|| AppError::conflict("calendar_work_reactivation_conflict"))
}

/// Transforme une ligne deja presente en admission idempotente apres avoir
/// verifie qu'elle garde bien le meme jour local.
fn existing_admission(
    row: CalendarWorkRow,
    request: &CalendarWorkRequest,
) -> Result<CalendarWorkAdmission, AppError> {
    let work: CalendarWork = row.try_into()?;
    if work.local_day != request.local_day() {
        return Err(AppError::conflict(
            "calendar_work_idempotency_key_conflicts_with_local_day",
        ));
    }
    Ok(CalendarWorkAdmission::Existing(work))
}

/// Ligne SQLx privee, convertie apres controle des invariants de stockage.
#[derive(Debug, sqlx::FromRow)]
struct CalendarWorkRow {
    id: i64,
    kind: String,
    local_day: String,
    idempotency_key: String,
    status: String,
    attempt_count: i64,
    available_at: String,
    lease_generation: i64,
    lease_owner: Option<String>,
    lease_expires_at: Option<String>,
    last_error_code: Option<String>,
    reserved_stream_id: Option<String>,
    created_at: String,
    updated_at: String,
    finished_at: Option<String>,
}

impl TryFrom<CalendarWorkRow> for CalendarWork {
    type Error = AppError;

    fn try_from(row: CalendarWorkRow) -> Result<Self, Self::Error> {
        let kind = CalendarWorkKind::from_storage(&row.kind)?;
        let state = CalendarWorkState::from_storage(&row.status)?;
        let attempt_count = u32::try_from(row.attempt_count)
            .map_err(|_| AppError::storage("calendar_native_work has an invalid attempt count"))?;
        let lease_generation = u64::try_from(row.lease_generation).map_err(|_| {
            AppError::storage("calendar_native_work has an invalid lease generation")
        })?;

        let lease_fields_valid = match state {
            CalendarWorkState::Pending => {
                row.lease_owner.is_none()
                    && row.lease_expires_at.is_none()
                    && row.finished_at.is_none()
            }
            CalendarWorkState::Leased => {
                row.lease_owner.is_some()
                    && row.lease_expires_at.is_some()
                    && row.finished_at.is_none()
            }
            CalendarWorkState::Completed | CalendarWorkState::DeadLetter => {
                row.lease_owner.is_none()
                    && row.lease_expires_at.is_none()
                    && row.finished_at.is_some()
            }
        };
        if !lease_fields_valid {
            return Err(AppError::storage(
                "calendar_native_work violates its state invariants",
            ));
        }

        Ok(Self {
            id: row.id,
            kind,
            local_day: row.local_day,
            idempotency_key: row.idempotency_key,
            state,
            attempt_count,
            available_at: row.available_at,
            lease_generation,
            lease_owner: row.lease_owner,
            lease_expires_at: row.lease_expires_at,
            last_error_code: row.last_error_code,
            reserved_stream_id: row.reserved_stream_id,
            created_at: row.created_at,
            updated_at: row.updated_at,
            finished_at: row.finished_at,
        })
    }
}

/// Convertit la reponse `RETURNING` commune aux mutations fencees.
fn mutation_from_optional_row(
    row: Option<CalendarWorkRow>,
) -> Result<CalendarWorkLeaseMutation, AppError> {
    match row {
        Some(row) => Ok(CalendarWorkLeaseMutation::Applied(row.try_into()?)),
        None => Ok(CalendarWorkLeaseMutation::FencedOut),
    }
}

/// Formate chaque instant en UTC RFC3339 pour conserver l'ordre lexicographique SQLite.
fn storage_timestamp(timestamp: OffsetDateTime) -> Result<String, AppError> {
    timestamp
        .to_offset(UtcOffset::UTC)
        .format(&Rfc3339)
        .map_err(|_| AppError::storage("calendar_work_timestamp_format_failed"))
}

/// Calcule la borne d'une lease en refusant les durees vides ou disproportionnees.
fn lease_expires_at(now: OffsetDateTime, lease_for: Duration) -> Result<OffsetDateTime, AppError> {
    if lease_for.is_zero() || lease_for > MAX_LEASE_DURATION {
        return Err(AppError::Config(
            "calendar_work_lease_duration_must_be_between_one_nanosecond_and_24_hours".to_owned(),
        ));
    }

    let seconds = i64::try_from(lease_for.as_secs()).map_err(|_| {
        AppError::storage("calendar_work_lease_duration_does_not_fit_sqlite_timestamp")
    })?;
    let nanos = i32::try_from(lease_for.subsec_nanos())
        .map_err(|_| AppError::storage("calendar_work_lease_duration_has_invalid_nanoseconds"))?;
    now.checked_add(TimeDuration::new(seconds, nanos))
        .ok_or_else(|| AppError::storage("calendar_work_lease_expiration_overflow"))
}

/// Valide les limites de batch afin de garder les operations SQLite predictibles.
fn checked_limit(limit: usize, maximum: usize, error_code: &'static str) -> Result<i64, AppError> {
    if !(1..=maximum).contains(&limit) {
        return Err(AppError::Config(error_code.to_owned()));
    }

    i64::try_from(limit).map_err(|_| AppError::storage("calendar_work_query_limit_overflow"))
}

/// Convertit la generation Rust dans le type entier SQLite, sans troncature silencieuse.
fn storage_generation(generation: u64) -> Result<i64, AppError> {
    i64::try_from(generation)
        .map_err(|_| AppError::storage("calendar_work_lease_generation_overflow"))
}

/// Valide une valeur technique, pas un texte de travail: pas d'espace, ni de contenu libre long.
fn normalize_technical_identifier(
    value: &str,
    maximum_bytes: usize,
    error_code: &'static str,
) -> Result<String, AppError> {
    let value = value.trim();
    let valid = !value.is_empty()
        && value.len() <= maximum_bytes
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':' | b'/')
        });
    if !valid {
        return Err(AppError::Config(error_code.to_owned()));
    }

    Ok(value.to_owned())
}

/// Retient uniquement un identifiant HLS historique qui peut etre remis dans
/// la file durable. Une valeur legacy malformee est traitee comme absente : le
/// runtime generera alors son candidat deterministe plutot que de propager une
/// reference invalide vers Piper/HLS.
pub(crate) fn valid_reserved_stream_id(value: Option<&str>) -> Option<String> {
    value.and_then(|value| {
        normalize_technical_identifier(
            value,
            MAX_RESERVED_STREAM_ID_BYTES,
            "calendar_work_invalid_reserved_stream_id",
        )
        .ok()
    })
}

/// Valide strictement le jour ISO local qui sert de cle de relecture aux consumers.
fn normalize_local_day(value: &str) -> Result<String, AppError> {
    let value = value.trim();
    let bytes = value.as_bytes();
    let valid_shape = bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| matches!(index, 4 | 7) || byte.is_ascii_digit());
    if !valid_shape {
        return Err(AppError::Config(
            "calendar_work_invalid_local_day".to_owned(),
        ));
    }

    let year = parse_day_component(&bytes[0..4])?;
    let month = parse_day_component(&bytes[5..7])?;
    let day = parse_day_component(&bytes[8..10])?;
    let month = Month::try_from(
        u8::try_from(month)
            .map_err(|_| AppError::Config("calendar_work_invalid_local_day".to_owned()))?,
    )
    .map_err(|_| AppError::Config("calendar_work_invalid_local_day".to_owned()))?;
    Date::from_calendar_date(
        year,
        month,
        u8::try_from(day)
            .map_err(|_| AppError::Config("calendar_work_invalid_local_day".to_owned()))?,
    )
    .map_err(|_| AppError::Config("calendar_work_invalid_local_day".to_owned()))?;

    Ok(value.to_owned())
}

/// Parse une petite tranche ASCII deja controlee sans rendre son contenu dans l'erreur.
fn parse_day_component(bytes: &[u8]) -> Result<i32, AppError> {
    let mut value = 0_i32;
    for byte in bytes {
        value = value
            .checked_mul(10)
            .and_then(|value| value.checked_add(i32::from(*byte - b'0')))
            .ok_or_else(|| AppError::Config("calendar_work_invalid_local_day".to_owned()))?;
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn test_store() -> CalendarWorkStore {
        let pool = SqlitePoolOptions::new()
            // `sqlite::memory:` appartient a une seule connexion: le plafond garde le schema commun au test.
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("sqlite memory pool");
        let store = CalendarWorkStore::new(pool);
        store.apply_schema().await.expect("calendar work schema");
        store
    }

    fn instant(seconds: i64) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(seconds).expect("test instant")
    }

    fn request(kind: CalendarWorkKind, key: &str) -> CalendarWorkRequest {
        CalendarWorkRequest::try_new(kind, "2026-08-30", key).expect("valid work request")
    }

    #[tokio::test]
    async fn admission_is_idempotent_and_does_not_store_business_payload() {
        let store = test_store().await;
        let now = instant(1_780_000_000);
        let request = request(CalendarWorkKind::Ingestion, "run-2026-08-30");

        let first = store.admit(&request, now).await.expect("first admission");
        let second = store
            .admit(&request, now)
            .await
            .expect("replayed admission");

        assert!(first.inserted());
        assert!(!second.inserted());
        assert_eq!(first.work().id, second.work().id);
        assert_eq!(first.work().state, CalendarWorkState::Pending);
        assert_eq!(store.pending(None, 8).await.expect("pending list").len(), 1);

        let columns: Vec<(String,)> =
            sqlx::query_as("SELECT name FROM pragma_table_info('calendar_native_work')")
                .fetch_all(&store.pool)
                .await
                .expect("schema columns");
        assert!(columns.iter().all(|(column,)| !column.contains("payload")));
        assert!(columns.iter().all(|(column,)| !column.contains("json")));
    }

    #[tokio::test]
    async fn open_work_capacity_is_bounded_without_rejecting_an_idempotent_replay() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("sqlite memory pool");
        let store = CalendarWorkStore::with_max_open_work(pool, 1)
            .expect("a one-work bound must be accepted");
        store.apply_schema().await.expect("calendar work schema");
        let now = instant(1_780_000_000);
        let first = request(CalendarWorkKind::Ingestion, "bounded-first");

        assert!(
            store
                .admit(&first, now)
                .await
                .expect("first work fits the bound")
                .inserted()
        );
        assert!(matches!(
            store
                .admit(&request(CalendarWorkKind::Summary, "bounded-second"), now)
                .await,
            Err(AppError::Conflict(message)) if message == "calendar_native_work_queue_full"
        ));
        assert!(
            !store
                .admit(&first, now)
                .await
                .expect("the existing idempotency key remains replayable when full")
                .inserted()
        );
    }

    #[tokio::test]
    async fn completed_summary_audio_reactivation_resets_attempts_and_preserves_stream() {
        let store = test_store().await;
        let now = instant(1_780_000_000);
        let request = request(CalendarWorkKind::SummaryAudio, "summary-42");
        store.admit(&request, now).await.expect("audio admission");
        let claimed = store
            .claim("worker-a", now, Duration::from_secs(30), 1)
            .await
            .expect("audio claim")
            .pop()
            .expect("one audio work");
        let reserved = store
            .reserve_stream_id(
                &claimed.lease,
                "calendar-summary-stream-42",
                now + TimeDuration::seconds(1),
            )
            .await
            .expect("stream reservation");
        assert!(matches!(reserved, CalendarWorkLeaseMutation::Applied(_)));
        let completed = store
            .complete(&claimed.lease, now + TimeDuration::seconds(2))
            .await
            .expect("audio completion");
        assert!(matches!(completed, CalendarWorkLeaseMutation::Applied(_)));

        let mut tx = store.pool.begin().await.expect("repair transaction");
        let reactivated = store
            .reactivate_audio_for_repair_in_tx(
                &mut tx,
                &request,
                Some("different-historical-stream"),
                now + TimeDuration::seconds(3),
            )
            .await
            .expect("reactivate completed audio work");
        tx.commit().await.expect("repair commit");

        assert!(reactivated.reactivated());
        assert_eq!(reactivated.work().state, CalendarWorkState::Pending);
        assert_eq!(reactivated.work().attempt_count, 0);
        assert_eq!(
            reactivated.work().reserved_stream_id.as_deref(),
            Some("calendar-summary-stream-42")
        );
        assert!(reactivated.work().finished_at.is_none());

        let reclaimed = store
            .claim(
                "worker-b",
                now + TimeDuration::seconds(3),
                Duration::from_secs(30),
                1,
            )
            .await
            .expect("reactivated claim")
            .pop()
            .expect("one reactivated work");
        assert_eq!(reclaimed.work.attempt_count, 1);
        assert_eq!(
            reclaimed.work.reserved_stream_id.as_deref(),
            Some("calendar-summary-stream-42")
        );
    }

    #[tokio::test]
    async fn closed_audio_reactivation_keeps_it_closed_when_capacity_is_full() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("sqlite memory pool");
        let store = CalendarWorkStore::with_max_open_work(pool, 1).expect("one-work bound");
        store.apply_schema().await.expect("calendar work schema");
        let now = instant(1_780_000_000);
        let closed = request(CalendarWorkKind::SummaryAudio, "summary-43");
        store.admit(&closed, now).await.expect("audio admission");
        let claimed = store
            .claim("worker-a", now, Duration::from_secs(30), 1)
            .await
            .expect("audio claim")
            .pop()
            .expect("one work");
        store
            .complete(&claimed.lease, now + TimeDuration::seconds(1))
            .await
            .expect("completion");
        store
            .admit(
                &request(CalendarWorkKind::Ingestion, "open-work"),
                now + TimeDuration::seconds(2),
            )
            .await
            .expect("open work fills capacity");

        let mut tx = store.pool.begin().await.expect("repair transaction");
        let result = store
            .reactivate_audio_for_repair_in_tx(
                &mut tx,
                &closed,
                Some("calendar-summary-stream-43"),
                now + TimeDuration::seconds(3),
            )
            .await;
        assert!(matches!(
            result,
            Err(AppError::Conflict(message)) if message == "calendar_native_work_queue_full"
        ));
        drop(tx);

        let row: (String, i64, Option<String>) = sqlx::query_as(
            "SELECT status, attempt_count, reserved_stream_id FROM calendar_native_work WHERE kind = 'summary_audio' AND idempotency_key = 'summary-43'",
        )
        .fetch_one(&store.pool)
        .await
        .expect("closed audio row");
        assert_eq!(row.0, "completed");
        assert_eq!(row.1, 1);
        assert!(row.2.is_none());
    }

    #[tokio::test]
    async fn claim_and_complete_are_fenced_by_generation_and_owner() {
        let store = test_store().await;
        let admitted_at = instant(1_780_000_000);
        store
            .admit(
                &request(CalendarWorkKind::Summary, "summary-run"),
                admitted_at,
            )
            .await
            .expect("admission");

        let first = store
            .claim("worker-a", admitted_at, Duration::from_secs(10), 1)
            .await
            .expect("first claim")
            .pop()
            .expect("leased work");
        assert_eq!(first.lease.generation(), 1);

        assert_eq!(
            store
                .recover_expired_leases(admitted_at + TimeDuration::seconds(11))
                .await
                .expect("lease recovery"),
            1
        );
        let second = store
            .claim(
                "worker-b",
                admitted_at + TimeDuration::seconds(11),
                Duration::from_secs(10),
                1,
            )
            .await
            .expect("second claim")
            .pop()
            .expect("reclaimed work");
        assert_eq!(second.lease.generation(), 2);

        assert_eq!(
            store
                .complete(&first.lease, admitted_at + TimeDuration::seconds(12))
                .await
                .expect("stale completion verdict"),
            CalendarWorkLeaseMutation::FencedOut
        );
        let completed = store
            .complete(&second.lease, admitted_at + TimeDuration::seconds(12))
            .await
            .expect("current completion");
        let CalendarWorkLeaseMutation::Applied(completed) = completed else {
            panic!("current lease must complete the work");
        };
        assert_eq!(completed.state, CalendarWorkState::Completed);
        assert_eq!(completed.attempt_count, 2);
    }

    #[tokio::test]
    async fn retry_keeps_work_pending_until_its_scheduled_time() {
        let store = test_store().await;
        let now = instant(1_780_000_000);
        store
            .admit(&request(CalendarWorkKind::Ingestion, "retry-run"), now)
            .await
            .expect("admission");
        let claimed = store
            .claim("worker-a", now, Duration::from_secs(30), 1)
            .await
            .expect("claim")
            .pop()
            .expect("leased work");
        let error = CalendarWorkErrorCode::try_new("provider_unavailable").expect("error code");
        let retry_at = now + TimeDuration::seconds(60);

        let retry = store
            .retry(
                &claimed.lease,
                &error,
                retry_at,
                now + TimeDuration::seconds(1),
            )
            .await
            .expect("retry");
        let CalendarWorkLeaseMutation::Applied(retried) = retry else {
            panic!("current lease must schedule retry");
        };
        assert_eq!(retried.state, CalendarWorkState::Pending);
        assert_eq!(
            retried.last_error_code.as_deref(),
            Some("provider_unavailable")
        );
        assert_eq!(store.pending(None, 8).await.expect("all pending").len(), 1);
        assert!(
            store
                .pending_ready(None, now + TimeDuration::seconds(59), 8)
                .await
                .expect("not ready before retry")
                .is_empty()
        );
        assert_eq!(
            store
                .pending_ready(None, retry_at, 8)
                .await
                .expect("ready at retry")
                .len(),
            1
        );
    }

    #[tokio::test]
    async fn expired_lease_recovery_requeues_work_and_invalidates_old_token() {
        let store = test_store().await;
        let now = instant(1_780_000_000);
        store
            .admit(&request(CalendarWorkKind::ItemAudio, "recovery-run"), now)
            .await
            .expect("admission");
        let claimed = store
            .claim("worker-a", now, Duration::from_secs(5), 1)
            .await
            .expect("claim")
            .pop()
            .expect("leased work");
        let reserved = store
            .reserve_stream_id(
                &claimed.lease,
                "calendar-stream-a",
                now + TimeDuration::seconds(1),
            )
            .await
            .expect("stream reservation");
        let CalendarWorkLeaseMutation::Applied(reserved) = reserved else {
            panic!("current lease must reserve a stream");
        };
        assert_eq!(
            reserved.reserved_stream_id.as_deref(),
            Some("calendar-stream-a")
        );
        let after_expiry = now + TimeDuration::seconds(6);

        assert_eq!(
            store
                .complete(&claimed.lease, after_expiry)
                .await
                .expect("expired completion verdict"),
            CalendarWorkLeaseMutation::FencedOut
        );
        assert_eq!(
            store
                .reserve_stream_id(&claimed.lease, "calendar-stream-b", after_expiry)
                .await
                .expect("expired stream reservation verdict"),
            CalendarWorkLeaseMutation::FencedOut
        );
        assert_eq!(
            store
                .recover_expired_leases(after_expiry)
                .await
                .expect("recovery"),
            1
        );
        let pending = store.pending(None, 8).await.expect("recovered pending");
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].state, CalendarWorkState::Pending);
        assert_eq!(pending[0].last_error_code.as_deref(), Some("lease_expired"));
        assert_eq!(
            pending[0].reserved_stream_id.as_deref(),
            Some("calendar-stream-a")
        );

        let reclaimed = store
            .claim("worker-b", after_expiry, Duration::from_secs(5), 1)
            .await
            .expect("reclaim")
            .pop()
            .expect("reclaimed work");
        assert!(reclaimed.lease.generation() > claimed.lease.generation());
        assert_eq!(reclaimed.work.attempt_count, 2);
        let reused = store
            .reserve_stream_id(
                &reclaimed.lease,
                "calendar-stream-different-candidate",
                after_expiry + TimeDuration::seconds(1),
            )
            .await
            .expect("stable stream lookup");
        let CalendarWorkLeaseMutation::Applied(reused) = reused else {
            panic!("current lease must read existing stream reservation");
        };
        assert_eq!(
            reused.reserved_stream_id.as_deref(),
            Some("calendar-stream-a")
        );
    }

    #[tokio::test]
    async fn repeated_expired_leases_stop_at_the_durable_attempt_budget() {
        let store = test_store().await;
        let admitted_at = instant(1_780_000_000);
        let request = request(CalendarWorkKind::SummaryAudio, "expired-budget");
        let admission = store.admit(&request, admitted_at).await.expect("admission");
        let work_id = admission.work().id;
        let mut last_expiry = admitted_at;

        for attempt in 1..=CALENDAR_WORK_MAX_ATTEMPTS {
            let claimed = store
                .claim(
                    format!("worker-{attempt}"),
                    last_expiry,
                    Duration::from_secs(5),
                    1,
                )
                .await
                .expect("claim")
                .pop()
                .expect("work remains claimable until its final expiry");
            assert_eq!(claimed.work.attempt_count, attempt);
            assert_eq!(claimed.lease.generation(), u64::from(attempt));

            if attempt == 1 {
                let reserved = store
                    .reserve_stream_id(
                        &claimed.lease,
                        "calendar-summary-expired-budget",
                        last_expiry + TimeDuration::seconds(1),
                    )
                    .await
                    .expect("reserve stream before crash");
                assert!(matches!(reserved, CalendarWorkLeaseMutation::Applied(_)));
            }

            last_expiry += TimeDuration::seconds(6);
            if attempt < CALENDAR_WORK_MAX_ATTEMPTS {
                assert_eq!(
                    store
                        .recover_expired_leases(last_expiry)
                        .await
                        .expect("recover expired lease"),
                    1
                );
            } else {
                // Le claim lui-meme doit fermer une lease expiree au dernier
                // essai, meme si le reveil periodique de recovery n'a pas eu
                // le temps de passer apres le crash.
                assert!(
                    store
                        .claim(
                            "worker-after-final-expiry",
                            last_expiry,
                            Duration::from_secs(5),
                            1,
                        )
                        .await
                        .expect("claim repairs an expired final lease")
                        .is_empty()
                );
            }
            assert_eq!(
                store
                    .complete(&claimed.lease, last_expiry)
                    .await
                    .expect("expired owner is fenced out"),
                CalendarWorkLeaseMutation::FencedOut
            );

            if attempt < CALENDAR_WORK_MAX_ATTEMPTS {
                let pending = store.pending(None, 8).await.expect("recovered pending");
                assert_eq!(pending.len(), 1);
                assert_eq!(pending[0].attempt_count, attempt);
                assert_eq!(pending[0].last_error_code.as_deref(), Some("lease_expired"));
            }
        }

        let terminal: (String, i64, i64, Option<String>, Option<String>, Option<String>) =
            sqlx::query_as(
                "SELECT status, attempt_count, lease_generation, last_error_code, reserved_stream_id, finished_at FROM calendar_native_work WHERE id = ?1",
            )
            .bind(work_id)
            .fetch_one(&store.pool)
            .await
            .expect("terminal work row");
        assert_eq!(terminal.0, "dead_letter");
        assert_eq!(terminal.1, i64::from(CALENDAR_WORK_MAX_ATTEMPTS));
        assert_eq!(terminal.2, i64::from(CALENDAR_WORK_MAX_ATTEMPTS));
        assert_eq!(
            terminal.3.as_deref(),
            Some(CALENDAR_WORK_ATTEMPTS_EXHAUSTED_CODE)
        );
        assert_eq!(
            terminal.4.as_deref(),
            Some("calendar-summary-expired-budget")
        );
        assert!(terminal.5.is_some());
        assert!(
            store
                .pending(None, 8)
                .await
                .expect("no pending work")
                .is_empty()
        );
        assert!(
            store
                .claim(
                    "worker-after-budget",
                    last_expiry,
                    Duration::from_secs(5),
                    1,
                )
                .await
                .expect("claim after terminal recovery")
                .is_empty()
        );

        let replay = store
            .admit(&request, last_expiry)
            .await
            .expect("idempotent replay");
        assert!(!replay.inserted());
        assert_eq!(replay.work().id, work_id);
        assert_eq!(replay.work().state, CalendarWorkState::DeadLetter);
        assert_eq!(replay.work().attempt_count, CALENDAR_WORK_MAX_ATTEMPTS);
    }

    #[tokio::test]
    async fn claim_terminalizes_a_pending_row_already_at_the_attempt_budget() {
        let store = test_store().await;
        let now = instant(1_780_000_000);
        let request = request(CalendarWorkKind::Ingestion, "claim-budget");
        let admission = store.admit(&request, now).await.expect("admission");

        sqlx::query(
            "UPDATE calendar_native_work SET attempt_count = ?1 WHERE id = ?2 AND status = 'pending'",
        )
        .bind(i64::from(CALENDAR_WORK_MAX_ATTEMPTS))
        .bind(admission.work().id)
        .execute(&store.pool)
        .await
        .expect("seed legacy exhausted pending row");

        assert!(
            store
                .claim("worker-claim", now, Duration::from_secs(30), 1)
                .await
                .expect("claim also repairs exhausted pending work")
                .is_empty()
        );

        let terminal: (String, i64, i64, Option<String>, Option<String>, Option<String>) =
            sqlx::query_as(
                "SELECT status, attempt_count, lease_generation, last_error_code, lease_owner, finished_at FROM calendar_native_work WHERE id = ?1",
            )
            .bind(admission.work().id)
            .fetch_one(&store.pool)
            .await
            .expect("dead-letter row");
        assert_eq!(terminal.0, "dead_letter");
        assert_eq!(terminal.1, i64::from(CALENDAR_WORK_MAX_ATTEMPTS));
        assert_eq!(terminal.2, 0, "claim must not create a new fencing token");
        assert_eq!(
            terminal.3.as_deref(),
            Some(CALENDAR_WORK_ATTEMPTS_EXHAUSTED_CODE)
        );
        assert!(terminal.4.is_none());
        assert!(terminal.5.is_some());

        let replay = store
            .admit(&request, now + TimeDuration::seconds(1))
            .await
            .expect("idempotent replay remains terminal");
        assert!(!replay.inserted());
        assert_eq!(replay.work().state, CalendarWorkState::DeadLetter);
        assert_eq!(replay.work().attempt_count, CALENDAR_WORK_MAX_ATTEMPTS);
    }

    #[tokio::test]
    async fn retry_at_the_attempt_budget_is_atomically_dead_lettered() {
        let store = test_store().await;
        let now = instant(1_780_000_000);
        store
            .admit(&request(CalendarWorkKind::Summary, "retry-budget"), now)
            .await
            .expect("admission");
        let claimed = store
            .claim("worker-retry", now, Duration::from_secs(30), 1)
            .await
            .expect("claim")
            .pop()
            .expect("one leased work");

        sqlx::query(
            "UPDATE calendar_native_work SET attempt_count = ?1 WHERE id = ?2 AND status = 'leased'",
        )
        .bind(i64::from(CALENDAR_WORK_MAX_ATTEMPTS))
        .bind(claimed.work.id)
        .execute(&store.pool)
        .await
        .expect("seed exhausted leased row");
        let error = CalendarWorkErrorCode::try_new("provider_unavailable").expect("error code");

        let result = store
            .retry(
                &claimed.lease,
                &error,
                now + TimeDuration::seconds(10),
                now + TimeDuration::seconds(1),
            )
            .await
            .expect("budget-aware retry");
        let CalendarWorkLeaseMutation::Applied(work) = result else {
            panic!("current exhausted lease must be terminalized");
        };
        assert_eq!(work.state, CalendarWorkState::DeadLetter);
        assert_eq!(work.attempt_count, CALENDAR_WORK_MAX_ATTEMPTS);
        assert_eq!(
            work.last_error_code.as_deref(),
            Some(CALENDAR_WORK_ATTEMPTS_EXHAUSTED_CODE)
        );
        assert!(work.finished_at.is_some());
        assert!(work.lease_owner.is_none());
        assert!(work.lease_expires_at.is_none());
    }
}
