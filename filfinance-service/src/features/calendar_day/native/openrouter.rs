//! Full payload diagnostics are emitted only through calendar_logs when scoped by a calendar operation.
//! Client OpenRouter borne partage par les chemins calendrier Rust.
//!
//! Ce module ne connait ni SQLite, ni Piper/HLS, ni les alertes. Il transforme seulement un
//! prompt type en completion JSON, avec une admission immediate et un budget de retry ferme.
//! Aucun type exposable par `Debug` ne contient la cle, le prompt ou la reponse du modele.

mod error_log;

// `fmt` sert aux implementations manuelles qui redigent les donnees sensibles.
use std::fmt;
// `IpAddr` permet d'autoriser HTTP uniquement pour un serveur de test local.
use std::net::IpAddr;
// `Arc` partage le semaphore, la cle et le sleeper entre les clones legers du client.
use std::sync::Arc;
// `Duration` donne une unite explicite aux timeouts et aux backoffs.
use std::time::Duration;

// `async_trait` rend le sleeper injectable sans imposer une attente reelle aux tests.
use async_trait::async_trait;
// Les en-tetes types evitent les noms de headers ecrits a la main dans le flux HTTP.
use reqwest::header::{HeaderMap, RETRY_AFTER};
// Le client est injecte et reutilise; le statut reste type pour classifier les retries.
use reqwest::{Client, StatusCode};
// Les visiteurs Serde reduisent les marqueurs d'erreur distants sans garder leur corps ou message.
use serde::de::{IgnoredAny, MapAccess, SeqAccess, Visitor};
// Serde produit et lit les contrats OpenRouter sans `Value` entre les etapes HTTP.
use serde::{Deserialize, Deserializer, Serialize};
// Le semaphore isole la concurrence calendrier; `TryAcquireError` preserve l'admission non bloquante.
use tokio::sync::{Semaphore, TryAcquireError};
// Le sleeper de production repose sur l'horloge asynchrone Tokio, sans bloquer un thread runtime.
use tokio::time::sleep;
// `Url` valide l'endpoint configurable avant le premier appel reseau.
use url::Url;

// Le modele reste court afin d'eviter qu'une valeur de configuration arbitraire gonfle les requetes.
const MAX_MODEL_BYTES: usize = 160;
// Une cle est bornee avant d'etre conservee en memoire, sans jamais etre incluse dans une erreur.
const MAX_API_KEY_BYTES: usize = 4_096;
// Le shadow refuse un timeout nul ou trop long pour ne pas monopoliser ses ressources.
const MAX_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
// Le prompt couvre largement deux snapshots bornes tout en fermant les allocations accidentelles.
const MAX_PROMPT_BYTES_HARD_LIMIT: usize = 512 * 1_024;
// Le plafond HTTP egale exactement le megaoctet decimal accepte ensuite par le canonicaliseur.
const MAX_RESPONSE_BYTES_HARD_LIMIT: usize = 1_000_000;
// Trois retries au plus donnent quatre tentatives et empechent une panne fournisseur de s'amplifier.
const MAX_RETRIES_HARD_LIMIT: usize = 3;
// La concurrence propre au calendrier reste petite et independante des workers live news.
const MAX_CONCURRENCY_HARD_LIMIT: usize = 8;
// Chaque pause est plafonnee, y compris lorsqu'un `Retry-After` distant demande davantage.
const MAX_RETRY_BACKOFF: Duration = Duration::from_secs(5);

/// Nom de modele OpenRouter valide avant toute construction de requete.
// La `String` est possedee pour survivre a la configuration; aucun contenu de prompt n'y entre.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct OpenRouterCalendarModel(String);

// Le constructeur maintient l'invariant de format du nouveau type.
impl OpenRouterCalendarModel {
    /// Valide un identifiant de modele court, non vide et sans caractere de controle.
    pub(crate) fn new(value: impl Into<String>) -> Result<Self, OpenRouterCalendarConfigError> {
        // `Into<String>` accepte une variable d'environnement possedee sans clone supplementaire.
        let value = value.into();
        // L'emprunt du texte nettoye evite une allocation avant que tous les controles passent.
        let trimmed = value.trim();
        // Un modele vide ne peut produire qu'une erreur fournisseur permanente.
        if trimmed.is_empty() {
            return Err(OpenRouterCalendarConfigError::EmptyModel);
        }
        // La taille UTF-8 est bornee car c'est cette representation qui part sur le reseau.
        if trimmed.len() > MAX_MODEL_BYTES {
            return Err(OpenRouterCalendarConfigError::ModelTooLong);
        }
        // L'allowlist couvre les identifiants OpenRouter usuels `provider/model:variant`.
        if !trimmed.bytes().all(|byte| {
            // Les caracteres autorises restent visibles et ne peuvent injecter une nouvelle ligne d'en-tete.
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'/' | b':')
        }) {
            return Err(OpenRouterCalendarConfigError::InvalidModel);
        }
        // Une nouvelle chaine compacte garantit aussi que les espaces de bord ne sont jamais envoyes.
        Ok(Self(trimmed.to_owned()))
    }

    /// Expose un emprunt court uniquement au serialiseur de la requete.
    fn as_str(&self) -> &str {
        // Le `&str` partage les octets sans transferer la propriete de la configuration.
        &self.0
    }
}

// Le modele n'est pas un secret et reste utile au diagnostic de configuration.
impl fmt::Debug for OpenRouterCalendarModel {
    // Le formatter est emprunte pendant l'affichage, sans conserver de reference au modele.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Le tuple debug rend explicite le nouveau type plutot qu'une `String` generique.
        formatter
            .debug_tuple("OpenRouterCalendarModel")
            .field(&self.0)
            .finish()
    }
}

/// Cle privee dont aucune implementation de formatage ne revele la valeur.
// Le champ reste prive: seul ce module peut produire l'en-tete `Authorization`.
pub(crate) struct OpenRouterApiKey(String);

// Le constructeur refuse les valeurs manifestement invalides sans les recopier dans l'erreur.
impl OpenRouterApiKey {
    /// Construit une cle possedee et bornee.
    pub(crate) fn new(value: impl Into<String>) -> Result<Self, OpenRouterCalendarConfigError> {
        // La valeur est deplacee depuis la configuration pour eviter une copie de secret.
        let value = value.into();
        // Une cle vide ou uniquement composee d'espaces est toujours une erreur permanente.
        if value.trim().is_empty() {
            return Err(OpenRouterCalendarConfigError::EmptyApiKey);
        }
        // La limite empeche une variable corrompue de devenir un en-tete demesure.
        if value.len() > MAX_API_KEY_BYTES {
            return Err(OpenRouterCalendarConfigError::ApiKeyTooLong);
        }
        // Les caracteres de controle sont interdits par HTTP et pourraient casser la redaction des traces.
        if value.chars().any(char::is_control) {
            return Err(OpenRouterCalendarConfigError::InvalidApiKey);
        }
        // Le secret est conserve exactement: aucun normaliseur ne doit modifier une cle valide.
        Ok(Self(value))
    }

    /// Emprunt prive pour l'en-tete Bearer et le masquage des secrets dans les logs d'erreur.
    fn expose_for_authorization(&self) -> &str {
        // L'emprunt ne cree aucune seconde copie durable de la cle.
        &self.0
    }
}

// Le `Debug` manuel est une barriere explicite contre une fuite lors d'un diagnostic.
impl fmt::Debug for OpenRouterApiKey {
    // Le contenu du formatter est constant quelle que soit la cle reelle.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        // `write_str` n'accede volontairement jamais au champ prive `self.0`.
        formatter.write_str("OpenRouterApiKey([REDACTED])")
    }
}

/// Limites locales qui isolent OpenRouter du pipeline live news.
// `Copy` est adapte a ces petites valeurs scalaires et evite un partage mutable de configuration.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct OpenRouterCalendarLimits {
    // Le timeout s'applique a chaque tentative HTTP complete.
    request_timeout: Duration,
    // La somme des deux prompts est verifiee avant toute admission dans le semaphore.
    max_prompt_bytes: usize,
    // Le corps HTTP est lu par chunks et interrompu des que cette limite est depassee.
    max_response_bytes: usize,
    // Cette valeur compte les nouvelles tentatives apres la tentative initiale.
    max_retries: usize,
    // Le backoff de base est multiplie de facon exponentielle puis plafonne.
    retry_backoff: Duration,
    // Chaque client possede son semaphore reserve au calendrier.
    max_concurrency: usize,
}

// Le constructeur centralise les bornes plutot que de les disperser dans le runtime.
impl OpenRouterCalendarLimits {
    /// Valide toutes les limites avant de creer un client actif.
    pub(crate) fn new(
        request_timeout: Duration,
        max_prompt_bytes: usize,
        max_response_bytes: usize,
        max_retries: usize,
        retry_backoff: Duration,
        max_concurrency: usize,
    ) -> Result<Self, OpenRouterCalendarConfigError> {
        // Un timeout nul echouerait immediatement et provoquerait des retries inutiles.
        if request_timeout.is_zero() || request_timeout > MAX_REQUEST_TIMEOUT {
            return Err(OpenRouterCalendarConfigError::InvalidRequestTimeout);
        }
        // Une limite de prompt doit accepter au moins un octet et respecter le plafond memoire.
        if max_prompt_bytes == 0 || max_prompt_bytes > MAX_PROMPT_BYTES_HARD_LIMIT {
            return Err(OpenRouterCalendarConfigError::InvalidPromptLimit);
        }
        // Une limite de reponse nulle ou superieure au plafond rendrait la lecture non sure.
        if max_response_bytes == 0 || max_response_bytes > MAX_RESPONSE_BYTES_HARD_LIMIT {
            return Err(OpenRouterCalendarConfigError::InvalidResponseLimit);
        }
        // Le budget de retries est un plafond ferme, independant des reponses du fournisseur.
        if max_retries > MAX_RETRIES_HARD_LIMIT {
            return Err(OpenRouterCalendarConfigError::TooManyRetries);
        }
        // Un retry configure doit comporter une pause non nulle pour eviter une boucle serree.
        if max_retries > 0 && retry_backoff.is_zero() {
            return Err(OpenRouterCalendarConfigError::InvalidRetryBackoff);
        }
        // Le backoff de base lui-meme reste plafonne avant la multiplication exponentielle.
        if retry_backoff > MAX_RETRY_BACKOFF {
            return Err(OpenRouterCalendarConfigError::InvalidRetryBackoff);
        }
        // Au moins une place est necessaire, et huit places protegent le live contre un fan-out calendrier.
        if max_concurrency == 0 || max_concurrency > MAX_CONCURRENCY_HARD_LIMIT {
            return Err(OpenRouterCalendarConfigError::InvalidConcurrency);
        }
        // Le type construit ne peut plus contenir de valeur hors borne.
        Ok(Self {
            request_timeout,
            max_prompt_bytes,
            max_response_bytes,
            max_retries,
            retry_backoff,
            max_concurrency,
        })
    }
}

// Le `Debug` manuel rend les limites inspectables sans modifier la politique de redaction du module.
impl fmt::Debug for OpenRouterCalendarLimits {
    // Toutes les valeurs affichees sont quantitatives et ne contiennent aucun contenu metier.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Le builder nomme chaque unite pour eviter une interpretation ambigue des nombres.
        formatter
            .debug_struct("OpenRouterCalendarLimits")
            .field("request_timeout_ms", &self.request_timeout.as_millis())
            .field("max_prompt_bytes", &self.max_prompt_bytes)
            .field("max_response_bytes", &self.max_response_bytes)
            .field("max_retries", &self.max_retries)
            .field("retry_backoff_ms", &self.retry_backoff.as_millis())
            .field("max_concurrency", &self.max_concurrency)
            .finish()
    }
}

/// Configuration immuable du endpoint et du modele OpenRouter calendrier.
// Les champs prives forcent tous les appels a passer par les validations de `new`.
#[derive(Clone)]
pub(crate) struct OpenRouterCalendarClientConfig {
    // L'URL possedee est clonable a faible cout lors de chaque construction de requete.
    endpoint: Url,
    // Le nouveau type garantit un modele valide avant la serialisation.
    model: OpenRouterCalendarModel,
    // Les limites regroupees sont copiees lors de la construction du client.
    limits: OpenRouterCalendarLimits,
}

// La configuration verifie aussi le transport afin de ne jamais envoyer de secret en clair en production.
impl OpenRouterCalendarClientConfig {
    /// Accepte HTTPS partout et HTTP uniquement sur une adresse de boucle locale pour les tests.
    pub(crate) fn new(
        endpoint: Url,
        model: OpenRouterCalendarModel,
        limits: OpenRouterCalendarLimits,
    ) -> Result<Self, OpenRouterCalendarConfigError> {
        // Les credentials dans l'URL risqueraient de fuiter via des couches HTTP externes.
        if !endpoint.username().is_empty() || endpoint.password().is_some() {
            return Err(OpenRouterCalendarConfigError::EndpointContainsCredentials);
        }
        // Une query ou un fragment pourrait contenir un token et n'est pas necessaire a l'API chat.
        if endpoint.query().is_some() || endpoint.fragment().is_some() {
            return Err(OpenRouterCalendarConfigError::EndpointContainsParameters);
        }
        // L'hote est obligatoire pour une requete HTTP distante ou un mock local.
        let Some(host) = endpoint.host_str() else {
            return Err(OpenRouterCalendarConfigError::InvalidEndpoint);
        };
        // `localhost` et les IP loopback permettent les tests sans affaiblir le transport de production.
        let is_loopback = host.eq_ignore_ascii_case("localhost")
            || host
                .parse::<IpAddr>()
                .is_ok_and(|address| address.is_loopback());
        // Toute destination non locale exige TLS pour proteger la cle Bearer en transit.
        if endpoint.scheme() != "https" && !(endpoint.scheme() == "http" && is_loopback) {
            return Err(OpenRouterCalendarConfigError::InsecureEndpoint);
        }
        // Le type final possede l'URL, le modele et les limites deja valides.
        Ok(Self {
            endpoint,
            model,
            limits,
        })
    }
}

// Le chemin de l'endpoint n'est volontairement pas affiche: une configuration mal nommee peut etre sensible.
impl fmt::Debug for OpenRouterCalendarClientConfig {
    // Le formatter recoit uniquement origine, modele et metriques non sensibles.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        // L'hote a deja ete valide par le constructeur et peut etre diagnostique sans chemin ni query.
        let endpoint_host = self.endpoint.host_str().unwrap_or("invalid");
        // Le port explicite est quantitatif et utile pour les mocks locaux.
        let endpoint_port = self.endpoint.port();
        // La cle API ne fait pas partie de cette structure et ne peut donc pas apparaitre ici.
        formatter
            .debug_struct("OpenRouterCalendarClientConfig")
            .field("endpoint_scheme", &self.endpoint.scheme())
            .field("endpoint_host", &endpoint_host)
            .field("endpoint_port", &endpoint_port)
            .field("model", &self.model)
            .field("limits", &self.limits)
            .finish()
    }
}

/// Requete calendrier possedee; ses textes ne disposent d'aucun `Debug` derivant leur contenu.
pub(crate) struct OpenRouterCalendarRequest {
    // Le profil ferme choisit le budget fixe du contrat natif sans valeur arbitraire appelante.
    profile: OpenRouterCalendarRequestProfile,
    // Le prompt deja joint correspond exactement a l'unique message `user` de la baseline.
    prompt: String,
}

// La construction refuse un appel vide avant de prendre une place de concurrence.
impl OpenRouterCalendarRequest {
    /// Construit le profil ingestion natif avec son plafond fixe de 24 000 tokens.
    pub(crate) fn ingestion(prompt: impl Into<String>) -> Result<Self, OpenRouterCalendarError> {
        // Le helper prive partage la validation tout en gardant le choix du profil dans une API fermee.
        Self::from_profile(OpenRouterCalendarRequestProfile::Ingestion, prompt)
    }

    #[cfg(test)]
    pub(crate) fn diagnostic_baseline(
        prompt: impl Into<String>,
    ) -> Result<Self, OpenRouterCalendarError> {
        Self::from_profile(OpenRouterCalendarRequestProfile::DiagnosticBaseline, prompt)
    }

    fn is_ingestion(&self) -> bool {
        !matches!(self.profile, OpenRouterCalendarRequestProfile::Summary)
    }

    /// Construit le profil summary natif, qui omet entierement le champ `max_tokens`.
    pub(crate) fn summary(prompt: impl Into<String>) -> Result<Self, OpenRouterCalendarError> {
        // Aucun entier de tokens libre ne traverse cette frontiere typee.
        Self::from_profile(OpenRouterCalendarRequestProfile::Summary, prompt)
    }

    /// Centralise la possession et le controle de vacuite du prompt complet.
    fn from_profile(
        profile: OpenRouterCalendarRequestProfile,
        prompt: impl Into<String>,
    ) -> Result<Self, OpenRouterCalendarError> {
        // `Into<String>` deplace si possible la chaine deja jointe par le builder de prompt.
        let prompt = prompt.into();
        // Des espaces seuls n'apportent aucune instruction et signalent un bug de construction local.
        if prompt.trim().is_empty() {
            return Err(OpenRouterCalendarError::InvalidPrompt);
        }
        // Les deux champs prives maintiennent le couple profil/payload jusqu'a la serialisation HTTP.
        Ok(Self { profile, prompt })
    }

