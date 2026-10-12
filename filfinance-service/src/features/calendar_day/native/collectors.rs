//! Full payload diagnostics are emitted only through calendar_logs when scoped by a calendar operation.
//! Collecte HTTP bornee et sans effet de bord pour le calendrier natif.
//!
//! Le chemin de production ne connait que les huit flux explicitement autorises. Les corps de
//! reponse ne sont ni serialisables ni affichables par `Debug`; seul un audit a base de statut,
//! hash et longueur peut quitter ce module. Un ensemble preserve l'ordre des collecteurs et
//! execute toutes les sources autorisees pour le jour demande.

use std::{
    collections::{HashMap, HashSet},
    fmt,
    str::FromStr,
    time::{Duration, Instant},
};

use super::feeds::{self, CalendarFeedDocument, CalendarFeedSource};
use reqwest::{Client, Response, StatusCode, header::HeaderMap, redirect::Policy};
use serde::Serialize;
use sha2::{Digest, Sha256};
use time::{Date, Month};
use url::Url;

/// Le timeout total couvre la connexion, les en-tetes et la lecture du corps de chaque source.
pub(crate) const NATIVE_CALENDAR_HTTP_TIMEOUT: Duration = Duration::from_secs(20);
/// Le plafond est compte en octets recus apres les decompresseurs HTTP eventuels de Reqwest.
pub(crate) const NATIVE_CALENDAR_MAX_BODY_BYTES: usize = 1_000_000;

const CF_MITIGATED_HEADER: &str = "cf-mitigated";
const CF_CHALLENGE_VALUE: &str = "challenge";

/// Jour calendrier ISO valide avant toute construction d'URL.
///
/// Le type empeche une entree libre de modifier la query ou le chemin des endpoints fermes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct CalendarLocalDay(Date);

impl CalendarLocalDay {
    /// Parse strictement une date `YYYY-MM-DD`, sans accepter un prefixe, une heure ou une query.
    pub(crate) fn parse(value: &str) -> Result<Self, CalendarLocalDayError> {
        let bytes = value.as_bytes();
        if bytes.len() != 10
            || bytes[4] != b'-'
            || bytes[7] != b'-'
            || !bytes
                .iter()
                .enumerate()
                .filter(|(index, _)| *index != 4 && *index != 7)
                .all(|(_, byte)| byte.is_ascii_digit())
        {
            return Err(CalendarLocalDayError::InvalidFormat);
        }

        let year = value[0..4]
            .parse::<i32>()
            .map_err(|_| CalendarLocalDayError::InvalidFormat)?;
        let month = value[5..7]
            .parse::<u8>()
            .map_err(|_| CalendarLocalDayError::InvalidFormat)?;
        let day = value[8..10]
            .parse::<u8>()
            .map_err(|_| CalendarLocalDayError::InvalidFormat)?;
        let month = Month::try_from(month).map_err(|_| CalendarLocalDayError::InvalidDate)?;

        Date::from_calendar_date(year, month, day)
            .map(Self)
            .map_err(|_| CalendarLocalDayError::InvalidDate)
    }

    fn year(self) -> i32 {
        self.0.year()
    }

    fn month(self) -> Month {
        self.0.month()
    }

    fn day(self) -> u8 {
        self.0.day()
    }
}

impl fmt::Display for CalendarLocalDay {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{:04}-{:02}-{:02}",
            self.year(),
            u8::from(self.month()),
            self.day()
        )
    }
}

impl FromStr for CalendarLocalDay {
    type Err = CalendarLocalDayError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

/// Erreur redigee de validation du jour; la valeur rejetee ne quitte jamais l'appelant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CalendarLocalDayError {
    InvalidFormat,
    InvalidDate,
}

/// Provenance native fermee; aucune chaine configuree ne peut creer un nouveau collecteur.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CalendarCollectorSource {
    #[serde(untagged)]
    Feed(CalendarFeedSource),
}

/// Identifiant de provenance expose entre les etapes du calendrier natif.
///
/// Le nom de compatibilite `CalendarCollectorSource` reste local au module de
/// collecte, tandis que les contrats qui quittent cette etape utilisent cet
/// alias ferme. Aucune chaine configurable ne peut donc introduire une
/// source arbitraire entre collecte, audit et persistence.
pub(crate) type CalendarSourceId = CalendarCollectorSource;

impl CalendarCollectorSource {
    /// Ordre stable du set de production. Tous les flux sont examines.
    pub(crate) const PRODUCTION_ORDER: [Self; 8] = [
        Self::Feed(CalendarFeedSource::ForexFactoryExport),
        Self::Feed(CalendarFeedSource::BeaJson),
        Self::Feed(CalendarFeedSource::BeaIcs),
        Self::Feed(CalendarFeedSource::FedFomc),
        Self::Feed(CalendarFeedSource::FedMonetaryRss),
        Self::Feed(CalendarFeedSource::FedSpeechesRss),
        Self::Feed(CalendarFeedSource::EurostatIcs),
        Self::Feed(CalendarFeedSource::EcbCalendar),
    ];

    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Feed(source) => source.name(),
        }
    }

    pub(crate) fn from_name(name: &str) -> Option<Self> {
        CalendarFeedSource::from_name(name).map(Self::Feed)
    }

    pub(crate) fn source_kind(
        self,
    ) -> crate::features::calendar_day::contracts::CalendarSourceKind {
        let Self::Feed(source) = self;
        crate::features::calendar_day::contracts::CalendarSourceKind::Feed(source)
    }

    pub(crate) fn is_context(self) -> bool {
        matches!(self, Self::Feed(source) if source.is_context())
    }

    /// Construit l'URL de production sans accepter de base, chemin ni parametre libre.
    pub(crate) fn production_url_for_day(self, local_day: CalendarLocalDay) -> Url {
        CalendarCollector::production(self).url_for_day(local_day)
    }

    fn production_endpoint(self) -> Url {
        let Self::Feed(source) = self;
        let endpoint = source.endpoint();
        Url::parse(endpoint).expect("calendar production endpoints must be valid URLs")
    }
}

/// Classification terminale d'une tentative, independent du texte retourne par le fournisseur.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CalendarCollectorOutcome {
    /// La reponse est exploitable et contient une representation reconnue du jour cible.
    Covered,
    /// La source n'est pas couverte pour cette tentative, mais une autre source peut l'etre.
    Unavailable(CalendarCollectorUnavailableReason),
    /// L'acces est explicitement refuse; la boucle peut encore essayer l'autre provenance fermee.
    Refused(CalendarCollectorRefusalReason),
}

/// Sous-cause non sensible d'une indisponibilite.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CalendarCollectorUnavailableReason {
    Timeout,
    Transport,
    Redirect,
    HttpStatus,
    BodyTooLarge,
    InvalidEncoding,
    MissingTargetDay,
    InvalidFeed,
}

/// Sous-cause fermee d'un refus en provenance du site.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CalendarCollectorRefusalReason {
    Unauthorized,
    Forbidden,
    RateLimited,
    LegalRestriction,
    Challenge,
}

/// Refus terminal deja observe pour une source et un jour donnes.
///
/// Il est relu depuis SQLite avant un nouveau run afin que le collecteur ne
/// refasse jamais une requete HTTP vers une source qui a deja refuse l'acces
/// pour ce jour. L'audit est tout de meme reemission, mais sans corps ni appel
/// sortant, pour garder le nouveau run explicable dans l'admin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CalendarCollectorQuarantine {
    pub(crate) source: CalendarCollectorSource,
    pub(crate) reason: CalendarCollectorRefusalReason,
    pub(crate) http_status: Option<u16>,
}

/// Audit transmissible sans le contenu source, son URL ni un message de transport.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CalendarCollectorAudit {
    pub source: CalendarSourceId,
    pub outcome: CalendarCollectorOutcome,
    pub http_status: Option<u16>,
    /// SHA-256 du corps accepte pour lecture, en hexadécimal minuscule.
    pub raw_sha256: Option<String>,
    /// Longueur exacte du même corps en octets; elle ne dépasse jamais un mégaoctet.
    pub raw_length_bytes: Option<usize>,
}