    /// Retourne le budget fixe du profil sous la forme optionnelle attendue par Serde.
    fn max_tokens(&self) -> Option<u32> {
        // Le `match` exhaustif oblige toute future variante a choisir explicitement son contrat natif.
        match self.profile {
            // L'ingestion fixe exactement 24 000 tokens de sortie.
            OpenRouterCalendarRequestProfile::Ingestion => Some(24_000),
            #[cfg(test)]
            OpenRouterCalendarRequestProfile::DiagnosticBaseline => Some(24_000),
            // Le summary laisse OpenRouter appliquer son contrat par defaut et omet la cle JSON.
            OpenRouterCalendarRequestProfile::Summary => None,
        }
    }

    /// Calcule la taille envoyee sans concatener ni dupliquer le prompt.
    fn byte_len(&self) -> usize {
        // `String::len` compte les octets UTF-8, unite utilisee par la limite HTTP locale.
        self.prompt.len()
    }
}

/// Profils de generation fermes qui portent les deux payloads calendrier natifs.
// `Debug` est sur car l'enum ne contient ni texte, ni secret, ni valeur de configuration libre.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OpenRouterCalendarRequestProfile {
    // L'ingestion qualifie les sources et exige le budget historique de 24 000 tokens.
    Ingestion,
    #[cfg(test)]
    DiagnosticBaseline,
    // Le summary n'envoie aucun `max_tokens`, conformement au workflow de reference.
    Summary,
}

// Le `Debug` revele seulement les tailles, utiles pour diagnostiquer une limite sans exposer le texte.
impl fmt::Debug for OpenRouterCalendarRequest {
    // Le profil ferme et la longueur UTF-8 suffisent a diagnostiquer le payload sans son contenu.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Aucune methode `field` ne recoit une reference a la chaine elle-meme.
        formatter
            .debug_struct("OpenRouterCalendarRequest")
            .field("profile", &self.profile)
            .field("prompt_bytes", &self.prompt.len())
            .finish_non_exhaustive()
    }
}

/// Completion OpenRouter validee et bornee, encore purement transitoire en phase 2.
pub(crate) struct OpenRouterCalendarCompletion {
    // Le modele effectif est type lorsque le fournisseur le retourne.
    response_model: Option<OpenRouterCalendarModel>,
    // Le contenu JSON reste prive et ne doit etre donne qu'au parseur de contrats.
    content: String,
}

// Les accesseurs empruntes evitent une copie du resultat IA entre client et parseur.
impl OpenRouterCalendarCompletion {
    /// Retourne le contenu au parseur Rust sans l'exposer a un logger.
    pub(crate) fn content(&self) -> &str {
        // La duree de vie du retour reste liee a la completion possedee.
        &self.content
    }

    /// Indique le modele effectif sans exposer le contenu de la completion.
    // Cet accesseur sert uniquement aux assertions HTTP; le runtime n'a pas besoin de persister le modele.
    #[cfg(test)]
    pub(crate) fn response_model(&self) -> Option<&OpenRouterCalendarModel> {
        // `as_ref` convertit `Option<T>` en `Option<&T>` sans clone.
        self.response_model.as_ref()
    }
}

// Le `Debug` manuel rend impossible l'affichage accidentel de la reponse IA brute.
impl fmt::Debug for OpenRouterCalendarCompletion {
    // Seules une metadonnee et une taille sont transmises au formatter.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        // `finish_non_exhaustive` rappelle visuellement que le contenu a ete redige.
        formatter
            .debug_struct("OpenRouterCalendarCompletion")
            .field("response_model", &self.response_model)
            .field("content_bytes", &self.content.len())
            .finish_non_exhaustive()
    }
}

/// Erreurs de configuration redigees; aucune variante ne transporte la valeur fautive.
// `thiserror` fournit `Display` et `Error` sans ajouter de chaine dynamique potentiellement sensible.
#[derive(Debug, thiserror::Error, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OpenRouterCalendarConfigError {
    /// Le modele est absent.
    #[error("openrouter_calendar_empty_model")]
    EmptyModel,
    /// Le modele depasse la limite locale.
    #[error("openrouter_calendar_model_too_long")]
    ModelTooLong,
    /// Le modele contient un caractere hors allowlist.
    #[error("openrouter_calendar_invalid_model")]
    InvalidModel,
    /// La cle est absente.
    #[error("openrouter_calendar_empty_api_key")]
    EmptyApiKey,
    /// La cle depasse la limite locale.
    #[error("openrouter_calendar_api_key_too_long")]
    ApiKeyTooLong,
    /// La cle contient un caractere de controle.
    #[error("openrouter_calendar_invalid_api_key")]
    InvalidApiKey,
    /// L'URL ne contient pas d'hote exploitable.
    #[error("openrouter_calendar_invalid_endpoint")]
    InvalidEndpoint,
    /// Les credentials URL sont interdits.
    #[error("openrouter_calendar_endpoint_credentials_forbidden")]
    EndpointContainsCredentials,
    /// Query et fragment sont interdits sur l'endpoint.
    #[error("openrouter_calendar_endpoint_parameters_forbidden")]
    EndpointContainsParameters,
    /// Une destination distante doit utiliser HTTPS.
    #[error("openrouter_calendar_insecure_endpoint")]
    InsecureEndpoint,
    /// Le timeout est nul ou depasse une minute.
    #[error("openrouter_calendar_invalid_timeout")]
    InvalidRequestTimeout,
    /// La limite de prompt sort des bornes du module.
    #[error("openrouter_calendar_invalid_prompt_limit")]
    InvalidPromptLimit,
    /// La limite de reponse sort des bornes du module.
    #[error("openrouter_calendar_invalid_response_limit")]
    InvalidResponseLimit,
    /// Le nombre de retries depasse le budget ferme.
    #[error("openrouter_calendar_too_many_retries")]
    TooManyRetries,
    /// Le backoff est nul avec retries ou depasse son plafond.
    #[error("openrouter_calendar_invalid_retry_backoff")]
    InvalidRetryBackoff,
    /// La concurrence sort de la plage 1..=8.
    #[error("openrouter_calendar_invalid_concurrency")]
    InvalidConcurrency,
}

/// Famille compacte du dernier echec transitoire, sans message fournisseur.
// `Copy` permet de la reporter dans l'erreur finale sans allocation ni contenu distant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OpenRouterCalendarFailureKind {
    /// Le delai local a expire.
    Timeout,
    /// La connexion ou le transport du corps a echoue.
    Transport,
    /// Le fournisseur a applique un rate limit HTTP 429.
    RateLimited,
    /// Le fournisseur a retourne 408 ou une erreur 5xx.
    Server,
}

/// Erreurs runtime fermees du client shadow.
// Toutes les variantes contiennent seulement codes et compteurs compatibles avec des logs rediges.
#[derive(Debug, thiserror::Error, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OpenRouterCalendarError {
    #[error("openrouter_calendar_incomplete_completion_{reason:?}")]
    IncompleteCompletion { reason: CalendarFinishReason },
    /// Un prompt vide a ete refuse localement.
    #[error("openrouter_calendar_invalid_prompt")]
    InvalidPrompt,
    /// Le prompt depasse la limite avant admission.
    #[error("openrouter_calendar_prompt_too_large")]
    PromptTooLarge,
    /// Toutes les places shadow sont occupees; l'appelant ne doit pas attendre.
    #[error("openrouter_calendar_busy")]
    Busy,
    /// Un 4xx permanent ne doit pas consommer le budget de retry.
    #[error("openrouter_calendar_http_status_{status_code}")]
    HttpStatus {
        // Le code seul est sans contenu et suffit au diagnostic de contrat/auth.
        status_code: u16,
    },
    /// La reponse HTTP depasse la limite avant ou pendant sa lecture.
    #[error("openrouter_calendar_response_too_large")]
    ResponseTooLarge,
    /// Classification fermee; aucun corps ni message distant n'est conserve.
    #[error("openrouter_calendar_invalid_response_{kind:?}")]
    InvalidResponse { kind: OpenRouterResponseFailureKind },
    /// Le premier choix ne contient aucun texte exploitable.
    #[error("openrouter_calendar_empty_completion")]
    EmptyCompletion,
    /// Le dernier echec transitoire est retourne apres un nombre ferme de tentatives.
    #[error("openrouter_calendar_retries_exhausted")]
    RetriesExhausted {
        // Le compteur inclut la tentative initiale et prouve la borne appliquee.
        attempts: usize,
        // La famille ne transporte ni URL, ni corps, ni message reqwest.
        last_failure: OpenRouterCalendarFailureKind,
        // Le statut est absent pour un echec transport et present pour HTTP.
        status_code: Option<u16>,
    },
}

/// Etape exacte du rejet, sans donnees controlees par le fournisseur.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OpenRouterResponseFailureKind {
    Request,
    Json,
    Envelope,
    EmbeddedError,
    Model,
    Content,
}

/// Horloge de backoff injectable; seule une duree quantitative traverse ce contrat.
#[async_trait]
trait OpenRouterRetrySleeper: Send + Sync {
    /// Attend sans bloquer le thread Tokio qui execute les autres workers.
    async fn sleep(&self, duration: Duration);
}

/// Implementation de production basee sur le timer asynchrone Tokio.
struct TokioOpenRouterRetrySleeper;

// Le trait isole l'effet temporel afin que les tests restent rapides et deterministes.
#[async_trait]
impl OpenRouterRetrySleeper for TokioOpenRouterRetrySleeper {
    // `async fn` retourne une future que Tokio suspend pendant le delai.
    async fn sleep(&self, duration: Duration) {
        // `.await` rend la main au runtime au lieu d'occuper un thread avec un sleep bloquant.
        sleep(duration).await;
    }
}

/// Client OpenRouter reutilisable et reserve au shadow calendrier.
// `Clone` duplique seulement des handles: reqwest conserve son pool et les `Arc` leur etat partage.
#[derive(Clone)]
pub(crate) struct OpenRouterCalendarClient {
    // Le client est injecte depuis l'application afin de reutiliser connexions et configuration TLS.
    http: Client,
    // La configuration immuable est partagee entre les clones du worker.
    config: Arc<OpenRouterCalendarClientConfig>,
    // La cle reste derriere un `Arc` prive sans derive `Clone` de sa valeur.
    api_key: Arc<OpenRouterApiKey>,
    // Le semaphore n'est jamais attendu: seule `try_acquire_owned` est utilisee.
    admission: Arc<Semaphore>,
    // Le sleeper injectable partage la meme politique entre toutes les tentatives.
    retry_sleeper: Arc<dyn OpenRouterRetrySleeper>,
}

// Les methodes maintiennent l'admission, les retries et la lecture bornee dans une seule abstraction.
impl OpenRouterCalendarClient {
    /// Construit le client autour d'un `reqwest::Client` deja cree par l'application.
    pub(crate) fn new(
        http: Client,
        config: OpenRouterCalendarClientConfig,
        api_key: OpenRouterApiKey,
    ) -> Self {
        // Le nombre de permis est copie avant que `config` soit deplace derriere son `Arc`.
        let max_concurrency = config.limits.max_concurrency;
        // Chaque allocation partagee est creee une fois, puis clonee par handle atomique.
        Self {
            http,
            config: Arc::new(config),
            api_key: Arc::new(api_key),
            admission: Arc::new(Semaphore::new(max_concurrency)),
            retry_sleeper: Arc::new(TokioOpenRouterRetrySleeper),
        }
    }

    /// Execute une completion JSON sans attendre si la capacite calendrier est deja saturee.
    pub(crate) async fn complete_json(
        &self,
        request: &OpenRouterCalendarRequest,
    ) -> Result<OpenRouterCalendarCompletion, OpenRouterCalendarError> {
        crate::features::calendar_logs::operation("complete_json", serde_json::json!({"profile":format!("{:?}", request.profile), "model":self.config.model.as_str()}), async {

        // La taille est refusee avant de prendre un permis ou d'allouer le payload Serde.
        if request.byte_len() > self.config.limits.max_prompt_bytes {
            return Err(OpenRouterCalendarError::PromptTooLarge);
        }
        // `try_acquire_owned` ne cree aucune file d'attente et renvoie immediatement en saturation.
        let _permit = self
            .admission
            .clone()
            .try_acquire_owned()
            // Le semaphore n'est jamais ferme par ce client; les deux cas restent rediges en `Busy`.
            .map_err(|_error: TryAcquireError| OpenRouterCalendarError::Busy)?;
        // Le permis reste possede par `_permit` pendant retries et backoffs, puis se libere par `Drop`.
        self.complete_with_retry(request).await

}).await
    }

    /// Applique un nombre total de tentatives derive du budget de retries valide.
    async fn complete_with_retry(
        &self,
        request: &OpenRouterCalendarRequest,
    ) -> Result<OpenRouterCalendarCompletion, OpenRouterCalendarError> {
        // Ajouter un est sur car `max_retries` est au plus trois.
        let total_attempts = self.config.limits.max_retries + 1;
        // La boucle est finie et son index sert au backoff exponentiel borne.
        for attempt_index in 0..total_attempts {
            // Une tentative retourne soit la completion, soit un echec deja classifie et redige.
            match crate::features::calendar_logs::context(
                serde_json::json!({"http_attempt":attempt_index + 1}),
                self.complete_once(request),
            )
            .await
            {
                // Le premier succes termine sans attendre les tentatives restantes.
                Ok(completion) => return Ok(completion),
                // Un echec permanent est retourne tel quel et n'est jamais retry.
                Err(AttemptError::Permanent(error)) => return Err(error),
                // Un echec transitoire peut etre rejoue tant qu'il reste un budget.
                Err(AttemptError::Retriable(failure)) => {
                    // L'index commence a zero; ajouter un donne le nombre reel de tentatives executees.
                    let attempts = attempt_index + 1;
                    // Le dernier echec devient une erreur compacte prouvant l'epuisement de la borne.
                    if attempts >= total_attempts {
                        return Err(OpenRouterCalendarError::RetriesExhausted {
                            attempts,
                            last_failure: failure.kind,
                            status_code: failure.status_code,
                        });
                    }
                    // `Retry-After` deja plafonne prime sur le backoff local, sinon le calcul reste borne.
                    let delay = failure
                        .retry_after
                        .unwrap_or_else(|| self.retry_delay(attempt_index));
                    // Le retry conserve seulement sa cause et son delai; l'appel complet est deja
                    // trace par la tentative qui vient d'echouer.
                    crate::features::calendar_logs::event(
                        "openrouter.retry",
                        serde_json::json!({"attempt": attempts, "delay_ms": delay.as_millis(), "status": failure.status_code, "failure":format!("{:?}", failure.kind)}),
                    );
                    self.retry_sleeper.sleep(delay).await;
                }
            }
        }
        // La boucle retourne dans tous les cas; ce fallback preserve un contrat redige si elle change plus tard.
        Err(OpenRouterCalendarError::RetriesExhausted {
            attempts: total_attempts,
            last_failure: OpenRouterCalendarFailureKind::Transport,
            status_code: None,
        })
    }

    /// Effectue exactement un POST OpenRouter et valide son enveloppe typee.
    async fn complete_once(
        &self,
        request: &OpenRouterCalendarRequest,
    ) -> Result<OpenRouterCalendarCompletion, AttemptError> {
        crate::features::calendar_logs::operation("complete_once", serde_json::json!({}), async {

        // Le payload emprunte le prompt et le modele seulement pendant la serialisation reqwest.
        let payload = OpenRouterChatRequest {
            model: self.config.model.as_str(),
            // La baseline envoie un unique message utilisateur qui contient deja toutes les instructions.
            messages: [OpenRouterChatMessage {
                role: OpenRouterChatRole::User,
                content: &request.prompt,
            }],
            // La temperature 0.1 reprend la baseline historique tout en gardant une variance faible.
            temperature: 0.1,
            // Serde omet `None`: seul le profil ingestion serialise le budget fixe de 24 000 tokens.
            max_tokens: request.max_tokens(),
            response_format: matches!(request.profile, OpenRouterCalendarRequestProfile::Ingestion)
                .then(ingestion_response_format),
            provider: matches!(request.profile, OpenRouterCalendarRequestProfile::Ingestion)
                .then_some(StructuredProvider {
                    require_parameters: true,
                }),
        };
        // Le builder utilise une copie de l'URL validee et ne concatene aucune valeur utilisateur.
        crate::features::calendar_logs::payload("openrouter.request", &serde_json::to_string(&payload).expect("request serializes"));
        crate::features::calendar_logs::event("openrouter.http_request", serde_json::json!({"method":"POST", "url": self.config.endpoint.as_str(), "timeout_ms":self.config.limits.request_timeout.as_millis()}));
        let send_result = self
            .http
            .post(self.config.endpoint.clone())
            // `bearer_auth` construit l'en-tete sans inserer la cle dans le payload JSON.
            .bearer_auth(self.api_key.expose_for_authorization())
            // `.json` serialise le contrat type et positionne le Content-Type adequat.
            .json(&payload)
            // Le timeout par tentative couvre connexion, envoi et lecture du corps.
            .timeout(self.config.limits.request_timeout)
            .send()
            .await;
        // L'erreur reqwest n'est jamais conservee: seule sa classification sure traverse le module.
        let mut response = match send_result {
            // Le succes transfere la propriete de la reponse a la lecture bornee.
            Ok(response) => response,
            // Les timeouts et erreurs de transport peuvent beneficier d'un retry court.
            Err(error) if error.is_timeout() => {
                return Err(AttemptError::retriable(
                    OpenRouterCalendarFailureKind::Timeout,
                    None,
                    None,
                ));
            }
            // Connexion et corps sont temporaires; une erreur de construction reste permanente.
            Err(error) if error.is_connect() || error.is_body() => {
                return Err(AttemptError::retriable(
                    OpenRouterCalendarFailureKind::Transport,
                    None,
                    None,
                ));
            }
            // Aucun detail reqwest ni URL n'est propage pour les autres erreurs locales.
            Err(_error) => {
                return Err(AttemptError::Permanent(
                    OpenRouterCalendarError::InvalidResponse {
                        kind: OpenRouterResponseFailureKind::Request,
                    },
                ));
            }
        };
        // Le statut est copie avant toute consommation du corps HTTP.
        let status = response.status();
        crate::features::calendar_logs::event("openrouter.http_response", serde_json::json!({"status": status.as_u16(), "headers": format!("{:?}", response.headers())}));
        if !status.is_success() {
            // Lire seulement le corps d'erreur borne, sans modifier le statut ni le budget de retry.
            self.log_http_error_response(&mut response, request).await;
        }
        // 429 est explicitement retriable et peut fournir une attente serveur plafonnee.
        if status == StatusCode::TOO_MANY_REQUESTS {
            return Err(AttemptError::retriable(
                OpenRouterCalendarFailureKind::RateLimited,
                Some(status.as_u16()),
                retry_after_duration(response.headers()),
            ));
        }
        // 408 et 5xx representent des indisponibilites transitoires du downstream.
        if status == StatusCode::REQUEST_TIMEOUT || status.is_server_error() {
            return Err(AttemptError::retriable(
                OpenRouterCalendarFailureKind::Server,
                Some(status.as_u16()),
                retry_after_duration(response.headers()),
            ));
        }
        // Les autres 4xx signalent auth, quota permanent ou contrat et ne sont jamais retries.
        if status.is_client_error() {
            return Err(AttemptError::Permanent(
                OpenRouterCalendarError::HttpStatus {
                    status_code: status.as_u16(),
                },
            ));
        }
        // Une redirection non suivie ou tout autre statut hors succes ne doit pas etre parse comme JSON IA.
        if !status.is_success() {
            return Err(AttemptError::Permanent(
                OpenRouterCalendarError::HttpStatus {
                    status_code: status.as_u16(),
                },
            ));
        }
        // Un Content-Length annonce trop grand evite meme la premiere allocation de corps.
        if response
            .content_length()
            .is_some_and(|length| length > self.config.limits.max_response_bytes as u64)
        {
            error_log::emit(
                status.as_u16(),
                self.config.model.as_str(),
                &[],
                self.api_key.expose_for_authorization(),
                &request.prompt,
                true,
            );
            return Err(AttemptError::Permanent(
                OpenRouterCalendarError::ResponseTooLarge,
            ));
        }
        // La capacite initiale est elle aussi bornee; la reallocation restera controlee par les chunks.
        let initial_capacity = response
            .content_length()
            .and_then(|length| usize::try_from(length).ok())
            .unwrap_or(0)
            .min(self.config.limits.max_response_bytes);
        // `mut` est necessaire car chaque chunk etend progressivement le meme buffer possede.
        let mut body = Vec::with_capacity(initial_capacity);
        // Chaque tour attend un chunk asynchrone, sans charger le corps complet d'un coup.
        loop {
            // Une erreur de lecture est classifiee sans conserver son texte potentiellement bavard.
            let chunk = match response.chunk().await {
                // `Some` transporte le prochain segment, `None` marque proprement la fin du flux.
                Ok(chunk) => chunk,
                // Un timeout pendant le corps reste transitoire et reutilise le meme budget ferme.
                Err(error) if error.is_timeout() => {
                    error_log::emit(
                        status.as_u16(),
                        self.config.model.as_str(),
                        &body,
                        self.api_key.expose_for_authorization(),
                        &request.prompt,
                        true,
                    );
                    return Err(AttemptError::retriable(
                        OpenRouterCalendarFailureKind::Timeout,
                        None,
                        None,
                    ));
                }
                // Les autres erreurs de corps sont traitees comme transport temporaire.
                Err(_error) => {
                    error_log::emit(
                        status.as_u16(),
                        self.config.model.as_str(),
                        &body,
                        self.api_key.expose_for_authorization(),
                        &request.prompt,
                        true,
                    );
                    return Err(AttemptError::retriable(
                        OpenRouterCalendarFailureKind::Transport,
                        None,
                        None,
                    ));
                }
            };
            // Le pattern `let Some` termine la boucle sans branche imbriquee quand le flux est fini.
            let Some(chunk) = chunk else {
                break;
            };
            // Le calcul sature avant comparaison pour rester sur meme en cas de tailles extremes.
            if body.len().saturating_add(chunk.len()) > self.config.limits.max_response_bytes {
                error_log::emit(
                    status.as_u16(),
                    self.config.model.as_str(),
                    &body,
                    self.api_key.expose_for_authorization(),
                    &request.prompt,
                    true,
                );
                return Err(AttemptError::Permanent(
                    OpenRouterCalendarError::ResponseTooLarge,
                ));
            }
            // `extend_from_slice` copie uniquement le chunk valide dans le buffer borne.
            body.extend_from_slice(&chunk);
        }
        crate::features::calendar_logs::payload("openrouter.response_body", &String::from_utf8_lossy(&body));
        let log_response_error = || {
            error_log::emit_without_payload(
                status.as_u16(),
                self.config.model.as_str(),
                &body,
                self.api_key.expose_for_authorization(),
                &request.prompt,
                false,
            )
        };
        // Le type Serde refuse toute enveloppe qui ne respecte pas la reponse OpenRouter attendue.
        let parsed: OpenRouterChatResponse = serde_json::from_slice(&body).map_err(|error| {
            log_response_error();
            // L'erreur de parsing et le corps ne quittent jamais cette closure de redaction.
            let kind = match error.classify() {
                serde_json::error::Category::Data => OpenRouterResponseFailureKind::Envelope,
                _ => OpenRouterResponseFailureKind::Json,
            };
            AttemptError::Permanent(OpenRouterCalendarError::InvalidResponse { kind })
        })?;
        // Les marqueurs embarques par le contrat HTTP priment sur tout fallback summary.
        if parsed.error_marker.is_truthy
            || parsed.status_code_marker.is_failure
            || parsed.status_marker.is_failure
        {
            // Le detail est reserve aux logs serveur, pas au Result ni aux artefacts admin.
            log_response_error();
            return Err(AttemptError::Permanent(
                OpenRouterCalendarError::InvalidResponse {
                    kind: OpenRouterResponseFailureKind::EmbeddedError,
                },
            ));
        }
        // Un modele present est revalide par le meme nouveau type que la configuration.
        let response_model = parsed
            .model
            .map(OpenRouterCalendarModel::new)
            .transpose()
            .map_err(|_error| {
                log_response_error();
                AttemptError::Permanent(OpenRouterCalendarError::InvalidResponse {
                    kind: OpenRouterResponseFailureKind::Model,
                })
            })?;
        // Les `Option` reproduisent `response?.choices?.[0]?.message?.content` de la baseline.
        let choice = parsed
            .choices
            .into_iter()
            .next()
            // Un choix JSON `null` est aplati comme un choix absent.
            .flatten();
        if request.is_ingestion() {
            let reason = choice
                .as_ref()
                .and_then(|choice| choice.finish_reason)
                .unwrap_or_default();
            if reason != CalendarFinishReason::Stop {
                log_response_error();
                return Err(AttemptError::Permanent(
                    OpenRouterCalendarError::IncompleteCompletion { reason },
                ));
            }
        }
        let content = choice
            // Le message lui-meme peut manquer dans une enveloppe partielle.
            .and_then(|choice| choice.message)
            // Le champ absent ou `null` reste `None` jusqu'a la normalisation profilee.
            .and_then(|message| message.content);
        // Le summary produit ses fallbacks sur un contenu falsy; l'ingestion conserve un contrat strict.
        let mut content =
            normalize_openrouter_content(request.profile, content).map_err(|_error| {
                log_response_error();
                // Une forme non admise ou une serialisation impossible reste un code sans contenu brut.
                AttemptError::Permanent(OpenRouterCalendarError::InvalidResponse {
                    kind: OpenRouterResponseFailureKind::Content,
                })
            })?;
        // Une chaine vide est refusee avant le parsing calendrier pour produire une cause stable.
        if content.trim().is_empty() {
            // Le profil summary remplace cette valeur falsy par `{}` avant ses textes fallback.
            if request.profile == OpenRouterCalendarRequestProfile::Summary {
                content = "{}".to_owned();
            } else {
                log_response_error();
                // L'ingestion exige toujours un payload `items` explicite.
                return Err(AttemptError::Permanent(
                    OpenRouterCalendarError::EmptyCompletion,
                ));
            }
        }
        // La completion possedee reste en memoire et son `Debug` masque le contenu.
        crate::features::calendar_logs::event(
            "openrouter.content_extracted",
            serde_json::json!({"bytes": content.len()}),
        );
        Ok(OpenRouterCalendarCompletion {
            response_model,
            content,
        })

}).await
    }

    async fn log_http_error_response(
        &self,
        response: &mut reqwest::Response,
        request: &OpenRouterCalendarRequest,
    ) {
        let mut body = Vec::new();
        let mut incomplete = false;
        // Le timeout de la requete originale couvre aussi cette lecture ; aucun delai ni retry ajoute.
        loop {
            match response.chunk().await {
                Ok(Some(chunk)) => {
                    let remaining = self
                        .config
                        .limits
                        .max_response_bytes
                        .saturating_sub(body.len());
                    body.extend_from_slice(&chunk[..chunk.len().min(remaining)]);
                    if chunk.len() > remaining {
                        incomplete = true;
                        break;
                    }
                }
                Ok(None) => break,
                Err(_) => {
                    incomplete = true;
                    break;
                }
            }
        }
        error_log::emit(
            response.status().as_u16(),
            self.config.model.as_str(),
            &body,
            self.api_key.expose_for_authorization(),
            &request.prompt,
            incomplete,
        );
    }

    /// Calcule `base * 2^index` sans depasser cinq secondes.
    fn retry_delay(&self, attempt_index: usize) -> Duration {
        // Le decalage est petit car il est borne par le nombre maximal de retries.
        let multiplier = 1_u32.checked_shl(attempt_index as u32).unwrap_or(u32::MAX);
        // `saturating_mul` evite un overflow de `Duration` avant l'application du plafond.
        self.config
            .limits
            .retry_backoff
            .saturating_mul(multiplier)
            .min(MAX_RETRY_BACKOFF)
    }

    // Cette injection est compilee seulement pour les tests et ne devient pas une API runtime.
    #[cfg(test)]
    fn with_retry_sleeper(mut self, retry_sleeper: Arc<dyn OpenRouterRetrySleeper>) -> Self {
        // La mutabilite locale remplace le handle avant que le client soit partage par les scenarios.
        self.retry_sleeper = retry_sleeper;
        // Renvoyer `Self` permet une construction fluide sans exposer le champ du trait objet.
        self
    }
}

// Le `Debug` manuel prouve les limites actives sans afficher endpoint complet, cle ou client reqwest.
impl fmt::Debug for OpenRouterCalendarClient {
    // Les `Arc` sont empruntes; leur compteur de references n'est pas une information metier utile.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        // La cle passe par son propre `Debug` constant et redige.
        formatter
            .debug_struct("OpenRouterCalendarClient")
            .field("config", &self.config)
            .field("api_key", &self.api_key)
            .field("available_permits", &self.admission.available_permits())
            .finish_non_exhaustive()
    }
}

/// Contrat JSON envoye a `/chat/completions`.
// Les durees de vie lient les emprunts a l'appel courant et empechent leur stockage dans le client.
#[derive(Serialize)]
struct OpenRouterChatRequest<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    provider: Option<StructuredProvider>,
    // Le modele est emprunte depuis la configuration validee.
    model: &'a str,
    // Le tableau de taille un fige l'unique message `user` produit par les deux profils natifs.
    messages: [OpenRouterChatMessage<'a>; 1],
    // La temperature numerique reste deterministe pour la parite de fixtures.
    temperature: f32,
    // `skip_serializing_if` supprime la cle pour le summary au lieu d'envoyer explicitement `null`.
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
}

/// Message chat minimal avec role ferme.
// L'emprunt de contenu evite une troisieme copie du prompt lors de la construction du payload.
#[derive(Serialize)]
struct OpenRouterChatMessage<'a> {
    // L'enum est serialise en minuscules par Serde.
    role: OpenRouterChatRole,
    // La chaine est empruntee et disparait avec la tentative.
    content: &'a str,
}

/// Roles strictement necessaires au prompt calendrier.
// `Copy` est possible car les variantes ne contiennent aucune donnee.
#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
enum OpenRouterChatRole {
    // Le prompt complet, regles et snapshot inclus, utilise le seul role present dans la baseline.
    User,
}

/// Enveloppe de reponse minimale acceptee depuis OpenRouter.
// Les champs inconnus comme `usage` sont ignores par defaut sans entrer dans les contrats inter-etapes.
#[derive(Deserialize)]
struct OpenRouterChatResponse {
    // Le modele effectif est optionnel pour tolerer les mocks minimaux de la baseline.
    model: Option<String>,
    // La valeur distante `error` est reduite a sa truthiness puis immediatement abandonnee.
    #[serde(default, rename = "error")]
    error_marker: OpenRouterTruthyMarker,
    // Le camelCase appartient au contrat HTTP et ne conserve que le resultat de `>= 400`.
    #[serde(default, rename = "statusCode")]
    status_code_marker: OpenRouterFailureStatusMarker,
    // Certains retours utilisent le nom court `status`; la meme coercition y est appliquee.
    #[serde(default, rename = "status")]
    status_marker: OpenRouterFailureStatusMarker,
    // Une liste absente devient vide afin que le profil summary applique son fallback ferme.
    #[serde(default)]
    // Chaque choix est optionnel pour traiter aussi un premier element JSON `null` comme absent.
    choices: Vec<Option<OpenRouterChatChoice>>,
}

/// Truthiness JavaScript compacte d'un champ `error` OpenRouter du contrat historique.
// Le type ne derive pas `Debug`: seul son booleen prive est lu juste apres la deserialisation.
#[derive(Default)]
struct OpenRouterTruthyMarker {
    // `true` signifie que le contrat historique prend le chemin d'erreur, sans conserver pourquoi.
    is_truthy: bool,
}

// L'implementation manuelle consomme toute forme JSON sans jamais la placer dans un `Value` durable.
impl<'de> Deserialize<'de> for OpenRouterTruthyMarker {
    // Le deserialiseur est generique afin de rester compatible avec serde_json sans en dependre dans le trait.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        // Cette borne est le contrat Serde standard pour lire une valeur pendant `'de`.
        D: Deserializer<'de>,
    {
        // `deserialize_any` delegue la forme concrete au visiteur qui la reduit a un booleen.
        deserializer.deserialize_any(OpenRouterTruthyVisitor)
    }
}

/// Visiteur sans etat qui reproduit seulement la truthiness utile de JavaScript.
struct OpenRouterTruthyVisitor;

// Le visiteur consomme les valeurs complexes avec `IgnoredAny`, donc aucun message n'est retenu.
impl<'de> Visitor<'de> for OpenRouterTruthyVisitor {
    // Le type produit reste le marqueur prive, jamais la valeur distante.
    type Value = OpenRouterTruthyMarker;