/// Corps HTML en memoire uniquement. Il ne peut ni etre serialise ni affiche par `Debug`.
pub(crate) struct CalendarCoveredSource {
    source: CalendarSourceId,
    local_day: CalendarLocalDay,
    raw: String,
    document: Option<CalendarFeedDocument>,
}

impl CalendarCoveredSource {
    pub(crate) fn document(&self) -> Option<&CalendarFeedDocument> {
        self.document.as_ref()
    }
    pub(crate) fn source(&self) -> CalendarSourceId {
        self.source
    }

    pub(crate) fn local_day(&self) -> CalendarLocalDay {
        self.local_day
    }

    /// Donne le corps seulement a l'etape native suivante; aucun clone n'est produit ici.
    pub(crate) fn raw_text(&self) -> &str {
        &self.raw
    }
}

impl fmt::Debug for CalendarCoveredSource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CalendarCoveredSource")
            .field("source", &self.source)
            .field("local_day", &self.local_day)
            .finish_non_exhaustive()
    }
}

/// Sortie en memoire d'un passage sequentiel sur le set.
pub(crate) enum CalendarCollectorSetResult {
    Covered {
        /// Toutes les sources dont la couverture du jour a ete verifiee.
        ///
        /// Le set les garde dans son ordre d'execution afin que l'etape IA puisse
        /// fusionner les evenements des fournisseurs sans qu'une premiere
        /// reponse exploitable masque l'autre source autorisee.
        sources: Vec<CalendarCoveredSource>,
        audits: Vec<CalendarCollectorAudit>,
    },
    NoCoverage {
        audits: Vec<CalendarCollectorAudit>,
    },
}

impl CalendarCollectorSetResult {
    /// La couverture facultative ne vaut jamais admission produit.
    pub(crate) fn required_source_failure(
        &self,
    ) -> Option<super::diagnostics::CalendarDiagnosticReason> {
        use super::diagnostics::CalendarDiagnosticReason as R;
        let required = CalendarCollectorSource::Feed(CalendarFeedSource::ForexFactoryExport);
        let outcome = self
            .audits()
            .iter()
            .find(|a| a.source == required)
            .map(|a| a.outcome);
        match outcome {
            Some(CalendarCollectorOutcome::Covered) => {
                if matches!(self, Self::Covered { sources, .. } if sources.iter().any(|s| s.source() == required && s.document().is_some()))
                {
                    None
                } else {
                    Some(R::RequiredSourceInvalid)
                }
            }
            Some(CalendarCollectorOutcome::Refused(_)) => Some(R::RequiredSourceRefused),
            Some(CalendarCollectorOutcome::Unavailable(
                CalendarCollectorUnavailableReason::MissingTargetDay,
            )) => Some(R::RequiredSourceMissingDay),
            Some(CalendarCollectorOutcome::Unavailable(
                CalendarCollectorUnavailableReason::InvalidFeed
                | CalendarCollectorUnavailableReason::InvalidEncoding,
            )) => Some(R::RequiredSourceInvalid),
            _ => Some(R::RequiredSourceUnavailable),
        }
    }

    pub(crate) fn audits(&self) -> &[CalendarCollectorAudit] {
        match self {
            Self::Covered { audits, .. } | Self::NoCoverage { audits } => audits,
        }
    }
}

impl fmt::Debug for CalendarCollectorSetResult {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Covered { sources, audits } => formatter
                .debug_struct("CalendarCollectorSetResult::Covered")
                .field("sources", sources)
                .field("audits", audits)
                .finish(),
            Self::NoCoverage { audits } => formatter
                .debug_struct("CalendarCollectorSetResult::NoCoverage")
                .field("audits", audits)
                .finish(),
        }
    }
}

/// Ensemble de collecteurs execute strictement dans son ordre de construction.
pub(crate) struct CalendarCollectorSet {
    client: NativeCalendarHttpClient,
    collectors: Vec<CalendarCollector>,
}

impl CalendarCollectorSet {
    /// Set ferme des exports officiels, sans endpoint configurable.
    pub(crate) fn production() -> Result<Self, CalendarCollectorSetBuildError> {
        let collectors = CalendarCollectorSource::PRODUCTION_ORDER
            .into_iter()
            .map(CalendarCollector::production)
            .collect();
        Self::new(collectors)
    }

    /// Essaie tous les collecteurs sequentiellement et conserve chaque couverture verifiee.
    ///
    /// Un resultat `Covered` exige au moins une source couverte. Une erreur ou
    /// un refus de l'autre source ne retire jamais une couverture deja validee,
    /// mais son audit est conserve afin que le run reste explicable.
    pub(crate) async fn collect(&self, local_day: CalendarLocalDay) -> CalendarCollectorSetResult {
        self.collect_with_quarantines(local_day, &[]).await
    }

    /// Execute chaque source qui n'est pas deja en refus terminal pour le
    /// jour. Les sources en quarantaine ne sont pas appelees; leur audit
    /// `Refused` est reconstitue depuis l'etat durable pour ne pas masquer la
    /// cause lors d'une execution forcee ulterieure.
    pub(crate) async fn collect_with_quarantines(
        &self,
        local_day: CalendarLocalDay,
        quarantines: &[CalendarCollectorQuarantine],
    ) -> CalendarCollectorSetResult {
        let mut audits = Vec::with_capacity(self.collectors.len());
        let mut sources = Vec::with_capacity(self.collectors.len());

        for collector in &self.collectors {
            let operation_started = Instant::now();
            crate::features::calendar_logs::event(
                "source.started",
                serde_json::json!({"source_id": format!("{:?}", collector.source_id()), "local_day": local_day.to_string()}),
            );
            let attempt = if let Some(quarantine) = quarantines
                .iter()
                .find(|quarantine| quarantine.source == collector.source_id())
            {
                CalendarCollectorAttempt::without_raw(
                    collector.source_id(),
                    CalendarCollectorOutcome::Refused(quarantine.reason),
                    quarantine.http_status,
                )
            } else {
                crate::features::calendar_logs::context(serde_json::json!({"source_id": format!("{:?}", collector.source_id()), "call_id": uuid::Uuid::new_v4().to_string()}), self.client.collect(collector, local_day)).await
            };
            let rows_read = attempt
                .covered
                .as_ref()
                .and_then(CalendarCoveredSource::document)
                .map(|document| document.records.len());
            crate::features::calendar_logs::event(
                "source.finished",
                serde_json::json!({
                    "audit": &attempt.audit,
                    "rows_read": rows_read,
                    "duration_ms": operation_started.elapsed().as_millis(),
                }),
            );
            audits.push(attempt.audit);
            if let Some(source) = attempt.covered {
                sources.push(source);
            }
        }

        if !sources.iter().any(|source| !source.source().is_context()) {
            CalendarCollectorSetResult::NoCoverage { audits }
        } else {
            CalendarCollectorSetResult::Covered { sources, audits }
        }
    }

    fn new(collectors: Vec<CalendarCollector>) -> Result<Self, CalendarCollectorSetBuildError> {
        if collectors.is_empty() {
            return Err(CalendarCollectorSetBuildError::EmptyCollectorSet);
        }

        let mut sources = HashSet::with_capacity(collectors.len());
        if collectors
            .iter()
            .any(|collector| !sources.insert(collector.source_id()))
        {
            return Err(CalendarCollectorSetBuildError::DuplicateSource);
        }

        Ok(Self {
            client: NativeCalendarHttpClient::new()?,
            collectors,
        })
    }

    /// Injection reservee aux tests loopback; aucun endpoint distant configurable n'existe en production.
    #[cfg(test)]
    pub(crate) fn loopback_for_tests(
        endpoints: impl IntoIterator<Item = (CalendarCollectorSource, Url)>,
    ) -> Result<Self, CalendarCollectorSetBuildError> {
        let collectors = endpoints
            .into_iter()
            .map(|(source, endpoint)| CalendarCollector::loopback_for_tests(source, endpoint))
            .collect::<Result<Vec<_>, _>>()?;
        Self::new(collectors)
    }
}