    // `expecting` est une description statique et ne contient aucun extrait de reponse.
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Cette chaine peut apparaitre dans une erreur Serde sans exposer le body fautif.
        formatter.write_str("un marqueur JSON d'erreur OpenRouter")
    }

    // `null` est falsy en JavaScript.
    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(OpenRouterTruthyMarker { is_truthy: false })
    }

    // `Option::None` represente egalement un JSON null pour certains deserialiseurs.
    fn visit_none<E>(self) -> Result<Self::Value, E> {
        // Retourner directement evite d'imposer ici une borne d'erreur uniquement utile a Serde.
        Ok(OpenRouterTruthyMarker { is_truthy: false })
    }

    // Un booleen conserve directement sa truthiness.
    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
        Ok(OpenRouterTruthyMarker { is_truthy: value })
    }

    // Zero est falsy; tout entier signe non nul est truthy.
    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
        Ok(OpenRouterTruthyMarker {
            is_truthy: value != 0,
        })
    }

    // La meme regle s'applique aux entiers non signes.
    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
        Ok(OpenRouterTruthyMarker {
            is_truthy: value != 0,
        })
    }

    // JSON interdit NaN; un flottant nul reste le seul cas falsy disponible ici.
    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E> {
        Ok(OpenRouterTruthyMarker {
            is_truthy: value != 0.0,
        })
    }

    // Une chaine vide est falsy, tandis qu'un message non vide marque une erreur.
    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
        Ok(OpenRouterTruthyMarker {
            is_truthy: !value.is_empty(),
        })
    }

    // Un tableau est toujours truthy en JavaScript, meme vide; son contenu est seulement draine.
    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        // `SeqAccess` permet de parcourir la valeur sans en connaitre ni stocker les elements.
        A: SeqAccess<'de>,
    {
        // Chaque `IgnoredAny` fait avancer le parseur puis est immediatement detruit.
        while sequence.next_element::<IgnoredAny>()?.is_some() {}
        // La forme tableau suffit a produire la truthiness attendue.
        Ok(OpenRouterTruthyMarker { is_truthy: true })
    }

    // Un objet est egalement toujours truthy; cles et valeurs sont toutes ignorees.
    fn visit_map<A>(self, mut object: A) -> Result<Self::Value, A::Error>
    where
        // `MapAccess` expose les paires une a une sans allocation de map intermediaire.
        A: MapAccess<'de>,
    {
        // Le type `IgnoredAny` garantit qu'aucun champ `message` ne survit a cette boucle.
        while object.next_entry::<IgnoredAny, IgnoredAny>()?.is_some() {}
        // La forme objet suffit, meme lorsque le body `error` est vide.
        Ok(OpenRouterTruthyMarker { is_truthy: true })
    }
}

/// Resultat compact de la coercition JavaScript `status >= 400`.
// Comme le marqueur d'erreur, ce type ne derive ni `Debug` ni serialisation.
#[derive(Default)]
struct OpenRouterFailureStatusMarker {
    // Le booleen prive est la seule information conservee depuis le statut embarque.
    is_failure: bool,
}

// Le deserialiseur manuel accepte nombres et chaines numeriques comme la comparaison JavaScript.
impl<'de> Deserialize<'de> for OpenRouterFailureStatusMarker {
    // Le type generique `D` fournit la valeur sans l'attacher a la duree de vie du resultat.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        // La borne Serde autorise tous les formats compatibles avec le visiteur.
        D: Deserializer<'de>,
    {
        // Le visiteur reduit immediatement la valeur a `is_failure`.
        deserializer.deserialize_any(OpenRouterFailureStatusVisitor)
    }
}

/// Visiteur de statut qui rejette seulement les valeurs coercibles superieures ou egales a 400.
struct OpenRouterFailureStatusVisitor;

/// Valeur d'un singleton tableau reduite aux formes que `Number([value])` peut rendre numeriques.
// La variante `Other` reste transitoire et bornee par le corps HTTP de 1 Mo; elle permet les tableaux imbriques JS.
#[derive(Deserialize)]
#[serde(untagged)]
enum OpenRouterArrayStatusValue {
    // Un nombre JSON reste directement comparable au seuil HTTP.
    Number(f64),
    // Une chaine numerique est coercible apres nettoyage de ses espaces.
    Text(String),
    // Toute autre forme JSON est reduite recursivement puis detruite, sans `Debug` ni propagation d'erreur.
    Other(serde_json::Value),
}

// Le helper consomme le singleton avant de calculer le statut, sans derive `Debug`.
impl OpenRouterArrayStatusValue {
    /// Reproduit la partie utile de `Number([value])` pour les statuts HTTP.
    fn coerced_number(self) -> Option<f64> {
        // Le `match` est exhaustif et detruit toute valeur non necessaire immediatement.
        match self {
            Self::Number(value) => Some(value),
            Self::Text(value) => value.trim().parse::<f64>().ok(),
            // JavaScript aplatit aussi les singletons imbriques, par exemple `Number([["500"]])`.
            Self::Other(value) => js_status_number(&value, false, 0),
        }
    }
}

/// Reproduit la coercition utile au seuil `>= 400` sans construire la chaine complete d'un tableau.
fn js_status_number(
    value: &serde_json::Value,
    nested_array_element: bool,
    depth: usize,
) -> Option<f64> {
    // Une profondeur fermee empeche un JSON artificiellement imbrique de monopoliser la pile du worker.
    if depth > 32 {
        return None;
    }
    match value {
        // `Number(null)` et `Number([null])` valent tous deux zero pour la comparaison qui nous interesse.
        serde_json::Value::Null => Some(0.0),
        // Un booleen racine vaut zero/un; dans un tableau il devient le texte non numerique `true`/`false`.
        serde_json::Value::Bool(value) if !nested_array_element => {
            Some(f64::from(u8::from(*value)))
        }
        serde_json::Value::Bool(_) => None,
        // Serde garantit deja la representation JSON finie avant cette conversion optionnelle.
        serde_json::Value::Number(value) => value.as_f64(),
        // Une chaine vide devient zero en JavaScript; les autres suivent la conversion numerique usuelle.
        serde_json::Value::String(value) if value.trim().is_empty() => Some(0.0),
        serde_json::Value::String(value) => value.trim().parse::<f64>().ok(),
        // `[]` devient zero; un singleton delegue a son element; plusieurs valeurs produisent une virgule/NaN.
        serde_json::Value::Array(values) if values.is_empty() => Some(0.0),
        serde_json::Value::Array(values) if values.len() == 1 => {
            js_status_number(&values[0], true, depth + 1)
        }
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => None,
    }
}

// Le visiteur gere aussi `[]`/`[500]`; les objets restent draines puis non numeriques.
impl<'de> Visitor<'de> for OpenRouterFailureStatusVisitor {
    // Le resultat ne contient jamais le statut original.
    type Value = OpenRouterFailureStatusMarker;

    // La description d'erreur reste statique et redigee.
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("un statut JSON OpenRouter coercible")
    }

    // Null se compare comme zero en JavaScript et ne marque donc pas un echec.
    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(OpenRouterFailureStatusMarker { is_failure: false })
    }

    // Certains deserialiseurs appellent `visit_none` pour null.
    fn visit_none<E>(self) -> Result<Self::Value, E> {
        // Null se compare comme zero; aucune delegation generique supplementaire n'est necessaire.
        Ok(OpenRouterFailureStatusMarker { is_failure: false })
    }

    // Un booleen devient zero ou un, toujours inferieur a 400.
    fn visit_bool<E>(self, _value: bool) -> Result<Self::Value, E> {
        Ok(OpenRouterFailureStatusMarker { is_failure: false })
    }

    // La comparaison signee reproduit directement le seuil du contrat.
    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
        Ok(OpenRouterFailureStatusMarker {
            is_failure: value >= 400,
        })
    }

    // La comparaison non signee evite toute conversion potentiellement tronquante.
    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
        Ok(OpenRouterFailureStatusMarker {
            is_failure: value >= 400,
        })
    }

    // Les statuts decimaux restent comparables meme s'ils sont inhabituels.
    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E> {
        Ok(OpenRouterFailureStatusMarker {
            is_failure: value >= 400.0,
        })
    }

    // JavaScript convertit une chaine numerique avant l'operateur `>=`.
    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
        // Une chaine non numerique devient NaN et rend la comparaison fausse.
        let is_failure = value
            .trim()
            .parse::<f64>()
            .is_ok_and(|status| status >= 400.0);
        // La chaine elle-meme est abandonnee avant la construction du marqueur.
        Ok(OpenRouterFailureStatusMarker { is_failure })
    }

    // Un tableau vide vaut zero et un singleton numerique prend la valeur de son unique element en JavaScript.
    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        // L'acces iteratif ne reserve aucun vecteur.
        A: SeqAccess<'de>,
    {
        // Le premier element est reduit a une valeur numerique optionnelle sans conserver son JSON brut.
        let first = sequence.next_element::<OpenRouterArrayStatusValue>()?;
        // La presence d'un second element rend `Array.toString()` non numerique pour un statut HTTP usuel.
        let has_second = sequence.next_element::<IgnoredAny>()?.is_some();
        // Drainer le reste garantit que le parseur JSON termine proprement la valeur.
        while sequence.next_element::<IgnoredAny>()?.is_some() {}
        // `[]` devient zero; `[500]` devient 500; plusieurs elements deviennent une chaine avec virgule.
        let is_failure = !has_second
            && first
                .and_then(OpenRouterArrayStatusValue::coerced_number)
                .is_some_and(|status| status >= 400.0);
        // Seul le verdict du seuil survit a la visite du tableau.
        Ok(OpenRouterFailureStatusMarker { is_failure })
    }

    // Un objet de statut est egalement non numerique et entierement ignore.
    fn visit_map<A>(self, mut object: A) -> Result<Self::Value, A::Error>
    where
        // `MapAccess` permet le drainage sans conserver les cles ou messages.
        A: MapAccess<'de>,
    {
        // Chaque paire est consommee dans des marqueurs jetables.
        while object.next_entry::<IgnoredAny, IgnoredAny>()?.is_some() {}
        // Aucun statut original n'est conserve dans le resultat.
        Ok(OpenRouterFailureStatusMarker { is_failure: false })
    }
}

/// Choix chat dont seul le message type est necessaire.
#[derive(Deserialize)]
struct OpenRouterChatChoice {
    #[serde(default, deserialize_with = "deserialize_finish_reason")]
    finish_reason: Option<CalendarFinishReason>,
    // Les index et finish reasons distants ne sont pas conserves; le message peut etre absent ou nul.
    message: Option<OpenRouterChatResponseMessage>,
}

fn deserialize_finish_reason<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<CalendarFinishReason>, D::Error> {
    // Unknown provider payloads never enter a diagnostic, including non-string markers.
    let value = serde_json::Value::deserialize(deserializer)?;
    if value.is_null() {
        return Ok(None);
    }
    Ok(Some(match value.as_str() {
        Some("stop") => CalendarFinishReason::Stop,
        Some("length") => CalendarFinishReason::Length,
        Some("content_filter") => CalendarFinishReason::ContentFilter,
        Some("tool_calls") => CalendarFinishReason::ToolCalls,
        Some("function_call") => CalendarFinishReason::FunctionCall,
        Some("error") => CalendarFinishReason::Error,
        _ => CalendarFinishReason::Unknown,
    }))
}

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CalendarFinishReason {
    Stop,
    Length,
    ContentFilter,
    ToolCalls,
    FunctionCall,
    Error,
    #[default]
    Missing,
    #[serde(other)]
    Unknown,
}

#[derive(Serialize)]
struct StructuredProvider {
    require_parameters: bool,
}

fn ingestion_response_format() -> serde_json::Value {
    use serde_json::json;
    let text = json!({"type":"string"});
    let nullable = json!({"type":["string","null"]});
    let importance = json!({"type":"string","enum":["low","medium","high"]});
    let properties = json!({
        "eventTime": nullable, "sourceTitle": text, "translatedTitle": text, "ttsText": text,
        "country": nullable, "currency": nullable, "forecastValue": nullable,
        "previousValue": nullable, "actualValue": nullable,
        "sourceImportance": importance, "aiImportance": importance, "finalImportance": importance,
        "category":{"type":"string","enum":["macro","rates","inflation","central_bank","geopolitics","company","crypto","other"]},
        "reasoning":text, "confidence":{"type":"number","minimum":0,"maximum":1},
        "sources":{"type":"array","minItems":1,"maxItems":16,"items":{
            "type":"object","additionalProperties":false,
            "required":["source","externalId","rawTitle","raw"],
            "properties":{
                "source":{"type":"string","enum":super::feeds::CalendarFeedSource::ALL.map(|s|s.name())},
                "externalId":text,"rawTitle":text,
                "raw":{"type":"object","properties":{},"additionalProperties":false}
            }
        }}
    });
    let required = properties
        .as_object()
        .expect("static schema object")
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    json!({"type":"json_schema","json_schema":{"name":"calendar_ingestion_v1","strict":true,"schema":{
        "type":"object","additionalProperties":false,"required":["items"],
        "properties":{"items":{"type":"array","maxItems":512,"items":{
            "type":"object","additionalProperties":false,"required":required,"properties":properties
        }}}
    }}})
}

/// Message de reponse dont le contenu peut etre explicitement nul.
#[derive(Deserialize)]
struct OpenRouterChatResponseMessage {
    // `Option` distingue `null`; l'enum type accepte les deux formes deja gerees par le contrat.
    content: Option<OpenRouterChatContent>,
}

/// Contenu assistant OpenRouter sous forme de texte, tableau ou objet direct.
// `Value` reste confine a cette frontiere de parsing privee; la completion inter-etape reste une `String`.
#[derive(Deserialize)]
#[serde(untagged)]
enum OpenRouterChatContent {
    // La forme chaine reste le contrat nominal des completions calendrier.
    Text(String),
    // Le tableau reste brut ici afin que le profil summary puisse le reserialiser sans perdre sa forme.
    Parts(Vec<serde_json::Value>),
    // Un objet direct est accepte par `parseMaybeJson` dans les deux profils natifs.
    Object(serde_json::Map<String, serde_json::Value>),
    // Nombres et booleens sont falsy/non exploitables pour summary et invalides pour ingestion.
    Scalar(OpenRouterChatScalar),
}

/// Scalaire JSON explicitement ferme aux nombres et booleens.
// Ce type ne derive pas `Debug`, ce qui empeche une valeur distante de rejoindre une trace par accident.
#[derive(Deserialize)]
#[serde(untagged)]
enum OpenRouterChatScalar {
    // La variante booleenne couvre `true` et `false` sans conversion textuelle.
    Boolean(bool),
    // `Number` preserve la representation JSON mais ne quitte jamais cette frontiere.
    Number(serde_json::Number),
}

// Cette conversion consomme le contenu parse et ne garde aucune seconde copie des segments.
impl OpenRouterChatContent {
    /// Retourne le texte transmis au canonicaliseur selon les regles propres a l'etape native.
    fn into_text(
        self,
        profile: OpenRouterCalendarRequestProfile,
    ) -> Result<String, serde_json::Error> {
        #[cfg(test)]
        let profile = if profile == OpenRouterCalendarRequestProfile::DiagnosticBaseline {
            OpenRouterCalendarRequestProfile::Ingestion
        } else {
            profile
        };
        // Une chaine simple est deja dans la forme attendue pour les deux profils.
        match (profile, self) {
            (_, Self::Text(text)) => Ok(text),
            // L'ingestion joint chaque `text|content` par retour ligne avant son parsing JSON.
            (OpenRouterCalendarRequestProfile::Ingestion, Self::Parts(parts)) => Ok(parts
                .into_iter()
                .map(openrouter_ingestion_part_text)
                .collect::<Vec<_>>()
                .join("\n")
                .trim()
                .to_owned()),
            // Le profil summary conserve un tableau comme objet JavaScript; Rust preserve donc le JSON array.
            (OpenRouterCalendarRequestProfile::Summary, Self::Parts(parts)) => {
                serde_json::to_string(&serde_json::Value::Array(parts))
            }
            // Un objet direct est deja le resultat parse du fournisseur; sa serialisation compacte alimente le canonicaliseur.
            (_, Self::Object(object)) => serde_json::to_string(&serde_json::Value::Object(object)),
            // Summary applique `{}` a tout scalaire; ses valeurs ne doivent pas entrer dans les fallbacks.
            (OpenRouterCalendarRequestProfile::Summary, Self::Scalar(scalar)) => {
                // Le destructuring consomme la valeur sans la formater ni la conserver.
                match scalar {
                    OpenRouterChatScalar::Boolean(_value) => Ok("{}".to_owned()),
                    OpenRouterChatScalar::Number(_value) => Ok("{}".to_owned()),
                }
            }
            // Ingestion ne peut construire un payload `{items}` depuis un nombre ou un booleen.
            (OpenRouterCalendarRequestProfile::Ingestion, Self::Scalar(_scalar)) => Err(
                serde_json::Error::io(std::io::Error::other("unsupported ingestion scalar")),
            ),
            #[cfg(test)]
            (OpenRouterCalendarRequestProfile::DiagnosticBaseline, _) => {
                unreachable!("normalized diagnostic profile")
            }
        }
    }
}

/// Normalise l'absence ou la valeur recue sans exposer son contenu dans une erreur.
fn normalize_openrouter_content(
    profile: OpenRouterCalendarRequestProfile,
    content: Option<OpenRouterChatContent>,
) -> Result<String, serde_json::Error> {
    // Un contenu present applique les regles texte/objet/tableau/scalaires propres au profil.
    match content {
        Some(content) => content.into_text(profile),
        // Le profil summary remplace `undefined` ou `null` par un objet vide avant ses fallbacks.
        None if profile == OpenRouterCalendarRequestProfile::Summary => Ok("{}".to_owned()),
        // Ingestion signale l'absence avec le meme chemin d'erreur ferme que les formes invalides.
        None => Err(serde_json::Error::io(std::io::Error::other(
            "missing ingestion content",
        ))),
    }
}