/// Erreur de construction sans URL, secret ou message de la bibliotheque HTTP.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CalendarCollectorSetBuildError {
    EmptyCollectorSet,
    DuplicateSource,
    InvalidTestEndpoint,
    HttpClient,
}

/// Collecteur lie a une provenance typee et a une base deja validee.
struct CalendarCollector {
    source: CalendarCollectorSource,
    endpoint: Url,
}

/// Contrat ferme d'une provenance calendrier.
///
/// Le set n'accepte que des implementations internes de ce trait : chacune
/// fournit un identifiant type et fabrique son URL depuis un jour deja valide.
/// Aucun appelant ne peut ainsi injecter une URL, un header ou une strategie de
/// retry entre la collecte, l'audit et la persistence.
trait CalendarSourceCollector {
    fn source_id(&self) -> CalendarSourceId;
    fn url_for_day(&self, local_day: CalendarLocalDay) -> Url;
}

impl CalendarCollector {
    fn production(source: CalendarCollectorSource) -> Self {
        Self {
            source,
            endpoint: source.production_endpoint(),
        }
    }
}

impl CalendarSourceCollector for CalendarCollector {
    fn source_id(&self) -> CalendarSourceId {
        self.source
    }

    fn url_for_day(&self, local_day: CalendarLocalDay) -> Url {
        let _ = local_day;
        self.endpoint.clone()
    }
}

impl CalendarCollector {
    #[cfg(test)]
    fn loopback_for_tests(
        source: CalendarCollectorSource,
        endpoint: Url,
    ) -> Result<Self, CalendarCollectorSetBuildError> {
        if endpoint.scheme() != "http"
            || endpoint.username() != ""
            || endpoint.password().is_some()
            || endpoint.query().is_some()
            || endpoint.fragment().is_some()
            || !endpoint.host_str().is_some_and(is_loopback_test_host)
        {
            return Err(CalendarCollectorSetBuildError::InvalidTestEndpoint);
        }

        Ok(Self { source, endpoint })
    }
}

/// Client prive : aucun appelant ne peut lui ajouter cookies, proxy, auth, referer ou politique redirect.
struct NativeCalendarHttpClient {
    client: Client,
    // At most eight one-MB bodies, no disk. Weekly preparation reuses the same export.
    cache: tokio::sync::Mutex<HashMap<CalendarFeedSource, (Instant, u16, String)>>,
}

impl NativeCalendarHttpClient {
    fn new() -> Result<Self, CalendarCollectorSetBuildError> {
        let client = Client::builder()
            .timeout(NATIVE_CALENDAR_HTTP_TIMEOUT)
            .cookie_store(false)
            .no_proxy()
            .redirect(Policy::none())
            .referer(false)
            .default_headers(HeaderMap::new())
            .build()
            .map_err(|_| CalendarCollectorSetBuildError::HttpClient)?;
        Ok(Self {
            client,
            cache: tokio::sync::Mutex::new(HashMap::new()),
        })
    }

    async fn collect<C: CalendarSourceCollector>(
        &self,
        collector: &C,
        local_day: CalendarLocalDay,
    ) -> CalendarCollectorAttempt {
        let CalendarCollectorSource::Feed(source) = collector.source_id();
        let cached = {
            let cache = self.cache.lock().await;
            cache
                .get(&source)
                .filter(|(at, _, _)| at.elapsed() < Duration::from_secs(3600))
                .map(|(_, status, raw)| (*status, raw.clone()))
        };
        if let Some((status, raw)) = cached {
            crate::features::calendar_logs::event(
                "source.cache_hit",
                serde_json::json!({"status": status, "bytes": raw.len()}),
            );
            return self.classify_body(collector.source_id(), local_day, status, raw);
        }
        crate::features::calendar_logs::event(
            "source.request",
            serde_json::json!({"method":"GET", "url":collector.url_for_day(local_day).as_str(), "timeout_ms": NATIVE_CALENDAR_HTTP_TIMEOUT.as_millis()}),
        );
        let response = match self
            .client
            .get(collector.url_for_day(local_day))
            .send()
            .await
        {
            Ok(response) => response,
            Err(error) => {
                return CalendarCollectorAttempt::without_raw(
                    collector.source_id(),
                    CalendarCollectorOutcome::Unavailable(classify_reqwest_error(&error)),
                    None,
                );
            }
        };

        let status = response.status();
        let status_code = status.as_u16();
        crate::features::calendar_logs::event(
            "source.response",
            serde_json::json!({"status":status_code, "headers":format!("{:?}", response.headers())}),
        );
        let challenge = response_is_challenge(&response);
        if challenge || !status.is_success() {
            Self::log_non_success_body(response, status_code).await;
            let outcome = if challenge {
                CalendarCollectorOutcome::Refused(CalendarCollectorRefusalReason::Challenge)
            } else if let Some(reason) = refusal_for_status(status) {
                CalendarCollectorOutcome::Refused(reason)
            } else {
                CalendarCollectorOutcome::Unavailable(if status.is_redirection() {
                    CalendarCollectorUnavailableReason::Redirect
                } else {
                    CalendarCollectorUnavailableReason::HttpStatus
                })
            };
            return CalendarCollectorAttempt::without_raw(
                collector.source_id(),
                outcome,
                Some(status_code),
            );
        }

        let body = match read_body_limited(response).await {
            Ok(body) => body,
            Err(reason) => {
                crate::features::calendar_logs::event(
                    "source.response_body_status",
                    serde_json::json!({"status": status_code, "captured_bytes": 0, "incomplete": true, "reason": reason}),
                );
                return CalendarCollectorAttempt::without_raw(
                    collector.source_id(),
                    CalendarCollectorOutcome::Unavailable(reason),
                    Some(status_code),
                );
            }
        };
        crate::features::calendar_logs::payload("source.body", &String::from_utf8_lossy(&body));
        let raw_sha256 = sha256_hex(&body);
        let raw_length_bytes = body.len();
        let raw = match String::from_utf8(body) {
            Ok(raw) => raw,
            Err(_) => {
                return CalendarCollectorAttempt::with_raw_audit(
                    collector.source_id(),
                    CalendarCollectorOutcome::Unavailable(
                        CalendarCollectorUnavailableReason::InvalidEncoding,
                    ),
                    status_code,
                    raw_sha256,
                    raw_length_bytes,
                );
            }
        };

        if !body_is_challenge(&raw) {
            let CalendarCollectorSource::Feed(source) = collector.source_id();
            self.cache
                .lock()
                .await
                .insert(source, (Instant::now(), status_code, raw.clone()));
        }
        self.classify_body(collector.source_id(), local_day, status_code, raw)
    }

    async fn log_non_success_body(mut response: Response, status: u16) {
        let mut body = Vec::new();
        let mut incomplete = false;
        // Diagnostic reads must not turn a rejected source into a long-running workflow.
        let deadline = tokio::time::Instant::now() + Duration::from_secs(1);
        loop {
            match tokio::time::timeout_at(deadline, response.chunk()).await {
                Ok(Ok(Some(chunk))) => {
                    let remaining = NATIVE_CALENDAR_MAX_BODY_BYTES.saturating_sub(body.len());
                    body.extend_from_slice(&chunk[..chunk.len().min(remaining)]);
                    if chunk.len() > remaining {
                        incomplete = true;
                        break;
                    }
                }
                Ok(Ok(None)) => break,
                _ => {
                    incomplete = true;
                    break;
                }
            }
        }
        crate::features::calendar_logs::event(
            "source.response_body_status",
            serde_json::json!({"status": status, "captured_bytes": body.len(), "incomplete": incomplete}),
        );
        crate::features::calendar_logs::payload(
            if incomplete {
                "source.response_body_incomplete"
            } else {
                "source.response_body"
            },
            &String::from_utf8_lossy(&body),
        );
    }

    fn classify_body(
        &self,
        source: CalendarCollectorSource,
        local_day: CalendarLocalDay,
        status_code: u16,
        raw: String,
    ) -> CalendarCollectorAttempt {
        let raw_sha256 = sha256_hex(raw.as_bytes());
        let raw_length_bytes = raw.len();
        if body_is_challenge(&raw) {
            return CalendarCollectorAttempt::with_raw_audit(
                source,
                CalendarCollectorOutcome::Refused(CalendarCollectorRefusalReason::Challenge),
                status_code,
                raw_sha256,
                raw_length_bytes,
            );
        }
        let CalendarCollectorSource::Feed(feed) = source;
        let document = match feeds::parse(feed, &raw, local_day) {
            Ok(document) => Some(document),
            Err(_) => {
                return CalendarCollectorAttempt::with_raw_audit(
                    source,
                    CalendarCollectorOutcome::Unavailable(
                        CalendarCollectorUnavailableReason::InvalidFeed,
                    ),
                    status_code,
                    raw_sha256,
                    raw_length_bytes,
                );
            }
        };
        let covered = document
            .as_ref()
            .map(|d| !d.records.is_empty())
            .unwrap_or_else(|| contains_target_day(&raw, local_day));
        if !covered {
            return CalendarCollectorAttempt::with_raw_audit(
                source,
                CalendarCollectorOutcome::Unavailable(
                    CalendarCollectorUnavailableReason::MissingTargetDay,
                ),
                status_code,
                raw_sha256,
                raw_length_bytes,
            );
        }

        let raw = if let Some(document) = &document {
            serde_json::to_string(document).expect("typed feed document serializes")
        } else {
            raw
        };

        crate::features::calendar_logs::event(
            "source.parsed",
            serde_json::json!({
                "normalized_bytes": raw.len(),
                "rows_read": document.as_ref().map(|d| d.records.len()),
            }),
        );
        CalendarCollectorAttempt {
            audit: CalendarCollectorAudit {
                source,
                outcome: CalendarCollectorOutcome::Covered,
                http_status: Some(status_code),
                raw_sha256: Some(raw_sha256),
                raw_length_bytes: Some(raw_length_bytes),
            },
            covered: Some(CalendarCoveredSource {
                source,
                local_day,
                raw,
                document,
            }),
        }
    }
}

/// Une tentative garde le corps seulement lorsque sa couverture a ete verifiee.
struct CalendarCollectorAttempt {
    audit: CalendarCollectorAudit,
    covered: Option<CalendarCoveredSource>,
}

impl CalendarCollectorAttempt {
    fn without_raw(
        source: CalendarCollectorSource,
        outcome: CalendarCollectorOutcome,
        http_status: Option<u16>,
    ) -> Self {
        Self {
            audit: CalendarCollectorAudit {
                source,
                outcome,
                http_status,
                raw_sha256: None,
                raw_length_bytes: None,
            },
            covered: None,
        }
    }

    fn with_raw_audit(
        source: CalendarCollectorSource,
        outcome: CalendarCollectorOutcome,
        http_status: u16,
        raw_sha256: String,
        raw_length_bytes: usize,
    ) -> Self {
        Self {
            audit: CalendarCollectorAudit {
                source,
                outcome,
                http_status: Some(http_status),
                raw_sha256: Some(raw_sha256),
                raw_length_bytes: Some(raw_length_bytes),
            },
            covered: None,
        }
    }
}

async fn read_body_limited(
    mut response: Response,
) -> Result<Vec<u8>, CalendarCollectorUnavailableReason> {
    if response
        .content_length()
        .is_some_and(|length| length > NATIVE_CALENDAR_MAX_BODY_BYTES as u64)
    {
        return Err(CalendarCollectorUnavailableReason::BodyTooLarge);
    }

    let mut body = Vec::with_capacity(
        response
            .content_length()
            .unwrap_or_default()
            .min(NATIVE_CALENDAR_MAX_BODY_BYTES as u64) as usize,
    );
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| classify_reqwest_error(&error))?
    {
        if chunk.len() > NATIVE_CALENDAR_MAX_BODY_BYTES.saturating_sub(body.len()) {
            crate::features::calendar_logs::payload(
                "source.body_incomplete",
                &String::from_utf8_lossy(&body),
            );
            return Err(CalendarCollectorUnavailableReason::BodyTooLarge);
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

fn classify_reqwest_error(error: &reqwest::Error) -> CalendarCollectorUnavailableReason {
    if error.is_timeout() {
        CalendarCollectorUnavailableReason::Timeout
    } else {
        CalendarCollectorUnavailableReason::Transport
    }
}

fn refusal_for_status(status: StatusCode) -> Option<CalendarCollectorRefusalReason> {
    match status {
        StatusCode::UNAUTHORIZED => Some(CalendarCollectorRefusalReason::Unauthorized),
        StatusCode::FORBIDDEN => Some(CalendarCollectorRefusalReason::Forbidden),
        StatusCode::TOO_MANY_REQUESTS => Some(CalendarCollectorRefusalReason::RateLimited),
        StatusCode::UNAVAILABLE_FOR_LEGAL_REASONS => {
            Some(CalendarCollectorRefusalReason::LegalRestriction)
        }
        _ => None,
    }
}

fn response_is_challenge(response: &Response) -> bool {
    response
        .headers()
        .get(CF_MITIGATED_HEADER)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.eq_ignore_ascii_case(CF_CHALLENGE_VALUE))
}

fn body_is_challenge(raw: &str) -> bool {
    let normalized = raw.to_ascii_lowercase();
    normalized.contains("challenge-form")
        || normalized.contains("verify you are human")
        || normalized.contains("<title>just a moment")
        || normalized.contains("<title>access denied")
        || normalized.contains("g-recaptcha")
        || normalized.contains("h-captcha")
        || normalized.contains("cf-chl-")
}

fn contains_target_day(raw: &str, local_day: CalendarLocalDay) -> bool {
    let normalized = normalize_language_text(&strip_markup_for_coverage(raw));
    target_day_needles(local_day)
        .iter()
        .any(|needle| normalized.contains(needle))
}

fn target_day_needles(local_day: CalendarLocalDay) -> Vec<String> {
    let year = local_day.year();
    let month = u8::from(local_day.month());
    let day = local_day.day();
    let mut needles = vec![
        local_day.to_string(),
        format!("{month:02}/{day:02}/{year}"),
        format!("{month}/{day}/{year}"),
        format!("{day:02}/{month:02}/{year}"),
        format!("{day}/{month}/{year}"),
        format!("{year} {month:02} {day:02}"),
        format!(
            "{}{}.{}",
            calendar_month_abbreviation(local_day.month()),
            day,
            year
        ),
    ];
    for month_name in month_names(local_day.month()) {
        needles.push(format!("{month_name} {day}"));
        needles.push(format!("{month_name} {day} {year}"));
        needles.push(format!("{day} {month_name}"));
        needles.push(format!("{day} {month_name} {year}"));
    }
    needles.sort();
    needles.dedup();
    needles
        .into_iter()
        .map(|needle| normalize_language_text(&needle))
        .collect()
}

fn month_names(month: Month) -> &'static [&'static str] {
    match month {
        Month::January => &["jan", "january", "janvier"],
        Month::February => &["feb", "february", "fevrier"],
        Month::March => &["mar", "march", "mars"],
        Month::April => &["apr", "april", "avril"],
        Month::May => &["may", "mai"],
        Month::June => &["jun", "june", "juin"],
        Month::July => &["jul", "july", "juillet"],
        Month::August => &["aug", "august", "aout"],
        Month::September => &["sep", "sept", "september", "septembre"],
        Month::October => &["oct", "october", "octobre"],
        Month::November => &["nov", "november", "novembre"],
        Month::December => &["dec", "december", "decembre"],
    }
}