/// Extrait un segment ingestion avec l'ordre `string`, `text`, `content`, puis vide du contrat.
fn openrouter_ingestion_part_text(part: serde_json::Value) -> String {
    // Une chaine directe est la premiere forme acceptee par le mapping JavaScript.
    match part {
        serde_json::Value::String(text) => text,
        // Un objet garde seulement les deux alias textuels; ses autres champs ne quittent pas cette frontiere.
        serde_json::Value::Object(mut object) => object
            .remove("text")
            // Une chaine vide est falsey en JavaScript et doit laisser place a `content`.
            .and_then(non_empty_json_text)
            .or_else(|| object.remove("content").and_then(non_empty_json_text))
            .unwrap_or_default(),
        // Les autres formes produisent une chaine vide dans le mapping historique.
        _ => String::new(),
    }
}

/// Convertit une valeur d'alias uniquement lorsqu'elle porte un texte non vide.
fn non_empty_json_text(value: serde_json::Value) -> Option<String> {
    // Seule la forme `String` correspond au contenu OpenRouter utile dans ce contrat type.
    match value {
        serde_json::Value::String(text) if !text.is_empty() => Some(text),
        // Null, scalaire non textuel, objet et tableau retombent sur l'alias suivant ou la chaine vide.
        _ => None,
    }
}

/// Echec interne d'une tentative avant application de la politique de retry.
#[derive(Debug)]
enum AttemptError {
    // Les erreurs permanentes sont deja converties dans le contrat public redige.
    Permanent(OpenRouterCalendarError),
    // Les erreurs temporaires conservent seulement statut et attente quantitative.
    Retriable(RetriableFailure),
}

// Le helper evite de repeter la construction de la preuve transitoire.
impl AttemptError {
    /// Construit une erreur retriable sans corps ni message distant.
    fn retriable(
        kind: OpenRouterCalendarFailureKind,
        status_code: Option<u16>,
        retry_after: Option<Duration>,
    ) -> Self {
        // Le mouvement de ces petits champs cree une valeur possedee pour la boucle de retry.
        Self::Retriable(RetriableFailure {
            kind,
            status_code,
            retry_after,
        })
    }
}

/// Metadonnees quantitatives suffisantes pour rejouer une tentative temporaire.
#[derive(Debug)]
struct RetriableFailure {
    // La famille explique la decision sans conserver l'erreur reqwest.
    kind: OpenRouterCalendarFailureKind,
    // Le code HTTP est optionnel car un timeout peut arriver avant toute reponse.
    status_code: Option<u16>,
    // Le delai du fournisseur est deja plafonne au moment de son parsing.
    retry_after: Option<Duration>,
}

/// Parse seulement la forme entiere en secondes de `Retry-After`, puis la plafonne.
fn retry_after_duration(headers: &HeaderMap) -> Option<Duration> {
    // L'absence ou un header non UTF-8 devient `None` et retombe sur le backoff local.
    let seconds = headers
        .get(RETRY_AFTER)?
        .to_str()
        .ok()?
        .parse::<u64>()
        .ok()?;
    // Le plafond ferme empeche le downstream de conserver un permis calendrier trop longtemps.
    Some(Duration::from_secs(seconds).min(MAX_RETRY_BACKOFF))
}

// Les tests restent dans le module pour inspecter l'admission et injecter l'horloge sans API publique.
#[cfg(test)]
mod tests {
    // Les types du parent sont accessibles car un sous-module Rust peut lire ses membres prives.
    use super::*;
    // `VecDeque` permet au mock de fournir une sequence deterministe de reponses.
    use std::collections::VecDeque;
    // Les compteurs atomiques observent les appels concurrents sans mutex sur les valeurs scalaires.
    use std::sync::atomic::{AtomicUsize, Ordering};
    // Le mutex Tokio protege les reponses et les delais sans bloquer un thread async.
    use tokio::sync::Mutex;

    /// Limites courtes et valides reutilisees dans les scenarios unitaires.
    fn test_limits(
        max_response_bytes: usize,
        max_retries: usize,
        max_concurrency: usize,
    ) -> OpenRouterCalendarLimits {
        // Les petites valeurs rendent timeout et backoff rapides tout en passant les bornes production.
        OpenRouterCalendarLimits::new(
            Duration::from_millis(100),
            16_384,
            max_response_bytes,
            max_retries,
            Duration::from_millis(10),
            max_concurrency,
        )
        .expect("les limites du test doivent etre valides")
    }

    /// Construit un client local sans lancer de socket pour les tests de validation et d'admission.
    fn local_client(limits: OpenRouterCalendarLimits) -> OpenRouterCalendarClient {
        // L'URL loopback est explicitement autorisee par le constructeur uniquement pour les tests.
        let endpoint = Url::parse("http://127.0.0.1:9/v1/chat/completions")
            .expect("l'URL statique du test doit etre valide");
        // Le modele illustre le format `provider/model` accepte par le nouveau type.
        let model = OpenRouterCalendarModel::new("test/calendar-model")
            .expect("le modele du test doit etre valide");
        // La configuration regroupe uniquement des valeurs deja validees.
        let config = OpenRouterCalendarClientConfig::new(endpoint, model, limits)
            .expect("la configuration locale doit etre valide");
        // La cle sentinelle sert aussi aux assertions de redaction.
        let api_key = OpenRouterApiKey::new("secret-calendar-test-key")
            .expect("la cle sentinelle doit etre valide");
        // `Client::new` suffit car chaque requete applique son propre timeout borne.
        OpenRouterCalendarClient::new(Client::new(), config, api_key)
    }

    /// Scenario: toutes les representations `Debug` restent exploitables sans secret ni contenu.
    /// Risque couvert: une trace de diagnostic ne doit jamais exposer prompt, reponse ou cle API.
    #[test]
    fn debug_output_is_redacted_by_construction() {
        // Un client complet contient la cle sentinelle qui ne doit jamais apparaitre.
        let client = local_client(test_limits(4_096, 0, 1));
        // Le prompt complet sentinelle facilite la detection d'une fuite de contenu.
        let request =
            OpenRouterCalendarRequest::ingestion("system-secret-prompt\nsource-secret-body")
                .expect("le prompt du test doit etre valide");
        // Une completion locale permet de verifier aussi la redaction de la sortie IA.
        let completion = OpenRouterCalendarCompletion {
            response_model: None,
            content: "native-secret-response".to_owned(),
        };
        // Chaque formatage passe par une implementation manuelle ne contenant que des tailles/metadonnees.
        let debug = format!("{client:?} {request:?} {completion:?}");
        // La cle ne doit jamais apparaitre, meme via l'`Arc` qui l'entoure.
        assert!(!debug.contains("secret-calendar-test-key"));
        // Le system prompt reste masque.
        assert!(!debug.contains("system-secret-prompt"));
        // Le snapshot utilisateur reste masque.
        assert!(!debug.contains("source-secret-body"));
        // La completion brute reste masquee.
        assert!(!debug.contains("native-secret-response"));
        // Le marqueur explicite confirme que le champ cle n'a pas simplement disparu silencieusement.
        assert!(debug.contains("[REDACTED]"));
    }

    /// Scenario: un semaphore sature renvoie `Busy` avant toute tentative reseau.
    /// Risque couvert: le calendrier ne doit jamais mettre le live news en attente dans une queue partagee.
    #[tokio::test]
    async fn saturated_admission_returns_busy_immediately() {
        // Une seule place rend la saturation deterministe.
        let client = local_client(test_limits(4_096, 0, 1));
        // Le test prend directement l'unique permis et le garde jusqu'a l'assertion.
        let _held_permit = client
            .admission
            .clone()
            .try_acquire_owned()
            .expect("le premier permis doit etre disponible");
        // La requete est valide, donc seule l'admission peut provoquer l'erreur observee.
        let request = OpenRouterCalendarRequest::summary("system\nsnapshot")
            .expect("le prompt du test doit etre valide");
        // L'appel ne touche pas le port local ferme car `try_acquire` echoue d'abord.
        let error = client
            .complete_json(&request)
            .await
            .expect_err("la saturation doit refuser le shadow");
        // Le code ferme `Busy` permet a l'orchestrateur d'abandonner sans bloquer le live.
        assert_eq!(error, OpenRouterCalendarError::Busy);
    }

    /// Scenario: les plafonds rejettent une configuration susceptible de monopoliser le runtime.
    /// Risque couvert: une variable d'environnement erronee ne doit pas desactiver les bornes locales.
    #[test]
    fn limits_reject_unbounded_values() {
        // Plus d'une minute de timeout est interdit.
        let timeout_error =
            OpenRouterCalendarLimits::new(Duration::from_secs(61), 1, 1, 0, Duration::ZERO, 1)
                .expect_err("un timeout trop long doit etre refuse");
        // La variante ne contient pas la valeur de configuration brute.
        assert_eq!(
            timeout_error,
            OpenRouterCalendarConfigError::InvalidRequestTimeout
        );
        // Quatre retries depassent le budget ferme de trois retries.
        let retry_error = OpenRouterCalendarLimits::new(
            Duration::from_secs(1),
            1,
            1,
            4,
            Duration::from_millis(1),
            1,
        )
        .expect_err("un budget de retry excessif doit etre refuse");
        // Le code d'erreur reste stable pour la configuration appelante.
        assert_eq!(retry_error, OpenRouterCalendarConfigError::TooManyRetries);
        // Un octet au-dessus du megaoctet decimal depasse aussi la borne du canonicaliseur.
        let response_error = OpenRouterCalendarLimits::new(
            Duration::from_secs(1),
            1,
            1_000_001,
            0,
            Duration::ZERO,
            1,
        )
        .expect_err("une reponse au-dessus du canonicaliseur doit etre refusee");
        // Le code ferme ne recopie pas la taille fautive dans les traces.
        assert_eq!(
            response_error,
            OpenRouterCalendarConfigError::InvalidResponseLimit
        );
        // Neuf appels simultanes depassent l'isolation maximale du calendrier.
        let concurrency_error =
            OpenRouterCalendarLimits::new(Duration::from_secs(1), 1, 1, 0, Duration::ZERO, 9)
                .expect_err("une concurrence excessive doit etre refusee");
        // L'erreur fermee ne divulgue aucune autre configuration.
        assert_eq!(
            concurrency_error,
            OpenRouterCalendarConfigError::InvalidConcurrency
        );
    }

    /// Scenario: seul HTTPS distant ou HTTP loopback peut recevoir la cle Bearer.
    /// Risque couvert: une mauvaise URL ne doit jamais transmettre le secret en clair sur le reseau.
    #[test]
    fn endpoint_policy_rejects_insecure_or_parameterized_urls() {
        // Le modele et les limites sont independants de l'URL testee.
        let model =
            OpenRouterCalendarModel::new("test/model").expect("le modele du test doit etre valide");
        // Une destination distante en HTTP est explicitement interdite.
        let insecure = OpenRouterCalendarClientConfig::new(
            Url::parse("http://example.com/v1/chat/completions")
                .expect("l'URL du test doit etre valide"),
            model.clone(),
            test_limits(4_096, 0, 1),
        )
        .expect_err("HTTP distant doit etre refuse");
        // La cause est precise sans recopier l'URL.
        assert_eq!(insecure, OpenRouterCalendarConfigError::InsecureEndpoint);
        // Une query peut contenir un secret et reste interdite meme sous HTTPS.
        let parameterized = OpenRouterCalendarClientConfig::new(
            Url::parse("https://openrouter.ai/api/v1/chat/completions?token=sentinel")
                .expect("l'URL du test doit etre valide"),
            model,
            test_limits(4_096, 0, 1),
        )
        .expect_err("une query endpoint doit etre refusee");
        // Le code ferme evite d'afficher le parametre fautif.
        assert_eq!(
            parameterized,
            OpenRouterCalendarConfigError::EndpointContainsParameters
        );
    }

    /// Scenario: les visiteurs reduisent les marqueurs comme JavaScript sans conserver leur valeur.
    /// Risque couvert: une coercition Serde non exercee pourrait laisser passer un echec HTTP embarque.
    #[test]
    fn embedded_marker_visitors_match_contract_coercions() {
        // Un objet error est truthy meme vide et doit etre detecte.
        let object_error: OpenRouterChatResponse = serde_json::from_value(serde_json::json!({
            "error": {},
            "choices": []
        }))
        .expect("le marqueur objet doit etre deserialisable");
        // Le visiteur n'a conserve que le verdict booleen.
        assert!(object_error.error_marker.is_truthy);
        // Une chaine error vide est falsy dans la baseline.
        let empty_error: OpenRouterChatResponse = serde_json::from_value(serde_json::json!({
            "error": "",
            "choices": []
        }))
        .expect("le marqueur vide doit etre deserialisable");
        // Le resultat compact reste faux sans stocker la chaine.
        assert!(!empty_error.error_marker.is_truthy);
        // Un tableau singleton se convertit via `Number([500])` et franchit le seuil.
        let singleton_status: OpenRouterChatResponse = serde_json::from_value(serde_json::json!({
            "statusCode": [500],
            "choices": []
        }))
        .expect("le statut singleton doit etre deserialisable");
        // Le marqueur ferme prouve la coercition numerique.
        assert!(singleton_status.status_code_marker.is_failure);
        // Les tableaux singletons imbriques sont eux aussi aplatis par `Array.toString()` en JavaScript.
        let nested_singleton_status: OpenRouterChatResponse =
            serde_json::from_value(serde_json::json!({
                "statusCode": [["500"]],
                "choices": []
            }))
            .expect("le statut singleton imbrique doit etre deserialisable");
        // La recursion bornee reproduit le meme seuil sans conserver le tableau distant.
        assert!(nested_singleton_status.status_code_marker.is_failure);
        // Un booleen dans un tableau devient le texte `true`, donc NaN et non une erreur HTTP.
        let boolean_singleton_status: OpenRouterChatResponse =
            serde_json::from_value(serde_json::json!({
                "statusCode": [true],
                "choices": []
            }))
            .expect("le statut booleen singleton doit etre deserialisable");
        assert!(!boolean_singleton_status.status_code_marker.is_failure);
        // Un tableau vide se convertit en zero et ne doit pas etre marque comme erreur.
        let empty_status: OpenRouterChatResponse = serde_json::from_value(serde_json::json!({
            "status": [],
            "choices": []
        }))
        .expect("le statut tableau vide doit etre deserialisable");
        // La comparaison a 400 reste fausse.
        assert!(!empty_status.status_marker.is_failure);
        // Une chaine numerique suit la coercition JavaScript du contrat historique.
        let string_status: OpenRouterChatResponse = serde_json::from_value(serde_json::json!({
            "status": "503",
            "choices": []
        }))
        .expect("le statut chaine doit etre deserialisable");
        // Le seuil est detecte sans conserver la chaine `503`.
        assert!(string_status.status_marker.is_failure);
    }

    /// Scenario: la taille du prompt est refusee avant de consommer un permis ou d'appeler HTTP.
    /// Risque couvert: un snapshot corrompu ne doit pas creer de pression reseau ou memoire supplementaire.
    #[tokio::test]
    async fn oversized_prompt_is_rejected_before_admission() {
        // La limite de quatre octets est volontairement petite mais valide.
        let limits = OpenRouterCalendarLimits::new(
            Duration::from_millis(100),
            4,
            4_096,
            0,
            Duration::ZERO,
            1,
        )
        .expect("les limites du test doivent etre valides");
        // Le client pointe vers un port ferme, ce qui prouve qu'aucun reseau ne doit etre atteint.
        let client = local_client(limits);
        // La somme des prompts depasse quatre octets.
        let request = OpenRouterCalendarRequest::ingestion("sys-data")
            .expect("le prompt non vide doit etre construit");
        // L'erreur locale est obtenue sans timeout ou erreur de connexion.
        let error = client
            .complete_json(&request)
            .await
            .expect_err("le prompt trop grand doit etre refuse");
        // La cause typee distingue cette erreur d'une saturation.
        assert_eq!(error, OpenRouterCalendarError::PromptTooLarge);
        // Le permis reste disponible puisqu'il n'a jamais ete acquis.
        assert_eq!(client.admission.available_permits(), 1);
    }

    /// Sleeper de test qui enregistre les delais sans attendre le temps reel.
    #[derive(Default)]
    struct RecordingSleeper {
        // Le vecteur asynchrone reste partageable entre appel client et assertions.
        durations: Mutex<Vec<Duration>>,
    }

    // L'implementation substitue seulement l'effet temporel, jamais la politique calculee par le client.
    #[async_trait]
    impl OpenRouterRetrySleeper for RecordingSleeper {
        // La future se termine des que la duree est enregistree.
        async fn sleep(&self, duration: Duration) {
            // Le verrou n'est garde que pendant un push sans aucun autre `.await`.
            self.durations.lock().await.push(duration);
        }
    }

    /// Reponse programmee par le serveur HTTP local.
    struct MockResponse {
        // Le statut permet de tester chaque classe de retry.
        status: StatusCode,
        // Le corps reste une fixture locale et n'est jamais formate par le client.
        body: String,
        // `Retry-After` optionnel teste le plafonnement de l'attente fournisseur.
        retry_after: Option<&'static str>,
    }

    /// Etat partage entre le handler Axum et le scenario appelant.
    struct MockState {
        // La file determine exactement la sequence 5xx/429/succes.
        responses: Mutex<VecDeque<MockResponse>>,
        // L'atomique prouve le nombre de tentatives sans bloquer le handler.
        attempts: AtomicUsize,
        // Les payloads JSON restent confines au mock pour verifier leur parite sans `Debug` production.
        requests: Mutex<Vec<serde_json::Value>>,
    }

    /// Handler minimal qui ne journalise ni header Authorization ni payload entrant.
    async fn mock_openrouter_handler(
        // `State` clone l'`Arc` gere par Axum et preserve un etat unique pour le test.
        axum::extract::State(state): axum::extract::State<Arc<MockState>>,
        // `Bytes` recoit le petit payload deja borne par les fixtures sans introduire de type permissif runtime.
        body: axum::body::Bytes,
    ) -> axum::response::Response {
        // L'ordre relache suffit: le test a seulement besoin d'un compteur final.
        state.attempts.fetch_add(1, Ordering::Relaxed);
        // La capture JSON ne quitte jamais le module de test et ne passe par aucun formatter applicatif.
        let request = serde_json::from_slice(&body)
            .expect("le client doit envoyer un payload OpenRouter JSON valide");
        // Le verrou est relache avant de prendre celui de la file de reponses.
        state.requests.lock().await.push(request);
        // Chaque requete consomme exactement une reponse programmee sous verrou asynchrone.
        let response = state
            .responses
            .lock()
            .await
            .pop_front()
            .expect("le mock doit disposer d'une reponse par tentative");
        // Le builder Axum cree une reponse HTTP sans decoder ni afficher le prompt recu.
        let mut builder = axum::http::Response::builder().status(response.status);
        // Le header est ajoute uniquement lorsque le scenario veut tester sa priorite.
        if let Some(retry_after) = response.retry_after {
            // La valeur statique est valide et ne contient aucune donnee sensible.
            builder = builder.header(RETRY_AFTER, retry_after);
        }
        // Le body du mock est deplace dans la reponse et n'est pas clone.
        builder
            .body(axum::body::Body::from(response.body))
            .expect("la reponse HTTP du mock doit etre valide")
    }

    /// Lance un serveur loopback ephemere et retourne son URL ainsi que son etat d'observation.
    async fn spawn_mock_server(
        responses: Vec<MockResponse>,
    ) -> (Url, Arc<MockState>, tokio::task::JoinHandle<()>) {
        // Le port zero demande au systeme un port libre et evite les collisions entre tests.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("le serveur mock doit ecouter sur loopback");
        // L'adresse effective est lue avant de deplacer le listener dans la tache serveur.
        let address = listener
            .local_addr()
            .expect("l'adresse du mock doit etre disponible");
        // La file et le compteur vivent derriere un `Arc` clone par Axum et le test.
        let state = Arc::new(MockState {
            responses: Mutex::new(responses.into()),
            attempts: AtomicUsize::new(0),
            requests: Mutex::new(Vec::new()),
        });
        // Une seule route POST suffit au contrat chat/completions.
        let router = axum::Router::new()
            .route(
                "/v1/chat/completions",
                axum::routing::post(mock_openrouter_handler),
            )
            .with_state(state.clone());
        // La tache est possedee par le test et sera avortee explicitement apres les assertions.
        let server = tokio::spawn(async move {
            // Une fermeture anticipee lors de `abort` est normale et ne doit pas paniquer.
            let _result = axum::serve(listener, router).await;
        });
        // L'URL est construite depuis l'adresse locale non sensible.
        let endpoint = Url::parse(&format!("http://{address}/v1/chat/completions"))
            .expect("l'URL du mock doit etre valide");
        // Le tuple rend au scenario tous les handles necessaires au controle de cycle de vie.
        (endpoint, state, server)
    }

    /// Handler volontairement lent qui permet d'exercer le timeout total de chaque tentative.
    async fn slow_openrouter_handler(
        // Le compteur atomique est le seul etat partage; aucun payload ou header entrant n'est lu.
        axum::extract::State(attempts): axum::extract::State<Arc<AtomicUsize>>,
    ) -> axum::response::Response {
        // L'increment intervient avant l'attente afin de compter meme une connexion annulee par le client.
        attempts.fetch_add(1, Ordering::Relaxed);
        // Le timer Tokio libere le thread pendant une duree largement superieure au timeout du test.
        tokio::time::sleep(Duration::from_millis(250)).await;
        // La reponse serait valide si le client ne l'avait pas deja abandonnee.
        axum::http::Response::builder()
            .status(StatusCode::OK)
            .body(axum::body::Body::from(success_response()))
            .expect("la reponse lente du mock doit etre valide")
    }

    /// Lance le serveur lent sur loopback avec un compteur partage d'appels.
    async fn spawn_slow_mock_server() -> (Url, Arc<AtomicUsize>, tokio::task::JoinHandle<()>) {
        // Le port zero evite toute collision avec les autres mocks executes en parallele.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("le serveur lent doit ecouter sur loopback");
        // L'adresse est lue avant de deplacer le listener dans sa tache longue duree.
        let address = listener
            .local_addr()
            .expect("l'adresse du serveur lent doit etre disponible");
        // Le compteur est clone par Axum et reste observable depuis le scenario.
        let attempts = Arc::new(AtomicUsize::new(0));
        // Une route identique a OpenRouter suffit; le handler ne lit aucune donnee sensible.
        let router = axum::Router::new()
            .route(
                "/v1/chat/completions",
                axum::routing::post(slow_openrouter_handler),
            )
            .with_state(attempts.clone());
        // Le serveur tourne dans une tache possedee et explicitement avortee a la fin du test.
        let server = tokio::spawn(async move {
            let _result = axum::serve(listener, router).await;
        });
        // HTTP est autorise parce que l'adresse est une boucle locale de test.
        let endpoint = Url::parse(&format!("http://{address}/v1/chat/completions"))
            .expect("l'URL du serveur lent doit etre valide");
        (endpoint, attempts, server)
    }

    /// Construit un client connecte au mock et injecte le sleeper sans attente.
    fn mock_client(
        endpoint: Url,
        limits: OpenRouterCalendarLimits,
        sleeper: Arc<RecordingSleeper>,
    ) -> OpenRouterCalendarClient {
        // Le modele valide apparait dans la requete et dans la reponse fixture.
        let model = OpenRouterCalendarModel::new("test/calendar-model")
            .expect("le modele du mock doit etre valide");
        // HTTP loopback est accepte uniquement parce que l'hote est local.
        let config = OpenRouterCalendarClientConfig::new(endpoint, model, limits)
            .expect("la configuration du mock doit etre valide");
        // La cle reste privee meme si le serveur ne la valide pas dans ces tests de politique.
        let api_key =
            OpenRouterApiKey::new("mock-secret-key").expect("la cle du mock doit etre valide");
        // Le sleeper enregistre les pauses tout en gardant le test rapide.
        OpenRouterCalendarClient::new(Client::new(), config, api_key).with_retry_sleeper(sleeper)
    }

    /// Produit une enveloppe OpenRouter typee dont le contenu est un objet JSON calendrier minimal.
    fn success_response() -> String {
        // La chaine externe est une fixture statique et son contenu interne reste valide apres deserialisation.
        r#"{"model":"test/calendar-model","choices":[{"finish_reason":"stop","message":{"content":"{\"items\":[]}"}}]}"#
            .to_owned()
    }

    #[tokio::test]
    async fn incomplete_ingestion_completions_are_terminal_and_redacted() {
        for (value, expected) in [
            (serde_json::json!("length"), CalendarFinishReason::Length),
            (
                serde_json::json!("content_filter"),
                CalendarFinishReason::ContentFilter,
            ),
            (
                serde_json::json!("tool_calls"),
                CalendarFinishReason::ToolCalls,
            ),
            (serde_json::json!("error"), CalendarFinishReason::Error),
            (
                serde_json::json!("private-provider-detail"),
                CalendarFinishReason::Unknown,
            ),
            (serde_json::Value::Null, CalendarFinishReason::Missing),
        ] {
            let body=serde_json::json!({"choices":[{"finish_reason":value,"message":{"content":"private-model-content"}}]}).to_string();
            let (endpoint, state, server) = spawn_mock_server(vec![MockResponse {
                status: StatusCode::OK,
                body,
                retry_after: None,
            }])
            .await;
            let client = mock_client(
                endpoint,
                test_limits(4096, 2, 1),
                Arc::new(RecordingSleeper::default()),
            );
            let error = client
                .complete_json(&OpenRouterCalendarRequest::ingestion("test").unwrap())
                .await
                .unwrap_err();
            assert_eq!(
                error,
                OpenRouterCalendarError::IncompleteCompletion { reason: expected }
            );
            assert!(!format!("{error:?}").contains("private"));
            assert_eq!(state.requests.lock().await.len(), 1);
            server.abort();
        }
    }

    #[tokio::test]
    async fn unsupported_schema_is_terminal_without_unstructured_fallback() {
        let (endpoint, state, server) = spawn_mock_server(vec![MockResponse {
            status: StatusCode::BAD_REQUEST,
            body: "private schema failure".to_owned(),
            retry_after: None,
        }])
        .await;
        let client = mock_client(
            endpoint,
            test_limits(4096, 2, 1),
            Arc::new(RecordingSleeper::default()),
        );
        assert_eq!(
            client
                .complete_json(&OpenRouterCalendarRequest::ingestion("test").unwrap())
                .await
                .unwrap_err(),
            OpenRouterCalendarError::HttpStatus { status_code: 400 }
        );
        let requests = state.requests.lock().await;
        assert_eq!(requests.len(), 1);
        assert_eq!(
            requests[0]["response_format"]["json_schema"]["strict"],
            true
        );
        assert_eq!(requests[0]["provider"]["require_parameters"], true);
        server.abort();
    }

    /// Produit la forme segmentee que le parseur du contrat joint par retours ligne.
    fn segmented_success_response() -> String {
        // `json!` construit une fixture lisible dont les segments utilisent les deux alias acceptes.
        serde_json::json!({
            "model": "test/calendar-model",
            "choices": [{
                "finish_reason": "stop",
                "message": {
                    "content": [
                        { "text": "{\"items\":" },
                        { "text": "", "content": "[]}" }
                    ]
                }
            }]
        })
        // La serialisation de cette valeur locale ne peut echouer et produit le corps du mock.
        .to_string()
    }

    /// Produit une reponse dont `content` est directement un objet JSON.
    fn object_success_response(content: serde_json::Value) -> String {
        // La fixture insere la valeur sans la convertir en chaine pour exercer la variante `Object`.
        serde_json::json!({
            "model": "test/calendar-model",
            "choices": [{ "finish_reason": "stop", "message": { "content": content } }]
        })
        // `Value` garantit une serialisation JSON valide pour le serveur local.
        .to_string()
    }

    /// Produit un tableau de segments destine au profil summary, qui ne doit pas les joindre.
    fn summary_parts_response() -> String {
        // Les champs additionnels prouvent que la reserialisation conserve le tableau recu.
        serde_json::json!({
            "model": "test/calendar-model",
            "choices": [{
                "message": {
                    "content": [
                        { "type": "text", "text": "premiere partie" },
                        { "content": "seconde partie" }
                    ]
                }
            }]
        })
        // Le corps compact reste sous la limite du mock.
        .to_string()
    }

    /// Construit les enveloppes que le profil summary transforme toutes en textes fallback.
    fn summary_fallback_response_bodies() -> Vec<String> {
        // Chaque fixture garde un modele valide afin d'isoler uniquement la forme de `choices/message/content`.
        vec![
            // Une liste de choix absente correspond a `response?.choices?.[0] === undefined`.
            serde_json::json!({ "model": "test/calendar-model" }).to_string(),
            // Un message sans champ content exerce l'acces optionnel jusqu'au dernier niveau.
            serde_json::json!({
                "model": "test/calendar-model",
                "choices": [{ "message": {} }]
            })
            .to_string(),
            // JSON `null` devient le meme `None` Rust qu'un champ absent.
            serde_json::json!({
                "model": "test/calendar-model",
                "choices": [{ "message": { "content": null } }]
            })
            .to_string(),
            // Une chaine vide est falsy dans `parseMaybeJson`.
            serde_json::json!({
                "model": "test/calendar-model",
                "choices": [{ "message": { "content": "" } }]
            })
            .to_string(),
            // Un nombre n'expose aucun champ summary et doit donc declencher les valeurs fallback.
            serde_json::json!({
                "model": "test/calendar-model",
                "choices": [{ "message": { "content": 42 } }]
            })
            .to_string(),
            // Un booleen suit la meme regle sans etre converti en texte visible.
            serde_json::json!({
                "model": "test/calendar-model",
                "choices": [{ "message": { "content": false } }]
            })
            .to_string(),
        ]
    }

    /// Construit les marqueurs embarques que le contrat traite comme erreurs malgre un HTTP 200.
    fn embedded_failure_response_bodies() -> Vec<String> {
        // Les sentinelles prouvent ensuite que le visiteur les draine sans les conserver dans l'erreur Rust.
        vec![
            // Un objet `error` est truthy en JavaScript, meme si seul son message explique la panne.
            serde_json::json!({
                "error": { "message": "provider-object-secret-sentinel" },
                "choices": [{ "message": { "content": null } }]
            })
            .to_string(),
            // Une chaine error non vide suit le meme chemin sans devoir etre parse comme objet.
            serde_json::json!({
                "error": "provider-string-secret-sentinel",
                "choices": [{ "message": { "content": "{}" } }]
            })
            .to_string(),
            // `statusCode` numerique reproduit la sortie camelCase du contrat HTTP.
            serde_json::json!({
                "statusCode": 401,
                "choices": [{ "message": { "content": "status-code-body-secret" } }]
            })
            .to_string(),
            // Le nom court accepte aussi une chaine numerique par coercition JavaScript.
            serde_json::json!({
                "status": "503",
                "choices": [{ "message": { "content": "status-body-secret" } }]
            })
            .to_string(),
            // JavaScript convertit un tableau singleton `[500]` vers le nombre 500.
            serde_json::json!({
                "statusCode": [500],
                "choices": [{ "message": { "content": "array-status-body-secret" } }]
            })
            .to_string(),
        ]
    }

    /// Scenario HTTP: ingestion et summary envoient exactement les profils natifs fermes.
    /// Le schema strict est reserve a l'ingestion ; le payload summary reste identique.
    #[tokio::test]
    async fn request_profiles_serialize_exact_native_payloads() {
        // Deux succes permettent d'observer successivement les profils ingestion et summary.
        let (endpoint, state, server) = spawn_mock_server(vec![
            MockResponse {
                status: StatusCode::OK,
                body: success_response(),
                retry_after: None,
            },
            MockResponse {
                status: StatusCode::OK,
                body: success_response(),
                retry_after: None,
            },
        ])
        .await;
        // Aucun retry n'est necessaire pour cette assertion de serialisation.
        let sleeper = Arc::new(RecordingSleeper::default());
        // Le client sequentiel conserve l'ordre exact des deux captures.
        let client = mock_client(endpoint, test_limits(4_096, 0, 1), sleeper);
        // Le premier prompt est deja joint, comme la chaine produite par `Array.join('\n')` dans le contrat historique.
        let ingestion = OpenRouterCalendarRequest::ingestion("prompt ingestion complet")
            .expect("le profil ingestion doit etre valide");
        // Le profil summary transporte lui aussi un unique texte complet.
        let summary = OpenRouterCalendarRequest::summary("prompt summary complet")
            .expect("le profil summary doit etre valide");
        // Chaque appel doit aboutir avant l'observation du payload capture.
        client
            .complete_json(&ingestion)
            .await
            .expect("le mock ingestion doit reussir");
        // Le second appel exerce l'omission Serde de `max_tokens`.
        client
            .complete_json(&summary)
            .await
            .expect("le mock summary doit reussir");
        // Le verrou emprunte le vecteur seulement pendant les assertions synchrones suivantes.
        let requests = state.requests.lock().await;
        // L'ingestion contient exactement modele, message user unique, temperature et budget fixe.
        assert_eq!(
            requests.first(),
            Some(&serde_json::json!({
                "model": "test/calendar-model",
                "messages": [{
                    "role": "user",
                    "content": "prompt ingestion complet"
                }],
                "temperature": 0.1,
                "max_tokens": 24_000,
                "response_format": ingestion_response_format(),
                "provider": {"require_parameters":true}
            }))
        );
        // Le summary omet `max_tokens`; l'egalite exacte prouve aussi l'absence de `response_format`.
        assert_eq!(
            requests.get(1),
            Some(&serde_json::json!({
                "model": "test/calendar-model",
                "messages": [{
                    "role": "user",
                    "content": "prompt summary complet"
                }],
                "temperature": 0.1
            }))
        );
        // Relacher explicitement l'emprunt facilite la lecture du cycle de vie avant l'arret serveur.
        drop(requests);
        // Le serveur ephemere est abandonne apres les deux preuves.
        server.abort();
    }