fn calendar_month_abbreviation(month: Month) -> &'static str {
    match month {
        Month::January => "jan",
        Month::February => "feb",
        Month::March => "mar",
        Month::April => "apr",
        Month::May => "may",
        Month::June => "jun",
        Month::July => "jul",
        Month::August => "aug",
        Month::September => "sep",
        Month::October => "oct",
        Month::November => "nov",
        Month::December => "dec",
    }
}

/// Retire les scripts/styles et les balises avant de chercher une date visible.
fn strip_markup_for_coverage(value: &str) -> String {
    let without_script = strip_html_block(value, "script");
    let without_style = strip_html_block(&without_script, "style");
    let mut plain = String::with_capacity(without_style.len());
    let mut inside_tag = false;
    for character in without_style.chars() {
        match character {
            '<' => {
                inside_tag = true;
                plain.push(' ');
            }
            '>' => inside_tag = false,
            _ if !inside_tag => plain.push(character),
            _ => {}
        }
    }
    plain
}

fn strip_html_block(value: &str, tag: &str) -> String {
    let lower = value.to_ascii_lowercase();
    let open = format!("<{tag}");
    let close = format!("</{tag}>");
    let mut output = String::with_capacity(value.len());
    let mut cursor = 0;
    while let Some(relative_start) = lower[cursor..].find(&open) {
        let start = cursor + relative_start;
        output.push_str(&value[cursor..start]);
        let Some(relative_end) = lower[start..].find(&close) else {
            cursor = value.len();
            break;
        };
        cursor = start + relative_end + close.len();
        output.push(' ');
    }
    output.push_str(&value[cursor..]);
    output
}

fn normalize_language_text(value: &str) -> String {
    let mut normalized = String::with_capacity(value.len());
    let mut previous_was_space = false;
    for character in value.chars() {
        if character.is_whitespace() {
            if !previous_was_space {
                normalized.push(' ');
                previous_was_space = true;
            }
            continue;
        }
        previous_was_space = false;
        let character = match character {
            'à' | 'á' | 'â' | 'ä' | 'ã' | 'å' | 'À' | 'Á' | 'Â' | 'Ä' | 'Ã' | 'Å' => {
                'a'
            }
            'ç' | 'Ç' => 'c',
            'è' | 'é' | 'ê' | 'ë' | 'È' | 'É' | 'Ê' | 'Ë' => 'e',
            'ì' | 'í' | 'î' | 'ï' | 'Ì' | 'Í' | 'Î' | 'Ï' => 'i',
            'ñ' | 'Ñ' => 'n',
            'ò' | 'ó' | 'ô' | 'ö' | 'õ' | 'Ò' | 'Ó' | 'Ô' | 'Ö' | 'Õ' => 'o',
            'ù' | 'ú' | 'û' | 'ü' | 'Ù' | 'Ú' | 'Û' | 'Ü' => 'u',
            'ý' | 'ÿ' | 'Ý' | 'Ÿ' => 'y',
            other => other,
        };
        normalized.extend(character.to_lowercase());
    }
    normalized
}

fn sha256_hex(value: &[u8]) -> String {
    let digest = Sha256::digest(value);
    let mut output = String::with_capacity(digest.len() * 2);
    for byte in digest {
        use std::fmt::Write;
        let _ = write!(output, "{byte:02x}");
    }
    output
}

#[cfg(test)]
fn is_loopback_test_host(host: &str) -> bool {
    use std::net::IpAddr;

    host.eq_ignore_ascii_case("localhost")
        || host
            .parse::<IpAddr>()
            .is_ok_and(|address| address.is_loopback())
}

#[cfg(test)]
mod tests {
    use std::{
        collections::VecDeque,
        sync::{
            Arc, Mutex,
            atomic::{AtomicUsize, Ordering},
        },
    };

    use axum::{
        Router,
        body::Body,
        extract::{Request, State},
        http::{StatusCode, header},
        response::Response,
        routing::get,
    };

    use super::*;

    struct MockResponse {
        status: StatusCode,
        body: String,
        headers: Vec<(String, String)>,
    }

    impl MockResponse {
        fn new(status: StatusCode, body: impl Into<String>) -> Self {
            Self {
                status,
                body: body.into(),
                headers: Vec::new(),
            }
        }

        fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
            self.headers.push((name.into(), value.into()));
            self
        }
    }

    #[derive(Debug)]
    struct RecordedRequest {
        path_and_query: String,
        has_authorization: bool,
        has_cookie: bool,
        has_referer: bool,
    }

    struct MockState {
        responses: Mutex<VecDeque<MockResponse>>,
        requests: Mutex<Vec<RecordedRequest>>,
    }

    async fn mock_handler(State(state): State<Arc<MockState>>, request: Request) -> Response {
        let record = RecordedRequest {
            path_and_query: request
                .uri()
                .path_and_query()
                .map(|value| value.as_str().to_owned())
                .unwrap_or_else(|| request.uri().path().to_owned()),
            has_authorization: request.headers().contains_key(header::AUTHORIZATION),
            has_cookie: request.headers().contains_key(header::COOKIE),
            has_referer: request.headers().contains_key(header::REFERER),
        };
        state
            .requests
            .lock()
            .expect("mock request mutex poisoned")
            .push(record);
        let response = state
            .responses
            .lock()
            .expect("mock response mutex poisoned")
            .pop_front()
            .unwrap_or_else(|| {
                MockResponse::new(StatusCode::INTERNAL_SERVER_ERROR, "missing mock response")
            });
        let mut builder = Response::builder().status(response.status);
        for (name, value) in response.headers {
            builder = builder.header(name, value);
        }
        builder
            .body(Body::from(response.body))
            .expect("mock response must be valid")
    }

    async fn spawn_mock_server(
        responses: Vec<MockResponse>,
    ) -> (Url, Arc<MockState>, tokio::task::JoinHandle<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("mock listener must bind loopback");
        let address = listener
            .local_addr()
            .expect("mock listener must report its address");
        let state = Arc::new(MockState {
            responses: Mutex::new(responses.into()),
            requests: Mutex::new(Vec::new()),
        });
        let router = Router::new()
            .route("/bea", get(mock_handler))
            .route("/ff_export", get(mock_handler))
            .route("/redirect", get(mock_handler))
            .with_state(state.clone());
        let task = tokio::spawn(async move {
            let _ = axum::serve(listener, router).await;
        });
        (
            Url::parse(&format!("http://{address}/")).expect("mock base URL must be valid"),
            state,
            task,
        )
    }

    async fn spawn_redirect_target() -> (Url, Arc<AtomicUsize>, tokio::task::JoinHandle<()>) {
        async fn target_handler(State(calls): State<Arc<AtomicUsize>>) -> Response {
            calls.fetch_add(1, Ordering::Relaxed);
            Response::builder()
                .status(StatusCode::OK)
                .body(Body::from("30 août 2026"))
                .expect("redirect target response must be valid")
        }

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("redirect target must bind loopback");
        let address = listener
            .local_addr()
            .expect("redirect target must report its address");
        let calls = Arc::new(AtomicUsize::new(0));
        let router = Router::new()
            .route("/destination", get(target_handler))
            .with_state(calls.clone());
        let task = tokio::spawn(async move {
            let _ = axum::serve(listener, router).await;
        });
        (
            Url::parse(&format!("http://{address}/")).expect("target base URL must be valid"),
            calls,
            task,
        )
    }

    fn local_day() -> CalendarLocalDay {
        CalendarLocalDay::parse("2026-08-30").expect("test local day must be valid")
    }

    #[tokio::test]
    async fn required_export_gate_preserves_all_eight_audits_and_optional_coverage() {
        use super::super::diagnostics::CalendarDiagnosticReason as R;
        let valid =
            r#"[{"title":"Test","country":"USD","date":"2026-09-07T12:00:00Z","impact":"High"}]"#;
        for (status, body, expected) in [
            (StatusCode::OK, valid, None),
            (
                StatusCode::FORBIDDEN,
                "private",
                Some(R::RequiredSourceRefused),
            ),
            (
                StatusCode::SERVICE_UNAVAILABLE,
                "private",
                Some(R::RequiredSourceUnavailable),
            ),
            (StatusCode::OK, "broken", Some(R::RequiredSourceInvalid)),
            (
                StatusCode::OK,
                r#"[{"title":"Test","country":"USD","date":"2026-09-08T12:00:00Z","impact":"Low"}]"#,
                Some(R::RequiredSourceMissingDay),
            ),
        ] {
            let mut responses = vec![MockResponse::new(status, body)];
            responses.push(MockResponse::new(
                StatusCode::OK,
                r#"{"Extra":{"release_dates":["2026-09-07T14:00:00Z"]}}"#,
            ));
            responses.extend(
                (0..6).map(|_| MockResponse::new(StatusCode::SERVICE_UNAVAILABLE, "private")),
            );
            let (base, state, server) = spawn_mock_server(responses).await;
            // Mock routes are shared but responses are ordered exactly as production.
            let set = CalendarCollectorSet::loopback_for_tests(
                CalendarCollectorSource::PRODUCTION_ORDER
                    .into_iter()
                    .map(|s| (s, endpoint(&base, "/ff_export"))),
            )
            .unwrap();
            let result = set.collect("2026-09-07".parse().unwrap()).await;
            assert_eq!(result.required_source_failure(), expected);
            assert_eq!(result.audits().len(), 8);
            assert_eq!(state.requests.lock().unwrap().len(), 8);
            assert_eq!(
                result.audits()[1].outcome,
                CalendarCollectorOutcome::Covered
            );
            server.abort();
        }
        assert_eq!(
            CalendarCollectorSetResult::NoCoverage { audits: vec![] }.required_source_failure(),
            Some(R::RequiredSourceUnavailable)
        );
    }

    #[tokio::test]
    async fn required_export_alone_suffices_when_all_optional_sources_fail() {
        let mut responses = vec![MockResponse::new(
            StatusCode::OK,
            r#"[{"title":"Test","country":"USD","date":"2026-09-07T12:00:00Z","impact":"Low"}]"#,
        )];
        responses
            .extend((0..7).map(|_| MockResponse::new(StatusCode::SERVICE_UNAVAILABLE, "private")));
        let (base, state, server) = spawn_mock_server(responses).await;
        let set = CalendarCollectorSet::loopback_for_tests(
            CalendarCollectorSource::PRODUCTION_ORDER
                .into_iter()
                .map(|s| (s, endpoint(&base, "/ff_export"))),
        )
        .unwrap();
        assert_eq!(
            set.collect("2026-09-07".parse().unwrap())
                .await
                .required_source_failure(),
            None
        );
        assert_eq!(state.requests.lock().unwrap().len(), 8);
        server.abort();
    }

    #[test]
    fn active_sources_are_only_official_feeds_and_audit_ids_are_flat_strings() {
        assert_eq!(CalendarCollectorSource::PRODUCTION_ORDER.len(), 8);
        for source in CalendarCollectorSource::PRODUCTION_ORDER {
            assert!(matches!(source, CalendarCollectorSource::Feed(_)));
            let url = source.production_url_for_day(local_day());
            assert!(!url.as_str().contains("economic-calendar"));
            assert!(!url.as_str().contains("www.forexfactory.com"));
            assert_eq!(
                serde_json::to_string(&source).unwrap(),
                format!("\"{}\"", source.name())
            );
            assert_eq!(
                CalendarCollectorSource::from_name(source.name()),
                Some(source)
            );
        }
    }

    #[test]
    fn passive_js_detection_is_not_a_challenge_but_challenge_markers_remain_terminal() {
        assert!(!body_is_challenge(
            "<main>FOMC calendar</main><script src='/cdn-cgi/challenge-platform/scripts/jsd/main.js'></script>"
        ));
        for body in [
            "<title>Just a moment...</title>",
            "<form id='challenge-form'></form>",
            "<div>Verify you are human</div>",
            "<div id='cf-chl-widget'></div>",
        ] {
            assert!(body_is_challenge(body));
        }
    }

    #[tokio::test]
    async fn feeds_collect_all_and_reuse_bounded_memory_cache_across_days() {
        let body =
            r#"[{"title":"Test","country":"USD","date":"2026-09-07T12:00:00Z","impact":"High"}]"#;
        let (base, state, server) = spawn_mock_server(vec![
            MockResponse::new(StatusCode::OK, body),
            MockResponse::new(
                StatusCode::OK,
                r#"{"Test":{"release_dates":["2026-09-07T12:00:00Z"]}}"#,
            ),
        ])
        .await;
        let collectors = CalendarCollectorSet::loopback_for_tests(vec![
            (
                CalendarCollectorSource::Feed(CalendarFeedSource::ForexFactoryExport),
                endpoint(&base, "/ff_export"),
            ),
            (
                CalendarCollectorSource::Feed(CalendarFeedSource::BeaJson),
                endpoint(&base, "/bea"),
            ),
        ])
        .unwrap();
        let first = collectors.collect("2026-09-07".parse().unwrap()).await;
        assert!(
            matches!(first, CalendarCollectorSetResult::Covered { ref sources,.. } if sources.len()==2)
        );
        assert!(matches!(
            collectors.collect("2026-09-08".parse().unwrap()).await,
            CalendarCollectorSetResult::NoCoverage { .. }
        ));
        let requests = state.requests.lock().unwrap();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].path_and_query, "/ff_export");
        assert!(
            requests
                .iter()
                .all(|r| !r.has_authorization && !r.has_cookie && !r.has_referer)
        );
        server.abort();
    }

    #[tokio::test]
    async fn rss_alone_cannot_admit_a_calendar_and_quarantined_export_is_not_retried() {
        let rss = "<rss><channel><item><title>Fed</title><pubDate>Mon, 07 Sep 2026 12:00:00 GMT</pubDate></item></channel></rss>";
        let (base, state, server) = spawn_mock_server(vec![
            MockResponse::new(StatusCode::OK, rss),
            MockResponse::new(StatusCode::FORBIDDEN, "private"),
        ])
        .await;
        let collectors = CalendarCollectorSet::loopback_for_tests(vec![
            (
                CalendarCollectorSource::Feed(CalendarFeedSource::FedMonetaryRss),
                endpoint(&base, "/bea"),
            ),
            (
                CalendarCollectorSource::Feed(CalendarFeedSource::ForexFactoryExport),
                endpoint(&base, "/ff_export"),
            ),
        ])
        .unwrap();
        let day = "2026-09-07".parse().unwrap();
        let old = CalendarCollectorQuarantine {
            source: CalendarCollectorSource::Feed(CalendarFeedSource::ForexFactoryExport),
            reason: CalendarCollectorRefusalReason::Forbidden,
            http_status: Some(403),
        };
        let result = collectors.collect_with_quarantines(day, &[old]).await;
        assert!(matches!(
            result,
            CalendarCollectorSetResult::NoCoverage { .. }
        ));
        assert_eq!(state.requests.lock().unwrap().len(), 1);
        let new = CalendarCollectorQuarantine {
            source: CalendarCollectorSource::Feed(CalendarFeedSource::ForexFactoryExport),
            ..old
        };
        collectors.collect_with_quarantines(day, &[new]).await;
        assert_eq!(
            state.requests.lock().unwrap().len(),
            1,
            "quarantined export is never retried; RSS uses the memory cache"
        );
        server.abort();
    }

    #[tokio::test]
    #[ignore = "operator-only read-only official feed probe, network required; not a CI gate"]
    async fn official_calendar_feed_access_probe() {
        let set = CalendarCollectorSet::production().unwrap();
        let result = set.collect("2026-09-16".parse().unwrap()).await;
        for audit in result.audits() {
            eprintln!(
                "{} {:?} status={:?} bytes={:?}",
                audit.source.name(),
                audit.outcome,
                audit.http_status,
                audit.raw_length_bytes
            );
        }
        assert!(result.audits().iter().all(|audit| audit.outcome
            != CalendarCollectorOutcome::Unavailable(
                CalendarCollectorUnavailableReason::InvalidFeed
            )));
    }

    fn endpoint(base: &Url, path: &str) -> Url {
        base.join(path).expect("test endpoint must be valid")
    }

    fn forex_factory_export(day: &str) -> String {
        serde_json::json!([{
            "title": "Test release",
            "country": "USD",
            "date": format!("{day}T12:00:00Z"),
            "impact": "High"
        }])
        .to_string()
    }

    fn bea_json(day: &str) -> String {
        serde_json::json!({
            "BEA test release": { "release_dates": [format!("{day}T12:00:00Z")] }
        })
        .to_string()
    }

    #[test]
    fn typed_day_rejects_any_url_shaped_input() {
        assert_eq!(
            CalendarLocalDay::parse("2026-08-30?to=2099-01-01"),
            Err(CalendarLocalDayError::InvalidFormat)
        );
        assert_eq!(
            CalendarLocalDay::parse("2026-02-30"),
            Err(CalendarLocalDayError::InvalidDate)
        );
    }

    #[test]
    fn production_urls_are_closed_and_match_the_authorized_feeds() {
        let day = local_day();
        assert_eq!(
            CalendarCollectorSource::Feed(CalendarFeedSource::BeaJson)
                .production_url_for_day(day)
                .as_str(),
            "https://apps.bea.gov/API/signup/release_dates.json"
        );
        assert_eq!(
            CalendarCollectorSource::Feed(CalendarFeedSource::ForexFactoryExport)
                .production_url_for_day(day)
                .as_str(),
            "https://nfs.faireconomy.media/ff_calendar_thisweek.json"
        );
    }

    #[test]
    fn refusal_statuses_are_classified_without_retaining_a_body() {
        assert_eq!(
            refusal_for_status(StatusCode::UNAUTHORIZED),
            Some(CalendarCollectorRefusalReason::Unauthorized)
        );
        assert_eq!(
            refusal_for_status(StatusCode::FORBIDDEN),
            Some(CalendarCollectorRefusalReason::Forbidden)
        );
        assert_eq!(
            refusal_for_status(StatusCode::TOO_MANY_REQUESTS),
            Some(CalendarCollectorRefusalReason::RateLimited)
        );
        assert_eq!(
            refusal_for_status(StatusCode::UNAVAILABLE_FOR_LEGAL_REASONS),
            Some(CalendarCollectorRefusalReason::LegalRestriction)
        );
        assert_eq!(refusal_for_status(StatusCode::OK), None);
    }

    #[test]
    fn source_coverage_recognizes_visible_french_and_abbreviated_dates() {
        let day = local_day();
        assert!(contains_target_day("<p>Dimanche 30 août 2026</p>", day));
        assert!(contains_target_day("<p>Calendar: aug30.2026</p>", day));
        assert!(!contains_target_day(
            "<script>2026-08-30</script><p>31 août 2026</p>",
            day
        ));
    }

    #[tokio::test]
    async fn loopback_request_has_no_cookie_auth_or_referer_and_collects_all_covered_sources() {
        let (base, state, server) = spawn_mock_server(vec![
            MockResponse::new(StatusCode::OK, bea_json("2026-08-30")),
            MockResponse::new(StatusCode::OK, forex_factory_export("2026-08-30")),
        ])
        .await;
        let set = CalendarCollectorSet::loopback_for_tests(vec![
            (
                CalendarCollectorSource::Feed(CalendarFeedSource::BeaJson),
                endpoint(&base, "bea"),
            ),
            (
                CalendarCollectorSource::Feed(CalendarFeedSource::ForexFactoryExport),
                endpoint(&base, "ff_export"),
            ),
        ])
        .expect("loopback set must be valid");

        let result = set.collect(local_day()).await;
        let sources = match result {
            CalendarCollectorSetResult::Covered { sources, audits } => {
                assert_eq!(audits.len(), 2);
                assert!(
                    audits
                        .iter()
                        .all(|audit| audit.outcome == CalendarCollectorOutcome::Covered)
                );
                sources
            }
            CalendarCollectorSetResult::NoCoverage { .. } => {
                panic!("both sources must cover the day")
            }
        };
        assert_eq!(sources.len(), 2);
        assert_eq!(
            sources[0].source(),
            CalendarCollectorSource::Feed(CalendarFeedSource::BeaJson)
        );
        assert_eq!(sources[0].local_day(), local_day());
        assert!(sources[0].raw_text().contains("BEA test release"));
        assert_eq!(
            sources[1].source(),
            CalendarCollectorSource::Feed(CalendarFeedSource::ForexFactoryExport)
        );
        assert!(sources[1].raw_text().contains("Test release"));

        let requests = state.requests.lock().expect("mock request mutex poisoned");
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].path_and_query, "/bea");
        assert_eq!(requests[1].path_and_query, "/ff_export");
        assert!(!requests[0].has_authorization);
        assert!(!requests[0].has_cookie);
        assert!(!requests[0].has_referer);
        server.abort();
    }

    #[tokio::test]
    async fn set_continues_sequentially_after_missing_day_and_keeps_raw_out_of_audit() {
        let secret = "CALENDAR_SOURCE_SECRET_9f34";
        let (base, state, server) = spawn_mock_server(vec![
            MockResponse::new(StatusCode::OK, bea_json("2026-08-31")),
            MockResponse::new(
                StatusCode::OK,
                forex_factory_export("2026-08-30").replace("Test release", secret),
            ),
        ])
        .await;
        let set = CalendarCollectorSet::loopback_for_tests(vec![
            (
                CalendarCollectorSource::Feed(CalendarFeedSource::BeaJson),
                endpoint(&base, "bea"),
            ),
            (
                CalendarCollectorSource::Feed(CalendarFeedSource::ForexFactoryExport),
                endpoint(&base, "ff_export"),
            ),
        ])
        .expect("loopback set must be valid");

        let result = set.collect(local_day()).await;
        let debug = format!("{result:?}");
        assert!(!debug.contains(secret));
        let audits_json = serde_json::to_string(result.audits()).expect("audit must serialize");
        assert!(!audits_json.contains(secret));
        assert_eq!(result.audits().len(), 2);
        assert_eq!(
            result.audits()[0].outcome,
            CalendarCollectorOutcome::Unavailable(
                CalendarCollectorUnavailableReason::MissingTargetDay
            )
        );
        assert_eq!(
            result.audits()[1].outcome,
            CalendarCollectorOutcome::Covered
        );
        assert!(result.audits()[0].raw_sha256.is_some());
        assert_eq!(
            result.audits()[0].raw_length_bytes,
            Some(bea_json("2026-08-31").len())
        );

        let sources = match result {
            CalendarCollectorSetResult::Covered { sources, .. } => sources,
            CalendarCollectorSetResult::NoCoverage { .. } => {
                panic!("second source must cover the day")
            }
        };
        assert_eq!(sources.len(), 1);
        assert_eq!(
            sources[0].source(),
            CalendarCollectorSource::Feed(CalendarFeedSource::ForexFactoryExport)
        );
        assert!(sources[0].raw_text().contains(secret));

        let requests = state.requests.lock().expect("mock request mutex poisoned");
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[1].path_and_query, "/ff_export");
        server.abort();
    }

    #[tokio::test]
    async fn challenge_body_is_refused_even_with_a_success_status() {
        let (base, _state, server) = spawn_mock_server(vec![MockResponse::new(
            StatusCode::OK,
            "<title>Just a moment...</title><div id=\"cf-chl-widget\">Cloudflare challenge</div>",
        )])
        .await;
        let set = CalendarCollectorSet::loopback_for_tests(vec![(
            CalendarCollectorSource::Feed(CalendarFeedSource::BeaJson),
            endpoint(&base, "bea"),
        )])
        .expect("loopback set must be valid");

        let result = set.collect(local_day()).await;
        assert_eq!(
            result.audits()[0].outcome,
            CalendarCollectorOutcome::Refused(CalendarCollectorRefusalReason::Challenge)
        );
        assert!(matches!(
            result,
            CalendarCollectorSetResult::NoCoverage { .. }
        ));
        server.abort();
    }

    #[tokio::test]
    async fn covered_source_wins_even_when_the_other_source_is_refused() {
        let (base, state, server) = spawn_mock_server(vec![
            MockResponse::new(StatusCode::OK, bea_json("2026-08-30")),
            MockResponse::new(StatusCode::FORBIDDEN, "do not retain this body"),
        ])
        .await;
        let set = CalendarCollectorSet::loopback_for_tests(vec![
            (
                CalendarCollectorSource::Feed(CalendarFeedSource::BeaJson),
                endpoint(&base, "bea"),
            ),
            (
                CalendarCollectorSource::Feed(CalendarFeedSource::ForexFactoryExport),
                endpoint(&base, "ff_export"),
            ),
        ])
        .expect("loopback set must be valid");

        let result = set.collect(local_day()).await;
        assert!(matches!(
            &result,
            CalendarCollectorSetResult::Covered { sources, audits }
                if sources.len() == 1
                    && sources[0].source() == CalendarCollectorSource::Feed(CalendarFeedSource::BeaJson)
                    && audits.len() == 2
                    && audits[1].outcome
                        == CalendarCollectorOutcome::Refused(
                            CalendarCollectorRefusalReason::Forbidden
                        )
        ));
        assert_eq!(
            state
                .requests
                .lock()
                .expect("mock request mutex poisoned")
                .len(),
            2
        );
        server.abort();
    }

    #[tokio::test]
    async fn quarantined_refused_source_is_never_requested_again_for_the_same_day() {
        let (base, state, server) = spawn_mock_server(vec![
            MockResponse::new(StatusCode::OK, bea_json("2026-08-30")),
            MockResponse::new(StatusCode::FORBIDDEN, "do not retain this body"),
            MockResponse::new(StatusCode::OK, bea_json("2026-08-30")),
        ])
        .await;
        let set = CalendarCollectorSet::loopback_for_tests(vec![
            (
                CalendarCollectorSource::Feed(CalendarFeedSource::BeaJson),
                endpoint(&base, "bea"),
            ),
            (
                CalendarCollectorSource::Feed(CalendarFeedSource::ForexFactoryExport),
                endpoint(&base, "ff_export"),
            ),
        ])
        .expect("loopback set must be valid");

        let first = set.collect(local_day()).await;
        assert!(matches!(
            first,
            CalendarCollectorSetResult::Covered { ref audits, .. }
                if audits[1].outcome
                    == CalendarCollectorOutcome::Refused(
                        CalendarCollectorRefusalReason::Forbidden
                    )
        ));
        let second = set
            .collect_with_quarantines(
                local_day(),
                &[CalendarCollectorQuarantine {
                    source: CalendarCollectorSource::Feed(CalendarFeedSource::ForexFactoryExport),
                    reason: CalendarCollectorRefusalReason::Forbidden,
                    http_status: Some(403),
                }],
            )
            .await;
        assert!(matches!(
            second,
            CalendarCollectorSetResult::Covered { ref sources, ref audits }
                if sources.len() == 1
                    && sources[0].source() == CalendarCollectorSource::Feed(CalendarFeedSource::BeaJson)
                    && audits[1].outcome
                        == CalendarCollectorOutcome::Refused(
                            CalendarCollectorRefusalReason::Forbidden
                        )
                    && audits[1].http_status == Some(403)
        ));

        let requests = state.requests.lock().expect("mock request mutex poisoned");
        assert_eq!(
            requests.len(),
            2,
            "the forced rerun reuses the covered BEA document and never retries ForexFactory"
        );
        server.abort();
    }

    #[tokio::test]
    async fn no_covered_source_returns_no_coverage_after_all_audits() {
        let (base, state, server) = spawn_mock_server(vec![
            MockResponse::new(StatusCode::OK, bea_json("2026-08-31")),
            MockResponse::new(StatusCode::TOO_MANY_REQUESTS, "do not retain this body"),
        ])
        .await;
        let set = CalendarCollectorSet::loopback_for_tests(vec![
            (
                CalendarCollectorSource::Feed(CalendarFeedSource::BeaJson),
                endpoint(&base, "bea"),
            ),
            (
                CalendarCollectorSource::Feed(CalendarFeedSource::ForexFactoryExport),
                endpoint(&base, "ff_export"),
            ),
        ])
        .expect("loopback set must be valid");

        let result = set.collect(local_day()).await;
        assert!(matches!(
            result,
            CalendarCollectorSetResult::NoCoverage { audits }
                if audits.len() == 2
                    && audits[0].outcome
                        == CalendarCollectorOutcome::Unavailable(
                            CalendarCollectorUnavailableReason::MissingTargetDay
                        )
                    && audits[1].outcome
                        == CalendarCollectorOutcome::Refused(
                            CalendarCollectorRefusalReason::RateLimited
                        )
        ));
        assert_eq!(
            state
                .requests
                .lock()
                .expect("mock request mutex poisoned")
                .len(),
            2
        );
        server.abort();
    }

    #[tokio::test]
    async fn redirect_is_not_followed() {
        let (target_base, target_calls, target_server) = spawn_redirect_target().await;
        let location = endpoint(&target_base, "destination").to_string();
        let (base, _state, server) = spawn_mock_server(vec![
            MockResponse::new(StatusCode::FOUND, "").header("location", location),
        ])
        .await;
        let set = CalendarCollectorSet::loopback_for_tests(vec![(
            CalendarCollectorSource::Feed(CalendarFeedSource::BeaJson),
            endpoint(&base, "redirect"),
        )])
        .expect("loopback set must be valid");

        let result = set.collect(local_day()).await;
        assert_eq!(
            result.audits()[0].outcome,
            CalendarCollectorOutcome::Unavailable(CalendarCollectorUnavailableReason::Redirect)
        );
        assert_eq!(target_calls.load(Ordering::Relaxed), 0);
        server.abort();
        target_server.abort();
    }

    #[tokio::test]
    async fn advertised_body_over_one_megabyte_is_rejected_before_raw_storage() {
        let (base, _state, server) = spawn_mock_server(vec![
            // Hyper construit ainsi un Content-Length coherent avec le corps :
            // le client peut tester son rejet precoce sans que le faux serveur
            // viole son propre contrat HTTP.
            MockResponse::new(
                StatusCode::OK,
                "x".repeat(NATIVE_CALENDAR_MAX_BODY_BYTES + 1),
            ),
        ])
        .await;
        let set = CalendarCollectorSet::loopback_for_tests(vec![(
            CalendarCollectorSource::Feed(CalendarFeedSource::BeaJson),
            endpoint(&base, "bea"),
        )])
        .expect("loopback set must be valid");

        let result = set.collect(local_day()).await;
        assert_eq!(
            result.audits()[0].outcome,
            CalendarCollectorOutcome::Unavailable(CalendarCollectorUnavailableReason::BodyTooLarge)
        );
        assert_eq!(result.audits()[0].raw_sha256, None);
        assert_eq!(result.audits()[0].raw_length_bytes, None);
        server.abort();
    }

    #[test]
    fn fixed_http_limits_and_loopback_only_configuration_are_enforced() {
        assert_eq!(NATIVE_CALENDAR_HTTP_TIMEOUT, Duration::from_secs(20));
        assert_eq!(NATIVE_CALENDAR_MAX_BODY_BYTES, 1_000_000);
        assert!(matches!(
            CalendarCollectorSet::loopback_for_tests(vec![(
                CalendarCollectorSource::Feed(CalendarFeedSource::BeaJson),
                Url::parse("https://example.test/calendar").expect("valid test URL"),
            )]),
            Err(CalendarCollectorSetBuildError::InvalidTestEndpoint)
        ));
    }
}