    #[tokio::test]
    async fn dots_uses_strict_json_schema_without_model_or_schema_fallback() {
        let (endpoint, state, server) = spawn_mock_server(vec![
            MockResponse {
                status: StatusCode::OK,
                body: success_response(),
                retry_after: None,
            },
            MockResponse {
                status: StatusCode::BAD_REQUEST,
                body: r#"{"error":{"code":400,"message":"fixture rejection"}}"#.to_owned(),
                retry_after: None,
            },
        ])
        .await;
        let mut client = mock_client(
            endpoint.clone(),
            test_limits(4096, 2, 1),
            Arc::new(RecordingSleeper::default()),
        );
        client.config = Arc::new(
            OpenRouterCalendarClientConfig::new(
                endpoint,
                OpenRouterCalendarModel::new("dots-studio/dots-3-note-preview:free").unwrap(),
                test_limits(4096, 2, 1),
            )
            .unwrap(),
        );
        let request = OpenRouterCalendarRequest::ingestion("JSON items fixture").unwrap();
        client.complete_json(&request).await.unwrap();
        assert_eq!(
            client.complete_json(&request).await.unwrap_err(),
            OpenRouterCalendarError::HttpStatus { status_code: 400 }
        );
        let requests = state.requests.lock().await;
        assert_eq!(
            requests.len(),
            2,
            "no fallback or retry on a permanent rejection"
        );
        for request in requests.iter() {
            assert_eq!(request["model"], "dots-studio/dots-3-note-preview:free");
            assert_eq!(request["response_format"], ingestion_response_format());
            assert_eq!(request["response_format"]["type"], "json_schema");
            assert_eq!(request["response_format"]["json_schema"]["strict"], true);
            assert_eq!(
                request["provider"],
                serde_json::json!({"require_parameters":true})
            );
            assert_eq!(request["max_tokens"], 24000);
        }
        drop(requests);
        server.abort();
    }

    /// Scenario HTTP: un tableau de segments devient le meme texte joint que dans l'ingestion native.
    /// Risque couvert: un fournisseur qui segmente son contenu ne doit pas creer une fausse divergence.
    #[tokio::test]
    async fn segmented_content_is_joined_like_native_ingestion() {
        // La reponse contient `text`, puis un `text` vide qui retombe sur l'alias `content`.
        let (endpoint, state, server) = spawn_mock_server(vec![MockResponse {
            status: StatusCode::OK,
            body: segmented_success_response(),
            retry_after: None,
        }])
        .await;
        // Le scenario nominal ne consomme aucun retry ni pause.
        let sleeper = Arc::new(RecordingSleeper::default());
        // Un seul permis suffit a cette completion.
        let client = mock_client(endpoint, test_limits(4_096, 0, 1), sleeper);
        // Le profil ingestion est pertinent car son parseur accepte explicitement les segments.
        let request = OpenRouterCalendarRequest::ingestion("prompt segments")
            .expect("le prompt segmente doit etre valide");
        // La completion possedee contient la jointure sans exposer son texte via `Debug`.
        let completion = client
            .complete_json(&request)
            .await
            .expect("le contenu segmente doit etre accepte");
        // Le retour ligne entre segments est un espace JSON valide et reproduit `.join('\n')`.
        assert_eq!(completion.content(), "{\"items\":\n[]}");
        // Une seule requete prouve qu'aucun retry n'a masque une erreur de parsing.
        assert_eq!(state.attempts.load(Ordering::Relaxed), 1);
        // Le serveur local est arrete apres l'assertion.
        server.abort();
    }

    /// Scenario HTTP: les objets directs sont serialises, mais un tableau summary reste un tableau.
    /// Risque couvert: partager une normalisation ingestion/summary masquerait le fallback reel du profil summary.
    #[tokio::test]
    async fn content_normalization_depends_on_closed_request_profile() {
        // Les trois reponses couvrent objet ingestion, objet summary et segments summary dans cet ordre.
        let (endpoint, state, server) = spawn_mock_server(vec![
            MockResponse {
                status: StatusCode::OK,
                body: object_success_response(serde_json::json!({ "items": [] })),
                retry_after: None,
            },
            MockResponse {
                status: StatusCode::OK,
                body: object_success_response(serde_json::json!({
                    "summaryText": "Resume fixture"
                })),
                retry_after: None,
            },
            MockResponse {
                status: StatusCode::OK,
                body: summary_parts_response(),
                retry_after: None,
            },
        ])
        .await;
        // Aucun retry ne doit intervenir dans ce scenario de conversion de contrat.
        let sleeper = Arc::new(RecordingSleeper::default());
        // La concurrence un conserve l'ordre des trois reponses programmees.
        let client = mock_client(endpoint, test_limits(4_096, 0, 1), sleeper);
        // L'objet ingestion direct doit devenir le texte JSON attendu par son canonicaliseur.
        let ingestion = OpenRouterCalendarRequest::ingestion("prompt objet ingestion")
            .expect("le profil ingestion doit etre valide");
        // Les deux appels summary partagent le meme profil ferme, quelle que soit la forme retournee.
        let summary = OpenRouterCalendarRequest::summary("prompt objet summary")
            .expect("le profil summary doit etre valide");
        // La premiere completion serialise l'objet sans enveloppe supplementaire.
        let ingestion_completion = client
            .complete_json(&ingestion)
            .await
            .expect("l'objet ingestion doit etre accepte");
        // La seconde completion applique la meme serialisation a un objet summary direct.
        let summary_object_completion = client
            .complete_json(&summary)
            .await
            .expect("l'objet summary doit etre accepte");
        // La troisieme completion conserve la nature tableau au lieu de joindre les segments.
        let summary_parts_completion = client
            .complete_json(&summary)
            .await
            .expect("le tableau summary doit etre preserve");
        // Le contenu objet ingestion reste un objet racine utilisable par le contrat calendrier.
        assert_eq!(ingestion_completion.content(), r#"{"items":[]}"#);
        // Le contenu objet summary reste lui aussi un objet JSON compact.
        assert_eq!(
            summary_object_completion.content(),
            r#"{"summaryText":"Resume fixture"}"#
        );
        // Parser le texte de sortie prouve la preservation exacte de la forme et des champs du tableau.
        let summary_parts: serde_json::Value =
            serde_json::from_str(summary_parts_completion.content())
                .expect("la completion summary segmentee doit rester du JSON");
        // L'egalite structurelle evite de rendre le test dependant de l'ordre des cles JSON.
        assert_eq!(
            summary_parts,
            serde_json::json!([
                { "type": "text", "text": "premiere partie" },
                { "content": "seconde partie" }
            ])
        );
        // Trois tentatives exactes prouvent qu'aucune conversion n'a declenche un retry cache.
        assert_eq!(state.attempts.load(Ordering::Relaxed), 3);
        // Le serveur local est arrete apres toutes les preuves de profil.
        server.abort();
    }

    /// Scenario HTTP: les valeurs falsy/scalaires activent summary fallback mais restent invalides en ingestion.
    /// Risque couvert: une normalisation commune ferait diverger le statut Ready historique du resume.
    #[tokio::test]
    async fn summary_fallback_inputs_are_profile_aware_and_redacted() {
        // Le meme ensemble est joue une fois en summary, puis une fois en ingestion pour comparer les profils.
        let bodies = summary_fallback_response_bodies();
        // Le vecteur possede exactement deux reponses par forme, sans file implicite supplementaire.
        let mut responses = Vec::with_capacity(bodies.len().saturating_mul(2));
        // La premiere moitie alimente les appels summary dans l'ordre documente par le helper.
        for body in &bodies {
            responses.push(MockResponse {
                status: StatusCode::OK,
                body: body.clone(),
                retry_after: None,
            });
        }
        // La seconde moitie rejoue les memes enveloppes contre le profil ingestion strict.
        for body in &bodies {
            responses.push(MockResponse {
                status: StatusCode::OK,
                body: body.clone(),
                retry_after: None,
            });
        }
        // Le mock partage un compteur qui prouvera l'absence de retries caches.
        let (endpoint, state, server) = spawn_mock_server(responses).await;
        // Aucun backoff ne doit intervenir car toutes les reponses HTTP sont des succes permanents.
        let sleeper = Arc::new(RecordingSleeper::default());
        // La reponse maximale couvre largement ces petites enveloppes typees.
        let client = mock_client(endpoint, test_limits(4_096, 0, 1), sleeper);
        // Le prompt sentinelle ne doit jamais reapparaitre dans une erreur ingestion.
        let summary = OpenRouterCalendarRequest::summary("summary-fallback-prompt-secret")
            .expect("le profil summary fallback doit etre valide");
        // L'ingestion utilise un profil distinct malgre un montage HTTP identique.
        let ingestion = OpenRouterCalendarRequest::ingestion("ingestion-strict-prompt-secret")
            .expect("le profil ingestion strict doit etre valide");
        // Chaque forme summary devient l'objet vide compris par le canonicaliseur de fallback.
        for _index in 0..bodies.len() {
            let completion = client
                .complete_json(&summary)
                .await
                .expect("summary doit accepter toute forme fallback");
            // Le contenu compact ne conserve ni scalaire ni valeur distante inutile.
            assert_eq!(completion.content(), "{}");
        }
        // Les memes formes ne satisfont jamais le contrat `{items}` de l'ingestion.
        for _index in 0..bodies.len() {
            let error = client
                .complete_json(&ingestion)
                .await
                .expect_err("ingestion doit refuser une forme sans payload items");
            // Selon la forme, l'erreur distingue contenu vide et forme invalide, toutes deux sans valeur brute.
            assert!(matches!(
                error,
                OpenRouterCalendarError::InvalidResponse { .. }
                    | OpenRouterCalendarError::EmptyCompletion
                    | OpenRouterCalendarError::IncompleteCompletion {
                        reason: CalendarFinishReason::Missing
                    }
            ));
            // Le formatter de l'erreur fermee ne peut recopier le prompt de la requete.
            assert!(!format!("{error:?}").contains("prompt-secret"));
        }
        // Deux profils fois six formes donnent douze requetes exactes, sans retry.
        assert_eq!(state.attempts.load(Ordering::Relaxed), bodies.len() * 2);
        // Le serveur ephemere est libere apres les preuves profilees.
        server.abort();
    }

    /// Scenario HTTP: les marqueurs d'erreur embarques priment sur les fallbacks des deux profils.
    /// Risque couvert: un HTTP 200 contenant une erreur fournisseur ne doit jamais devenir un summary Ready.
    #[tokio::test]
    async fn embedded_error_and_status_markers_are_rejected_and_redacted() {
        // Les quatre marqueurs sont joues pour summary puis ingestion afin de prouver une politique commune.
        let bodies = embedded_failure_response_bodies();
        // Deux profils multiplient exactement le nombre de reponses programmees.
        let mut responses = Vec::with_capacity(bodies.len().saturating_mul(2));
        // La premiere passe cible le chemin summary, ou le fallback `{}` serait sinon actif.
        for body in &bodies {
            responses.push(MockResponse {
                status: StatusCode::OK,
                body: body.clone(),
                retry_after: None,
            });
        }
        // La seconde passe confirme la meme rejection pour l'ingestion stricte.
        for body in &bodies {
            responses.push(MockResponse {
                status: StatusCode::OK,
                body: body.clone(),
                retry_after: None,
            });
        }
        // Le statut HTTP du mock reste toujours 200; seule l'enveloppe typee peut provoquer l'erreur.
        let (endpoint, state, server) = spawn_mock_server(responses).await;
        // Aucun retry n'est configure car ces marqueurs representent un echec permanent du payload.
        let sleeper = Arc::new(RecordingSleeper::default());
        // La limite de corps couvre toutes les petites sentinelles sans declencher la borne de taille.
        let client = mock_client(endpoint, test_limits(4_096, 0, 1), sleeper);
        // Les prompts possedent eux aussi une sentinelle qui ne doit pas contaminer l'erreur.
        let summary = OpenRouterCalendarRequest::summary("summary-marker-prompt-secret")
            .expect("le profil summary marqueur doit etre valide");
        // L'ingestion reutilise les memes enveloppes apres la premiere passe.
        let ingestion = OpenRouterCalendarRequest::ingestion("ingestion-marker-prompt-secret")
            .expect("le profil ingestion marqueur doit etre valide");
        // Chaque marqueur doit interrompre summary avant sa normalisation de contenu absent.
        for _index in 0..bodies.len() {
            let error = client
                .complete_json(&summary)
                .await
                .expect_err("summary doit rejeter le marqueur embarque");
            // Le contrat expose seulement une famille statique, jamais le statut ou message recu.
            assert_eq!(
                error,
                OpenRouterCalendarError::InvalidResponse {
                    kind: OpenRouterResponseFailureKind::EmbeddedError,
                }
            );
            // Ni le prompt ni une sentinelle fournisseur ne peut apparaitre via `Debug`.
            let debug = format!("{error:?}");
            assert!(!debug.contains("secret"));
            assert!(!debug.contains("sentinel"));
        }
        // L'ingestion applique exactement la meme priorite des marqueurs sur son contenu.
        for _index in 0..bodies.len() {
            let error = client
                .complete_json(&ingestion)
                .await
                .expect_err("ingestion doit rejeter le marqueur embarque");
            // Une erreur fermee identique evite de conserver la forme distante dans le rapport.
            assert_eq!(
                error,
                OpenRouterCalendarError::InvalidResponse {
                    kind: OpenRouterResponseFailureKind::EmbeddedError,
                }
            );
            // Le formatter reste constant pour la seconde famille de prompt.
            assert!(!format!("{error:?}").contains("secret"));
        }
        // Huit HTTP 200 exacts prouvent qu'aucun retry n'a masque la classification interne.
        assert_eq!(state.attempts.load(Ordering::Relaxed), bodies.len() * 2);
        // Le serveur local est arrete apres la verification des deux profils.
        server.abort();
    }

    #[tokio::test]
    async fn response_rejection_stages_are_distinct_redacted_and_never_retried() {
        use OpenRouterResponseFailureKind as K;
        let cases = [
            ("{invalid-secret-sentinel".to_owned(), K::Json),
            (r#"{"choices":"secret-sentinel"}"#.to_owned(), K::Envelope),
            (r#"{"model":""}"#.to_owned(), K::Model),
            (
                serde_json::json!({"choices":[{"finish_reason":"stop","message":{"content":42}}]})
                    .to_string(),
                K::Content,
            ),
            (
                serde_json::json!({"error":{"message":"provider-secret-sentinel"}}).to_string(),
                K::EmbeddedError,
            ),
        ];
        let responses = cases
            .iter()
            .map(|(body, _)| MockResponse {
                status: StatusCode::OK,
                body: body.clone(),
                retry_after: None,
            })
            .collect();
        let (endpoint, state, server) = spawn_mock_server(responses).await;
        // Un budget disponible ne doit jamais transformer ces rejets permanents en retries.
        let client = mock_client(
            endpoint,
            test_limits(4_096, 2, 1),
            Arc::new(RecordingSleeper::default()),
        );
        let request = OpenRouterCalendarRequest::ingestion("prompt-secret-sentinel").unwrap();
        for (_, kind) in &cases {
            let error = client.complete_json(&request).await.unwrap_err();
            assert_eq!(
                error,
                OpenRouterCalendarError::InvalidResponse { kind: *kind }
            );
            let diagnostic =
                super::super::diagnostics::CalendarIngestionDiagnostic::openrouter(error);
            let encoded = serde_json::to_string(&diagnostic).unwrap();
            assert!(!encoded.contains("secret"));
            assert!(!format!("{error:?}").contains("sentinel"));
        }
        assert_eq!(state.attempts.load(Ordering::Relaxed), cases.len());
        server.abort();
    }

    #[tokio::test]
    async fn http_and_embedded_errors_log_numeric_code_message_and_body_only_on_failure() {
        // Le registre global des callsites tracing est partage par les tests paralleles.
        // Executer ce test de rendu dans son propre processus conserve le vrai subscriber
        // et toutes ses assertions, sans imposer de serialiser la suite Rust entiere.
        const CHILD: &str = "FILFINANCE_TEST_OPENROUTER_LOG_CHILD";
        if std::env::var_os(CHILD).is_none() {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .arg("features::calendar_day::native::openrouter::tests::http_and_embedded_errors_log_numeric_code_message_and_body_only_on_failure")
                .arg("--exact")
                .env(CHILD, "1")
                .output().unwrap();
            let stdout = String::from_utf8_lossy(&output.stdout);
            assert!(
                output.status.success(),
                "isolated logger test failed: {stdout} {}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(
                stdout.contains("1 passed; 0 failed"),
                "the isolated test must actually execute"
            );
            return;
        }
        use std::io::{self, Write};
        use tracing::instrument::WithSubscriber;
        #[derive(Clone)]
        struct Sink(Arc<std::sync::Mutex<Vec<u8>>>);
        impl Write for Sink {
            fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
                self.0.lock().unwrap().extend_from_slice(bytes);
                Ok(bytes.len())
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        let statuses = [
            StatusCode::OK,
            StatusCode::BAD_REQUEST,
            StatusCode::TOO_MANY_REQUESTS,
            StatusCode::BAD_GATEWAY,
        ];
        let mut responses = statuses.iter().map(|status| MockResponse {
            status: *status,
            body: serde_json::json!({"error":{"code":502,"message":"Upstream fixture rejected\nforged log","metadata":{"raw":"explanation retained"}},"authorization":"Bearer never-log-this"}).to_string(),
            retry_after: None,
        }).collect::<Vec<_>>();
        responses.push(MockResponse {
            status: StatusCode::BAD_REQUEST,
            body: "oversized-error-body".repeat(1_000),
            retry_after: None,
        });
        responses.push(MockResponse {
            status: StatusCode::OK,
            body: serde_json::json!({"choices":[{"finish_reason":"stop","message":{"content":"{\"items\":[]}"}}]}).to_string(),
            retry_after: None,
        });
        let (endpoint, state, server) = spawn_mock_server(responses).await;
        let client = mock_client(
            endpoint,
            test_limits(4096, 0, 1),
            Arc::new(RecordingSleeper::default()),
        );
        let request = OpenRouterCalendarRequest::ingestion("private-prompt").unwrap();
        let sink = Sink(Arc::default());
        let captured = sink.clone();
        let subscriber = tracing_subscriber::fmt()
            .without_time()
            .with_ansi(false)
            .with_writer(move || captured.clone())
            .finish();
        async {
            for status in statuses {
                let error = client.complete_json(&request).await.unwrap_err();
                match status {
                    StatusCode::OK => assert!(matches!(
                        error,
                        OpenRouterCalendarError::InvalidResponse {
                            kind: OpenRouterResponseFailureKind::EmbeddedError
                        }
                    )),
                    StatusCode::BAD_REQUEST => assert!(matches!(
                        error,
                        OpenRouterCalendarError::HttpStatus { status_code: 400 }
                    )),
                    _ => assert!(matches!(
                        error,
                        OpenRouterCalendarError::RetriesExhausted { attempts: 1, .. }
                    )),
                }
            }
            // Le logger respecte le plafond configure sans transformer ce 400 en retry.
            assert!(matches!(
                client.complete_json(&request).await.unwrap_err(),
                OpenRouterCalendarError::HttpStatus { status_code: 400 }
            ));
            client.complete_json(&request).await.unwrap();
        }
        .with_subscriber(subscriber)
        .await;
        let output = String::from_utf8(sink.0.lock().unwrap().clone()).unwrap();
        assert_eq!(
            output.matches("calendar_openrouter_error_response").count(),
            5
        );
        assert_eq!(output.matches("provider_code=Some(502)").count(), 4);
        assert_eq!(
            output.lines().count(),
            5,
            "provider newlines must be escaped"
        );
        assert!(output.contains("truncated=true"));
        assert!(output.contains("captured_bytes=4096"));
        assert!(output.contains("Upstream fixture rejected"));
        assert!(output.contains("explanation retained"));
        assert!(!output.contains("never-log-this"));
        assert!(!output.contains("private-prompt"));
        assert!(
            !output.contains("items"),
            "successful completions must not be logged"
        );
        assert_eq!(state.attempts.load(Ordering::Relaxed), 6);
        server.abort();
    }

    /// Scenario HTTP: un statut tableau vide vaut zero et ne doit pas creer une fausse erreur.
    /// Risque couvert: une coercition trop stricte divergerait de `Number([]) === 0` dans le processus historique.
    #[tokio::test]
    async fn empty_array_status_marker_is_not_a_failure() {
        // Les deux alias de statut sont testes separement sur ingestion puis summary.
        let (endpoint, state, server) = spawn_mock_server(vec![
            MockResponse {
                status: StatusCode::OK,
                body: serde_json::json!({
                    "model": "test/calendar-model",
                    "statusCode": [],
                    "choices": [{ "finish_reason":"stop", "message": { "content": "{\"items\":[]}" } }]
                })
                .to_string(),
                retry_after: None,
            },
            MockResponse {
                status: StatusCode::OK,
                body: serde_json::json!({
                    "model": "test/calendar-model",
                    "status": [],
                    "choices": [{ "message": { "content": "{}" } }]
                })
                .to_string(),
                retry_after: None,
            },
        ])
        .await;
        // Aucun retry n'est necessaire puisque les deux marqueurs valent zero apres coercition.
        let sleeper = Arc::new(RecordingSleeper::default());
        // Le client sequentiel associe chaque alias au profil attendu.
        let client = mock_client(endpoint, test_limits(4_096, 0, 1), sleeper);
        // Le profil ingestion doit recevoir normalement son contenu items.
        let ingestion = OpenRouterCalendarRequest::ingestion("empty-array-status-ingestion")
            .expect("le prompt ingestion doit etre valide");
        // Le profil summary doit lui aussi atteindre sa completion.
        let summary = OpenRouterCalendarRequest::summary("empty-array-status-summary")
            .expect("le prompt summary doit etre valide");
        // La premiere reponse prouve que `statusCode: []` ne declenche pas le marqueur.
        let ingestion_completion = client
            .complete_json(&ingestion)
            .await
            .expect("un tableau statusCode vide doit etre accepte");
        // La seconde reponse couvre le champ court `status`.
        let summary_completion = client
            .complete_json(&summary)
            .await
            .expect("un tableau status vide doit etre accepte");
        // Les contenus traversent inchanges, preuve que seul le marqueur a ete evalue.
        assert_eq!(ingestion_completion.content(), r#"{"items":[]}"#);
        assert_eq!(summary_completion.content(), "{}");
        // Deux requetes sans retry terminent la preuve de coercition.
        assert_eq!(state.attempts.load(Ordering::Relaxed), 2);
        // Le serveur local est arrete apres les assertions.
        server.abort();
    }

    /// Scenario HTTP: un 500 est rejoue une fois puis la completion typee est retournee.
    /// Risque couvert: la politique retry doit etre bornee et ne jamais recopier le corps d'erreur.
    #[tokio::test]
    async fn transient_server_error_retries_then_returns_typed_completion() {
        // Le mock fournit exactement un echec transitoire puis un succes.
        let (endpoint, state, server) = spawn_mock_server(vec![
            MockResponse {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                body: "sensitive-provider-error".to_owned(),
                retry_after: None,
            },
            MockResponse {
                status: StatusCode::OK,
                body: success_response(),
                retry_after: None,
            },
        ])
        .await;
        // Le sleeper partage permet de verifier le backoff sans attendre dix millisecondes.
        let sleeper = Arc::new(RecordingSleeper::default());
        // Un retry autorise exactement les deux reponses preparees.
        let client = mock_client(endpoint, test_limits(4_096, 1, 1), sleeper.clone());
        // Le texte sentinelle ne doit jamais entrer dans un message d'erreur ou de debug.
        let request = OpenRouterCalendarRequest::ingestion("return JSON\nshared snapshot")
            .expect("le prompt du test doit etre valide");
        // Le second appel reussit et rend le contrat de completion.
        let completion = client
            .complete_json(&request)
            .await
            .expect("le retry 5xx doit aboutir");
        // Le contenu est disponible uniquement via l'accesseur destine au parseur Rust.
        assert_eq!(completion.content(), r#"{"items":[]}"#);
        // Le modele de reponse est lui aussi passe par le nouveau type valide.
        assert_eq!(
            completion
                .response_model()
                .map(OpenRouterCalendarModel::as_str),
            Some("test/calendar-model")
        );
        // Deux appels et pas davantage prouvent le budget ferme.
        assert_eq!(state.attempts.load(Ordering::Relaxed), 2);
        // Le premier retry applique exactement le backoff de base.
        assert_eq!(
            sleeper.durations.lock().await.as_slice(),
            &[Duration::from_millis(10)]
        );
        // Le serveur ephemere est arrete sans attendre une nouvelle connexion.
        server.abort();
    }

    /// Scenario HTTP DEV: deux 500 epuisent deux retries avant une completion valide.
    /// Risque couvert: le backoff de cinq secondes doit rester plafonne et le budget a trois appels.
    #[tokio::test]
    async fn dev_backoff_caps_both_retries_without_real_wait() {
        // Les deux erreurs transitoires forcent les deux pauses, puis la troisieme reponse termine le run.
        let (endpoint, state, server) = spawn_mock_server(vec![
            MockResponse {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                body: "first-redacted-provider-error".to_owned(),
                retry_after: None,
            },
            MockResponse {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                body: "second-redacted-provider-error".to_owned(),
                retry_after: None,
            },
            MockResponse {
                status: StatusCode::OK,
                body: success_response(),
                retry_after: None,
            },
        ])
        .await;
        // Le sleeper enregistre les durees et rend immediatement la main: le test n'attend jamais dix secondes.
        let sleeper = Arc::new(RecordingSleeper::default());
        let limits = OpenRouterCalendarLimits::new(
            Duration::from_secs(1),
            16_384,
            4_096,
            2,
            MAX_RETRY_BACKOFF,
            1,
        )
        .expect("le tuple DEV borne doit etre valide");
        let client = mock_client(endpoint, limits, sleeper.clone());
        let request = OpenRouterCalendarRequest::ingestion("return JSON\nshared snapshot")
            .expect("le prompt du test doit etre valide");

        let completion = client
            .complete_json(&request)
            .await
            .expect("la troisieme tentative doit aboutir");
        assert_eq!(completion.content(), r#"{"items":[]}"#);
        assert_eq!(state.attempts.load(Ordering::Relaxed), 3);
        assert_eq!(
            sleeper.durations.lock().await.as_slice(),
            &[MAX_RETRY_BACKOFF, MAX_RETRY_BACKOFF]
        );
        server.abort();
    }

    /// Scenario HTTP: un 429 respecte `Retry-After` mais le plafonne a cinq secondes.
    /// Risque couvert: le fournisseur ne doit pas immobiliser indefiniment une place calendrier.
    #[tokio::test]
    async fn rate_limit_retry_after_is_capped() {
        // Le header de 600 secondes depasse volontairement le plafond local.
        let (endpoint, state, server) = spawn_mock_server(vec![
            MockResponse {
                status: StatusCode::TOO_MANY_REQUESTS,
                body: "rate-limited-body".to_owned(),
                retry_after: Some("600"),
            },
            MockResponse {
                status: StatusCode::OK,
                body: success_response(),
                retry_after: None,
            },
        ])
        .await;
        // L'horloge enregistre la valeur apres plafonnement.
        let sleeper = Arc::new(RecordingSleeper::default());
        // Un retry suffit pour obtenir la seconde reponse.
        let client = mock_client(endpoint, test_limits(4_096, 1, 1), sleeper.clone());
        // Le prompt reste minimal et valide.
        let request = OpenRouterCalendarRequest::ingestion("system\nsnapshot")
            .expect("le prompt du test doit etre valide");
        // La completion confirme que le 429 a bien ete classe comme transitoire.
        client
            .complete_json(&request)
            .await
            .expect("le retry apres 429 doit aboutir");
        // Le mock observe exactement deux tentatives.
        assert_eq!(state.attempts.load(Ordering::Relaxed), 2);
        // La pause distante est ramenee au plafond ferme de cinq secondes.
        assert_eq!(
            sleeper.durations.lock().await.as_slice(),
            &[MAX_RETRY_BACKOFF]
        );
        // Le serveur est nettoye apres la preuve.
        server.abort();
    }

    /// Scenario HTTP: un 401 est permanent et ne consomme jamais le retry configure.
    /// Risque couvert: une cle invalide ne doit pas multiplier les appels ni exposer le corps fournisseur.
    #[tokio::test]
    async fn permanent_client_error_is_not_retried() {
        // Une seule reponse suffit si la classification 4xx est correcte.
        let (endpoint, state, server) = spawn_mock_server(vec![MockResponse {
            status: StatusCode::UNAUTHORIZED,
            body: "body-containing-sensitive-details".to_owned(),
            retry_after: None,
        }])
        .await;
        // Meme avec trois retries disponibles, le sleeper ne doit pas etre appele.
        let sleeper = Arc::new(RecordingSleeper::default());
        // Le maximum de retries rend l'absence de rejeu explicite.
        let client = mock_client(endpoint, test_limits(4_096, 3, 1), sleeper.clone());
        // Le contenu du prompt n'intervient pas dans la classification HTTP.
        let request = OpenRouterCalendarRequest::summary("system\nsnapshot")
            .expect("le prompt du test doit etre valide");
        // L'erreur retourne uniquement le code HTTP, jamais le body.
        let error = client
            .complete_json(&request)
            .await
            .expect_err("un 401 doit rester permanent");
        // Le contrat redige expose le statut utile a l'operateur.
        assert_eq!(
            error,
            OpenRouterCalendarError::HttpStatus { status_code: 401 }
        );
        // Une seule tentative prouve que le budget n'est pas gaspille sur un echec permanent.
        assert_eq!(state.attempts.load(Ordering::Relaxed), 1);
        // Aucun backoff n'a ete declenche.
        assert!(sleeper.durations.lock().await.is_empty());
        // Le `Debug` de l'erreur ne contient pas le corps distant.
        assert!(!format!("{error:?}").contains("sensitive-details"));
        // Le serveur n'est plus necessaire apres les assertions.
        server.abort();
    }

    /// Scenario HTTP: chaque tentative lente expire et le budget total reste strictement borne.
    /// Risque couvert: un fournisseur bloque ne doit pas monopoliser le worker calendrier indefiniment.
    #[tokio::test]
    async fn request_timeout_retries_once_then_returns_redacted_exhaustion() {
        // Le serveur met 250 ms a repondre alors que chaque tentative dispose de 20 ms.
        let (endpoint, attempts, server) = spawn_slow_mock_server().await;
        // Le sleeper supprime l'attente de backoff mais conserve sa preuve quantitative.
        let sleeper = Arc::new(RecordingSleeper::default());
        // Un seul retry autorise exactement deux tentatives timeout.
        let limits = OpenRouterCalendarLimits::new(
            Duration::from_millis(20),
            16_384,
            4_096,
            1,
            Duration::from_millis(10),
            1,
        )
        .expect("les limites timeout du test doivent etre valides");
        // Le client partage le meme contrat que les autres tests HTTP loopback.
        let client = mock_client(endpoint, limits, sleeper.clone());
        // Les sentinelles de prompt ne doivent jamais entrer dans l'erreur retournee.
        let request =
            OpenRouterCalendarRequest::summary("system-timeout-secret\nsnapshot-timeout-secret")
                .expect("le prompt timeout doit etre valide");
        // Les deux expirations deviennent une preuve fermee sans erreur reqwest brute.
        let error = client
            .complete_json(&request)
            .await
            .expect_err("les tentatives lentes doivent epuiser le budget");
        // L'enum prouve a la fois le nombre exact d'appels et l'absence de statut HTTP sur un timeout.
        assert_eq!(
            error,
            OpenRouterCalendarError::RetriesExhausted {
                attempts: 2,
                last_failure: OpenRouterCalendarFailureKind::Timeout,
                status_code: None,
            }
        );
        // Le serveur a vu exactement la tentative initiale et le retry autorise.
        assert_eq!(attempts.load(Ordering::Relaxed), 2);
        // Un unique backoff separe les deux tentatives.
        assert_eq!(
            sleeper.durations.lock().await.as_slice(),
            &[Duration::from_millis(10)]
        );
        // Le format d'erreur ne recopie aucune sentinelle du prompt.
        assert!(!format!("{error:?}").contains("timeout-secret"));
        // L'abandon explicite rend au runtime le socket du serveur lent encore endormi.
        server.abort();
    }

    /// Scenario HTTP: un corps plus grand que la limite est interrompu et classe sans retry.
    /// Risque couvert: une reponse inattendue ne doit pas provoquer une allocation non bornee.
    #[tokio::test]
    async fn response_body_is_strictly_bounded() {
        // Le body de 256 octets depasse volontairement la limite de 64 octets.
        let (endpoint, state, server) = spawn_mock_server(vec![MockResponse {
            status: StatusCode::OK,
            body: "x".repeat(256),
            retry_after: None,
        }])
        .await;
        // Aucun retry n'est utile pour un contrat de taille permanent.
        let sleeper = Arc::new(RecordingSleeper::default());
        // La petite limite exerce le controle Content-Length ou chunk, selon le serveur.
        let client = mock_client(endpoint, test_limits(64, 0, 1), sleeper);
        // Le prompt reste sous sa propre limite.
        let request = OpenRouterCalendarRequest::summary("system\nsnapshot")
            .expect("le prompt du test doit etre valide");
        // La lecture refuse la reponse avant tout parsing Serde.
        let error = client
            .complete_json(&request)
            .await
            .expect_err("le corps trop grand doit etre refuse");
        // L'erreur distingue clairement la borne memoire d'un JSON simplement invalide.
        assert_eq!(error, OpenRouterCalendarError::ResponseTooLarge);
        // Une seule tentative suffit car rejouer le meme corps ne resoudrait rien.
        assert_eq!(state.attempts.load(Ordering::Relaxed), 1);
        // Le serveur ephemere est arrete explicitement.
        server.abort();
    }
}
