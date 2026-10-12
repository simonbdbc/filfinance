//! Contrats purs de l'orchestrateur calendrier natif.
//!
//! Ce module fige le comportement canonique du calendrier natif sans effectuer
//! d'I/O, sans persister de resultat et sans produire d'audio ou d'alerte.
//!
//! `serde_json::Value` n'apparait qu'a la frontiere de parsing de la reponse IA.
//! Une fois cette frontiere franchie, chaque etape manipule les structures Rust
//! ci-dessous. `BTreeMap` donne un ordre de cles stable aux hashes, tandis que le
//! `HashMap` de deduplication sert uniquement aux recherches et n'est jamais
//! parcouru pour produire le resultat. Les `Result` propagent des erreurs
//! redigees par `?`, sans conserver le texte source ou la reponse brute.

// `BTreeMap` stabilise l'ordre JSON des objets bruts; `HashMap` privilegie les recherches de doublons.
use std::collections::{BTreeMap, HashMap};
// Le module `fmt` fournit les traits de formatage utilises par les implementations `Debug` redigees.
use std::fmt;

// Les traits Serde derives couvrent les contrats usuels; les traits nommes servent aux formats manuels.
use serde::{Deserialize, Deserializer, Serialize, Serializer};
// `Value` et `Map` restent confines a la frontiere IA; `Number` preserve les nombres JSON sans perte.
use serde_json::{Map, Number, Value};
// `Digest` rend disponibles les methodes du hacheur SHA-256 concret `Sha256`.
use sha2::{Digest, Sha256};
// Ces types portent les dates; `Rfc3339` impose le format horaire accepte entre les etapes.
use time::{Date, Month, OffsetDateTime, Weekday, format_description::well_known::Rfc3339};
// `Url` porte une URL deja parsemee et permet de valider la provenance fournie par le collecteur.
use url::Url;

/// Version changee uniquement lorsqu'un golden canonique change volontairement.
// `&'static str` emprunte une chaine compilee dans le binaire et n'alloue rien a chaque payload.
pub const CALENDAR_NATIVE_CONTRACT_VERSION: &str = "2026-08-24.calendar-native-v1";

// Les bornes rendent le futur worker predictible avant toute concurrence ou file asynchrone.
// `usize` correspond naturellement aux tailles de chaines et vecteurs retournees par Rust.
const MAX_SOURCE_TEXT_CHARS: usize = 10_000;
// La reponse modele est bornee en octets, unite directement fournie par `str::len`.
const MAX_MODEL_OUTPUT_BYTES: usize = 1_000_000;
// La resynchronisation des reponses IA reste bornee meme sur une sortie parseable hostile.
const MAX_BALANCED_JSON_CANDIDATES: usize = 256;
// Le budget evite le pire cas quadratique tout en autorisant plusieurs reprises sur la borne modele.
const MAX_BALANCED_JSON_SCAN_STEPS: usize = MAX_MODEL_OUTPUT_BYTES * 8;
// Cette limite empeche qu'un unique run monopolise la file calendrier avec trop d'items.
const MAX_CALENDAR_ITEMS: usize = 512;
// Chaque item borne aussi sa fan-out de provenances independamment de la limite globale.
const MAX_ITEM_SOURCES: usize = 16;
// Le JSON `raw` est mesure avant sa conversion recursive vers le type compact.
const MAX_RAW_SOURCE_BYTES: usize = 12_000;
// Les metadonnees de provenance sont la seule portion d'un item qui peut
// rejoindre SQLite: elles restent donc beaucoup plus petites qu'une reponse IA.
const MAX_PERSISTED_SOURCE_EXTERNAL_ID_BYTES: usize = 160;
const MAX_PERSISTED_SOURCE_RAW_TITLE_CHARS: usize = 240;
const MAX_PERSISTED_SOURCE_RAW_TITLE_BYTES: usize = 1_024;

/// Provenance strictement fermee : seuls les flux actuellement autorises
/// peuvent traverser les etapes du calendrier natif.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum CalendarSourceKind {
    /// Flux officiel dont l'identifiant et l'endpoint sont fermes.
    Feed(super::native::feeds::CalendarFeedSource),
}

// Le bloc `impl` regroupe les conversions et controles qui appartiennent au type de provenance.
impl CalendarSourceKind {
    // L'entree `&str` est seulement empruntee: parser un nom ne prend pas possession du buffer appelant.
    pub(crate) fn from_name(value: &str) -> Option<Self> {
        super::native::feeds::CalendarFeedSource::from_name(value.trim()).map(Self::Feed)
    }

    // La valeur filaire est empruntee au type pour serialiser sans allouer une nouvelle `String`.
    fn as_str(&self) -> &str {
        // Le `match` exhaustif oblige le compilateur a signaler toute future variante non serialisee.
        match self {
            Self::Feed(source) => source.name(),
        }
    }

    // L'emprunt `&self` permet le controle d'autorisation sans consommer la provenance du snapshot.
    fn is_authorized_collector(&self) -> bool {
        // `matches!` exprime ici une appartenance fermee sans exposer de chaine libre a la collecte.
        matches!(self, Self::Feed(_))
    }
}

// Le format filaire reste une chaine dans le tableau `sources` du contrat canonique.
// Le trait custom evite la representation d'enum externe que produirait un derive Serde par defaut.
impl Serialize for CalendarSourceKind {
    // Le parametre generique `S` laisse Serde choisir JSON ou tout autre encodeur compatible.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        // Cette borne garantit a la compilation que `S` sait encoder une chaine.
        S: Serializer,
    {
        // L'emprunt retourne par `as_str` suffit: aucune copie de provenance n'est requise.
        serializer.serialize_str(self.as_str())
    }
}

// `'de` lie la duree de vie des donnees lues au deserialiseur, meme si le resultat final possede sa chaine.
impl<'de> Deserialize<'de> for CalendarSourceKind {
    // Comme pour `Serialize`, le generique `D` rend le contrat independant du format JSON concret.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        // La borne autorise Serde a fournir des donnees valides pendant toute la lecture `'de`.
        D: Deserializer<'de>,
    {
        // `?` retourne immediatement l'erreur Serde sans la convertir en erreur metier ni la masquer.
        let value = String::deserialize(deserializer)?;
        Self::from_name(&value)
            .ok_or_else(|| serde::de::Error::custom("unsupported calendar source"))
    }
}

/// Niveau d'importance canonique partage entre les etapes.
// `Copy` est adapte a ce petit enum sans allocation; les derives Serde et d'ordre figent son contrat filaire.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
// Serde convertit les variantes Rust en chaines filaires minuscules separees par des underscores.
#[serde(rename_all = "snake_case")]
pub enum CalendarImportance {
    /// Information de faible priorite dans le calendrier.
    Low,
    /// Valeur prudente utilisee comme fallback canonique.
    Medium,
    /// Information importante susceptible d'alimenter le resume.
    High,
}

// La normalisation reste associee au type afin que tous les appelants partagent les memes fallbacks.
impl CalendarImportance {
    // `Option<&Value>` distingue un champ absent d'un JSON present, sans prendre possession du DOM parse.
    fn parse(value: Option<&Value>, fallback: Self) -> Self {
        // Aucun `trim` n'est applique ici: des marges rendent volontairement la valeur invalide.
        match value.and_then(value_as_enum_text).as_deref() {
            Some(value) if value.eq_ignore_ascii_case("low") => Self::Low,
            Some(value) if value.eq_ignore_ascii_case("medium") => Self::Medium,
            Some(value) if value.eq_ignore_ascii_case("high") => Self::High,
            _ => fallback,
        }
    }
}

/// Categorie canonique acceptee par le pipeline natif actuel.
// Ce petit enum est `Copy`; Serde assure la meme forme snake_case entre Rust et fixtures.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum CalendarCategory {
    /// Evenement macroeconomique general ou fallback prudent de la baseline.
    Macro,
    /// Decision, anticipation ou mouvement de taux d'interet.
    Rates,
    /// Publication ou indicateur directement lie a l'inflation.
    Inflation,
    /// Communication ou decision emanant d'une banque centrale.
    CentralBank,
    /// Evenement geopolitique susceptible d'influencer les marches.
    Geopolitics,
    /// Publication ou evenement propre a une entreprise.
    Company,
    /// Evenement lie aux crypto-actifs et a leur ecosysteme.
    Crypto,
    /// Categorie residuelle explicitement choisie lorsque les autres variantes ne conviennent pas.
    Other,
}

// L'`impl` rattache la normalisation au type afin d'eviter des mappings de chaines divergents entre etapes.
impl CalendarCategory {
    // Le fallback `Macro` reproduit la baseline lorsque le champ est absent ou hors allowlist.
    fn parse(value: Option<&Value>) -> Self {
        // La categorie partage la conversion JavaScript sans `trim` des niveaux d'importance.
        match value.and_then(value_as_enum_text).as_deref() {
            // Les gardes comparent sans tenir compte de la casse, selon la normalisation canonique.
            Some(value) if value.eq_ignore_ascii_case("rates") => Self::Rates,
            // Chaque branche retourne une variante `Copy`, donc aucune chaine n'est conservee dans le contrat.
            Some(value) if value.eq_ignore_ascii_case("inflation") => Self::Inflation,
            // Le nom filaire snake_case est traduit vers la variante Rust en CamelCase.
            Some(value) if value.eq_ignore_ascii_case("central_bank") => Self::CentralBank,
            // Les variantes restantes suivent la meme allowlist fermee et auditable.
            Some(value) if value.eq_ignore_ascii_case("geopolitics") => Self::Geopolitics,
            // La branche entreprise ne reutilise jamais une categorie textuelle libre.
            Some(value) if value.eq_ignore_ascii_case("company") => Self::Company,
            // La branche crypto reste une variante compile-time au meme titre que les categories macro.
            Some(value) if value.eq_ignore_ascii_case("crypto") => Self::Crypto,
            // La valeur filaire explicite `other` devient `Other`, distincte du fallback invalide `Macro`.
            Some(value) if value.eq_ignore_ascii_case("other") => Self::Other,
            // Le pattern joker couvre a la fois l'absence et toute valeur hors allowlist.
            _ => Self::Macro,
        }
    }
}

/// Valeur JSON explicitement typee; le parseur borne sa representation avant construction.
// `Clone` est requis quand le JSON brut doit etre possede; Serde permet ensuite de hacher ce type stable.
#[derive(Clone, Serialize, Deserialize, PartialEq)]
// `untagged` conserve exactement la forme JSON d'origine, sans ajouter un champ discriminant Rust.
#[serde(untagged)]
pub enum CalendarCompactValue {
    /// Absence JSON explicite, distincte d'un champ Rust absent.
    Null,
    /// Booleen copie directement car `bool` implemente `Copy`.
    Bool(bool),
    /// Nombre Serde possede, qui preserve les representations JSON acceptees.
    Number(Number),
    /// Chaine possedee, necessaire car le DOM `Value` d'entree est seulement emprunte.
    String(String),
    /// Tableau recursif possede et borne avant l'appel de conversion.
    Array(Vec<CalendarCompactValue>),
    /// Objet trie par cle pour rendre sa serialisation et son hash deterministes.
    Object(BTreeMap<String, CalendarCompactValue>),
}

// La conversion recursive quitte volontairement `serde_json::Value` apres la frontiere d'entree.
impl CalendarCompactValue {
    // `&Value` evite de consommer le DOM parse; les feuilles possedees sont clonees au besoin.
    fn from_json(value: &Value) -> Self {
        // Ce `match` exhaustif garantit qu'une nouvelle variante Serde devra etre traitee explicitement.
        match value {
            Value::Null => Self::Null,
            // Le dereferencement copie un `bool`, qui n'a pas de cout d'allocation.
            Value::Bool(value) => Self::Bool(*value),
            // `Number` n'est pas `Copy`; `clone` transfere sa representation dans le contrat possede.
            Value::Number(value) => Self::Number(value.clone()),
            // La chaine doit egalement etre clonee pour ne plus dependre du DOM d'entree.
            Value::String(value) => Self::String(value.clone()),
            Value::Array(values) => {
                // L'iterateur emprunte chaque enfant, le convertit, puis `collect` construit le `Vec` possede.
                Self::Array(values.iter().map(Self::from_json).collect::<Vec<_>>())
            }
            Value::Object(values) => Self::Object(
                // Iterer sur la map Serde n'en prend pas possession; seules les cles du contrat sont clonees.
                values
                    .iter()
                    .map(|(key, value)| (key.clone(), Self::from_json(value)))
                    // Le type cible `BTreeMap` est deduit par la variante `Object` et trie les cles.
                    .collect(),
            ),
        }
    }
}

/// Snapshot borne d'une source. Le texte reste en memoire et n'est jamais serialise.
// `Clone` permet au futur worker de transmettre un snapshot possede; `PartialEq/Eq` servent aux goldens.
#[derive(Clone, Serialize, PartialEq, Eq)]
// Les noms JSON restent en camelCase pour preserver le contrat publie sans renommer les champs Rust.
#[serde(rename_all = "camelCase")]
pub struct CalendarSourceSnapshot {
    /// Version possedee par le payload afin qu'il reste autonome apres la construction.
    pub contract_version: String,
    /// Jour produit ISO possede, reutilise par les etapes ulterieures sans emprunt externe.
    pub local_day: String,
    /// Provenance typee; les variantes libres restent interdites pour une collecte native.
    pub source: CalendarSourceKind,
    /// URL demandee, conservee dans le contrat mais exclue du `Debug` manuel.
    pub url: String,
    /// URL effectivement atteinte, distincte pour une future gestion controlee des redirections.
    pub fetched_url: String,
    /// Etat HTTP compact; un `bool` evite de transporter une reponse reseau complete.
    pub ok: bool,
    /// Preuve que le texte contient bien le jour demande avant tout appel IA.
    pub target_day_found: bool,
    /// Empreinte stable permettant la comparaison sans conserver le texte dans les rapports.
    pub text_hash: String,
    /// Nombre d'unites UTF-16, identique a la semantique `text.length` JavaScript historique.
    pub text_length: usize,
    // `serde(skip)` impose que le contenu collecte ne quitte jamais ce contrat en serialisation.
    #[serde(skip)]
    // La `String` privee possede le texte compact; son acces est limite a `prompt_text`.
    text: String,
}

// Les methodes construisent et exposent le snapshot tout en maintenant sa politique de redaction.
impl CalendarSourceSnapshot {
    /// Construit le meme contenu compact/date-aware que `Build Ingest Prompt`.
    pub fn from_response_text(
        // Les `&str` d'entree sont empruntes pendant l'appel et clones seulement dans le payload final.
        local_day: &str,
        // `source` est pris par valeur: le snapshot devient l'unique proprietaire de cette provenance.
        source: CalendarSourceKind,
        url: &str,
        // `u16` suffit a representer tous les codes HTTP sans allocation.
        status: u16,
        body: &str,
    ) -> Result<Self, CalendarOrchestrationError> {
        // `?` stoppe la construction sur une date invalide et propage l'erreur typee a l'appelant.
        validate_local_day(local_day)?;
        // Le controle precede tout traitement couteux du corps pour refuser une provenance non autorisee.
        if !source.is_authorized_collector() {
            return Err(CalendarOrchestrationError::UnsupportedSource);
        }
        // Le texte devient une `String` possedee, nettoyee et bornee avant hash ou stockage en memoire.
        let text = if matches!(source, CalendarSourceKind::Feed(_)) {
            body.to_owned()
        } else {
            compact_source_text(body)
        };
        // L'emprunt `&text` evite un clone; `?` propage une incoherence de date de facon redigee.
        let target_day_found = !matches!(&source, CalendarSourceKind::Feed(feed) if feed.is_context())
            && contains_target_day(&text, local_day)?;
        // `Ok(Self { ... })` construit le type courant et l'enveloppe dans le succes du `Result`.
        Ok(Self {
            // `to_owned` transforme chaque constante/tranche empruntee en champ autonome du snapshot.
            contract_version: CALENDAR_NATIVE_CONTRACT_VERSION.to_owned(),
            local_day: local_day.to_owned(),
            // Le mouvement transfere `source` dans le snapshot sans clone supplementaire.
            source,
            url: url.to_owned(),
            fetched_url: url.to_owned(),
            // La plage semi-ouverte accepte 2xx/3xx et exclut 400, comme la baseline.
            ok: (200..400).contains(&status),
            target_day_found,
            // `as_bytes` emprunte la representation UTF-8 utilisee par SHA-256 sans allocation.
            text_hash: sha256_hex(text.as_bytes()),
            // La compatibilite JavaScript mesure `text.length` en UTF-16; cette preuve conserve la meme unite.
            text_length: text.encode_utf16().count(),
            // Cette derniere affectation deplace la `String`; aucune copie du contenu source n'est gardee.
            text,
        })
    }

    /// Donne le contenu a l'etape de prompt; le journal calendrier isole le
    /// capture explicitement pour le diagnostic, sans l'ajouter aux rapports
    /// ou traces applicatives ordinaires.
    pub fn prompt_text(&self) -> &str {
        // Le retour emprunte est lie a `&self`: l'appelant peut lire mais ni modifier ni conserver seul le texte.
        &self.text
    }
}

// L'implementation manuelle empeche `Debug` de contourner le `serde(skip)` du texte source.
impl fmt::Debug for CalendarSourceSnapshot {
    // Le formatter mutable est emprunte par convention du trait; il ne devient jamais un champ du snapshot.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Le builder de debug ne recoit volontairement ni URL ni texte, donc aucune donnee omise ne fuit.
        formatter
            .debug_struct("CalendarSourceSnapshot")
            // Chaque `&champ` est un emprunt temporaire impose par l'API de formatage.
            .field("contract_version", &self.contract_version)
            .field("local_day", &self.local_day)
            .field("source", &self.source)
            .field("ok", &self.ok)
            .field("target_day_found", &self.target_day_found)
            .field("text_hash", &self.text_hash)
            .field("text_length", &self.text_length)
            // `finish_non_exhaustive` signale aussi visuellement que des champs ont ete volontairement rediges.
            .finish_non_exhaustive()
    }
}

/// Provenance canonique d'un evenement calendrier.
// Les derives produisent le contrat JSON type et les comparaisons de fixtures sans `Debug` potentiellement bavard.
#[derive(Clone, Serialize, Deserialize, PartialEq)]
// L'attribut traduit les noms Rust snake_case en champs camelCase du contrat publie.
#[serde(rename_all = "camelCase")]
pub struct CalendarItemSourceDraft {
    /// Provenance typee serialisee comme chaine par son implementation Serde manuelle.
    pub source: CalendarSourceKind,
    /// `Option<String>` distingue l'absence historique d'un identifiant possede et present.
    pub external_id: Option<String>,
    /// URL source optionnelle possedee, jamais supposee presente lors du parsing IA.
    pub source_url: Option<String>,
    /// Titre brut optionnel conserve pour la parite, separe du titre visible traduit.
    pub raw_title: Option<String>,
    /// Valeur brute typee et ordonnee, utilisable pour le hash sans conserver un `Value` permissif.
    pub raw: CalendarCompactValue,
}

/// Liaison typee entre une source effectivement couverte et son URL canonique du jour.
///
/// Cette valeur est construite par le collecteur, jamais a partir de la reponse IA. Elle empeche
/// donc l'IA de choisir une URL persistante ou une source qui n'a pas ete verifiee pendant le run.
#[derive(Clone, PartialEq, Eq)]
pub struct CalendarCoveredSourceBinding {
    source: CalendarSourceKind,
    canonical_url: String,
}

impl CalendarCoveredSourceBinding {
    /// Construit une liaison seulement pour l'endpoint ferme de la source et du jour ouvrable.
    pub fn for_local_day(
        local_day: &str,
        source: CalendarSourceKind,
        canonical_url: Url,
    ) -> Result<Self, CalendarOrchestrationError> {
        validate_local_day(local_day)?;
        if !source.is_authorized_collector() {
            return Err(CalendarOrchestrationError::UnsupportedSource);
        }
        if !is_canonical_collector_url(local_day, &source, &canonical_url) {
            return Err(CalendarOrchestrationError::InvalidCoveredSourceBinding);
        }
        Ok(Self {
            source,
            canonical_url: canonical_url.into(),
        })
    }

    /// URL datee et validee qui remplace toute URL proposee par l'IA.
    pub fn canonical_url(&self) -> &str {
        &self.canonical_url
    }
}

impl fmt::Debug for CalendarCoveredSourceBinding {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CalendarCoveredSourceBinding")
            .field("source", &self.source)
            .field("canonical_url", &self.canonical_url)
            .finish()
    }
}

/// Projette atomiquement les provenances IA sur les seules sources couvertes du run natif.
///
/// La projection conserve uniquement `source`, `externalId`, `rawTitle` et l'URL datee du
/// collecteur. Les structures `raw` ne traversent jamais cette frontiere vers la persistance.
pub fn project_native_ingestion_provenance(
    draft: &mut CalendarIngestionDraft,
    covered_sources: &[CalendarCoveredSourceBinding],
) -> Result<(), CalendarOrchestrationError> {
    crate::features::calendar_logs::sync_operation("project_native_ingestion_provenance", || {
        let mut projected_items = draft.items.clone();
        for item in &mut projected_items {
            project_native_item_provenance(item, covered_sources)?;
        }
        draft.items = projected_items;
        Ok(())
    })
}

/// Projette les provenances d'un item deja canonicalise sans faire de persistance.
pub fn project_native_item_provenance(
    item: &mut CalendarItemDraft,
    covered_sources: &[CalendarCoveredSourceBinding],
) -> Result<(), CalendarOrchestrationError> {
    crate::features::calendar_logs::sync_operation("project_native_item_provenance", || {
        if item.sources.is_empty() {
            return Err(CalendarOrchestrationError::MissingItemProvenance);
        }
        if covered_sources.is_empty() {
            return Err(CalendarOrchestrationError::SourceCoverageMismatch);
        }

        let mut bindings = BTreeMap::new();
        for binding in covered_sources {
            if bindings.insert(binding.source.clone(), binding).is_some() {
                return Err(CalendarOrchestrationError::DuplicateCoveredSourceBinding);
            }
        }

        let mut projected_by_source: BTreeMap<CalendarSourceKind, CalendarItemSourceDraft> =
            BTreeMap::new();
        for source in &item.sources {
            if !source.source.is_authorized_collector() {
                return Err(CalendarOrchestrationError::UnknownItemProvenance);
            }
            if source.source_url.is_some() {
                return Err(CalendarOrchestrationError::ModelItemProvenanceUrl);
            }
            let binding = bindings
                .get(&source.source)
                .ok_or(CalendarOrchestrationError::UncoveredItemProvenance)?;
            let projected = CalendarItemSourceDraft {
                source: source.source.clone(),
                external_id: project_native_source_external_id(source.external_id.as_deref())?,
                source_url: Some(binding.canonical_url.clone()),
                raw_title: project_native_source_raw_title(source.raw_title.as_deref())?,
                raw: CalendarCompactValue::Null,
            };
            if let Some(existing) = projected_by_source.get(&source.source) {
                if existing.external_id != projected.external_id
                    || existing.raw_title != projected.raw_title
                {
                    return Err(CalendarOrchestrationError::ConflictingItemProvenance);
                }
            } else {
                projected_by_source.insert(source.source.clone(), projected);
            }
        }

        let mut sources = Vec::with_capacity(projected_by_source.len());
        for binding in covered_sources {
            if let Some(source) = projected_by_source.remove(&binding.source) {
                sources.push(source);
            }
        }
        if sources.is_empty() {
            return Err(CalendarOrchestrationError::MissingItemProvenance);
        }
        item.sources = sources;
        Ok(())
    })
}

/// Conserve un identifiant compact, mais refuse une URL, un cookie ou un
/// fragment de payload qui pourrait etre rendu par les surfaces admin.
pub(crate) fn project_native_source_external_id(
    value: Option<&str>,
) -> Result<Option<String>, CalendarOrchestrationError> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    if value.len() > MAX_PERSISTED_SOURCE_EXTERNAL_ID_BYTES
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
        || contains_unsafe_native_source_metadata(value)
    {
        return Err(CalendarOrchestrationError::UnsafeItemProvenanceMetadata);
    }
    Ok(Some(value.to_owned()))
}

/// Reduit un titre de provenance a une ligne courte de texte humain.
///
/// Cette allowlist bloque balises, URLs, JSON et marqueurs anti-bot avant que
/// le titre ne puisse etre insere dans `sources_json` ou `calendar_day_item_sources`.
pub(crate) fn project_native_source_raw_title(
    value: Option<&str>,
) -> Result<Option<String>, CalendarOrchestrationError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.is_empty() {
        return Ok(None);
    }
    let plain_title = normalized.chars().all(|character| {
        character.is_alphanumeric()
            || character.is_whitespace()
            || matches!(
                character,
                '.' | ','
                    | ':'
                    | ';'
                    | '!'
                    | '?'
                    | '\''
                    | '"'
                    | '’'
                    | '-'
                    | '–'
                    | '—'
                    | '('
                    | ')'
                    | '%'
                    | '+'
                    | '/'
                    | '&'
                    | '$'
                    | '€'
                    | '£'
            )
    });
    if normalized.chars().count() > MAX_PERSISTED_SOURCE_RAW_TITLE_CHARS
        || normalized.len() > MAX_PERSISTED_SOURCE_RAW_TITLE_BYTES
        || !plain_title
        || contains_unsafe_native_source_metadata(&normalized)
    {
        return Err(CalendarOrchestrationError::UnsafeItemProvenanceMetadata);
    }
    Ok(Some(normalized))
}

/// Garde les marqueurs de challenge, d'URL et de secret hors des metadonnees
/// persistees, sans inclure la valeur fautive dans l'erreur redigee.
fn contains_unsafe_native_source_metadata(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    value.contains("://")
        || value.contains("www.")
        || value.contains('<')
        || value.contains('>')
        || value.contains('{')
        || value.contains('}')
        || value.contains('[')
        || value.contains(']')
        || [
            "cf-mitigated",
            "cf_clearance",
            "captcha",
            "challenge",
            "enable javascript",
            "enable cookies",
            "set-cookie",
            "authorization",
            "bearer ",
            "session",
            "token",
        ]
        .iter()
        .any(|marker| value.contains(marker))
}

/// Evenement canonique produit par l'etape d'ingestion.
// `Clone` rend le draft transferable entre futures etapes; Serde fige son schema; `PartialEq` sert aux goldens.
#[derive(Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CalendarItemDraft {
    /// Horaire RFC3339 optionnel; `None` represente explicitement un evenement sans heure.
    pub event_time: Option<String>,
    /// Titre source possede, distinct du texte traduit expose aux sorties francaises.
    pub source_title: String,
    /// Titre francais obligatoire et possede par le contrat.
    pub translated_title: String,
    /// Texte francais destine au futur worker Piper existant.
    pub tts_text: String,
    /// Pays optionnel conserve sous forme textuelle faute d'allowlist historique fermee.
    pub country: Option<String>,
    /// Devise optionnelle issue de la source ou du modele.
    pub currency: Option<String>,
    /// Prevision optionnelle; une `String` preserve les formats non numeriques de la baseline.
    pub forecast_value: Option<String>,
    /// Valeur precedente optionnelle, egalement preservee sans coercition numerique.
    pub previous_value: Option<String>,
    /// Valeur publiee optionnelle telle que recue dans le contrat canonique.
    pub actual_value: Option<String>,
    /// Importance annoncee par la source et normalisee vers un enum ferme.
    pub source_importance: CalendarImportance,
    /// Importance proposee par l'IA, separee pour garder la preuve de decision.
    pub ai_importance: CalendarImportance,
    /// Importance finale appliquee apres la regle de dominance `high`.
    pub final_importance: CalendarImportance,
    /// Categorie canonique fermee, avec fallback `Macro` compatible avec le contrat publie.
    pub category: CalendarCategory,
    /// Justification francaise possedee et requise avant la suite du pipeline.
    pub reasoning: String,
    /// Confiance bornee entre zero et un apres coercition compatible JavaScript.
    pub confidence: f64,
    /// Provenances possedees et bornees pour eviter un fan-out non controle.
    pub sources: Vec<CalendarItemSourceDraft>,
}

/// Payload canonique d'ingestion, stable et independant des lignes SQLite.
// Les derives rendent le payload clonable/serialisable et comparable sans lui donner un `Debug` non redige.
#[derive(Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CalendarIngestionDraft {
    /// Version possedee incluse dans le hash et les preuves de parite.
    pub contract_version: String,
    /// Jour produit ISO commun a tous les items du vecteur.
    pub local_day: String,
    /// `Vec` possede un nombre borne d'items dans leur ordre canonique.
    pub items: Vec<CalendarItemDraft>,
}

// Ces lectures derivees restent pures et n'introduisent ni cache mutable ni divergence de donnees.
impl CalendarIngestionDraft {
    // Le hash emprunte le payload complet et retourne une erreur si sa serialisation canonique echoue.
    pub fn canonical_hash(&self) -> Result<String, CalendarOrchestrationError> {
        // Le generique est infere comme `CalendarIngestionDraft`; aucun `Value` intermediaire n'est cree.
        canonical_hash(self)
    }

    // Le compteur est recalcule depuis l'enum canonique pour ne pas stocker une preuve desynchronisable.
    pub fn high_count(&self) -> usize {
        // `iter` emprunte les items; `filter` ne clone rien et `count` consomme seulement l'iterateur.
        self.items
            .iter()
            .filter(|item| item.final_importance == CalendarImportance::High)
            .count()
    }
}

// Le draft reste inspectable sans rendre les titres, raisonnements ou provenances visibles
// par `Debug`. Le journal calendrier isole les capture explicitement par Serde; les autres
// traces restent redigees.
impl fmt::Debug for CalendarIngestionDraft {
    // Le formatter est emprunte pour la duree de l'appel et le `Result` propage une erreur de formatage eventuelle.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Le builder `debug_struct` ne recoit que les metadonnees explicitement autorisees ci-dessous.
        formatter
            .debug_struct("CalendarIngestionDraft")
            // La version et le jour restent utiles pour correler un contrat sans exposer ses formulations.
            .field("contract_version", &self.contract_version)
            // Le jour est reemprunte directement au draft et ne cree aucune copie dans le formatter.
            .field("local_day", &self.local_day)
            // Les compteurs sont derives par emprunt; aucun item ni texte n'est clone dans la trace.
            .field("item_count", &self.items.len())
            // `high_count` parcourt les items sans jamais les rendre eux-memes au builder de debug.
            .field("high_count", &self.high_count())
            // `finish` clot le builder et retourne directement le `fmt::Result` attendu par le trait.
            .finish()
    }
}

/// Item SQLite complet utilise par le prompt summary natif.
// La structure reprend l'ordre des champs de `CalendarDayItem`: Serde peut ainsi reconstruire le
// meme objet JSON sans repasser par un `Value` implicite ni perdre de contexte economique.
#[derive(Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CalendarSummaryInputItem {
    /// Identifiant SQLite copie par valeur, stable pour la preuve de parite.
    pub id: i64,
    /// Jour produit stocke avec l'item, controle contre le run avant le hand-off.
    pub local_day: String,
    /// Horaire optionnel possede, utilise pour l'ordre du resume.
    pub event_time: Option<String>,
    /// Titre brut conserve exactement comme dans l'entree historique `allItems`.
    pub source_title: String,
    /// Titre francais possede, utilise par la synthese et la gate visible.
    pub translated_title: String,
    /// Narration evenementielle deja normalisee, utile au contexte du resume.
    pub tts_text: String,
    /// Pays optionnel transmis sans normalisation supplementaire.
    pub country: Option<String>,
    /// Devise optionnelle transmise sans normalisation supplementaire.
    pub currency: Option<String>,
    /// Valeur attendue optionnelle telle qu'elle est stockee par la baseline.
    pub forecast_value: Option<String>,
    /// Valeur precedente optionnelle telle qu'elle est stockee par la baseline.
    pub previous_value: Option<String>,
    /// Valeur publiee optionnelle telle qu'elle est stockee par la baseline.
    pub actual_value: Option<String>,
    /// Importance source optionnelle gardee comme texte canonique SQLite.
    pub source_importance: Option<String>,
    /// Importance IA optionnelle gardee comme texte canonique SQLite.
    pub ai_importance: Option<String>,
    /// Importance finale typee, utilisee pour selectionner les informations importantes.
    pub final_importance: CalendarImportance,
    /// Categorie canonique transmise au modele pour conserver le meme contexte.
    pub category: String,
    /// Justification francaise optionnelle transmise au prompt de synthese.
    pub reasoning: Option<String>,
    /// Confiance flottante optionnelle; `PartialEq` suffit car elle n'est jamais une cle de map.
    pub confidence: Option<f64>,
    /// Empreinte SQLite de l'item, partagee telle quelle avec la baseline.
    pub content_hash: String,
    /// JSON des provenances conserve comme chaine, exactement comme le contrat d'items publie.
    pub sources_json: String,
    /// Horodatage de creation SQLite present dans l'objet baseline.
    pub created_at: String,
    /// Horodatage de mise a jour SQLite present dans l'objet baseline.
    pub updated_at: String,
}

// Ce comportement appartient au contrat d'item car canonicaliseur et builder doivent partager le meme tri.
impl CalendarSummaryInputItem {
    /// Retourne l'horaire de tri: evenement, puis creation SQLite, puis sentinelle de fin.
    pub fn sort_time(&self) -> &str {
        // Une chaine vide est fausse en JavaScript; `filter` preserve cette nuance avant le fallback.
        self.event_time
            .as_deref()
            .filter(|value| !value.is_empty())
            .or_else(|| (!self.created_at.is_empty()).then_some(self.created_at.as_str()))
            .unwrap_or("9999-12-31T23:59:59Z")
    }
}

/// Resume canonique et preuves compactes de ses entrees.
// Les derives permettent le passage type entre etapes et des assertions exactes sur les fixtures.
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CalendarSummaryDraft {
    /// Version de contrat possedee, commune a l'ingestion et au resume.
    pub contract_version: String,
    /// Jour produit ISO auquel le resume appartient.
    pub local_day: String,
    /// Titre francais possede, masque par le `Debug` manuel.
    pub title: String,
    /// Resume francais possede, destine aux sorties apres les gates ulterieures.
    pub summary_text: String,
    /// Narration francaise possedee, future entree du worker Piper existant.
    pub narration_text: String,
    /// Nombre total d'items utilises, preuve compacte verifiee au callback.
    pub input_item_count: usize,
    /// Nombre d'items `high`, recalcule depuis les entrees typees.
    pub input_high_item_count: usize,
    /// Identifiants possedes des items `high`, tries lors de la validation de coherence.
    pub input_high_item_ids: Vec<i64>,
}

// Le resume masque volontairement les textes dans les traces `Debug` du runtime natif.
// Cette implementation manuelle garantit que les contenus IA ne reapparaissent pas via `{:?}`.
impl fmt::Debug for CalendarSummaryDraft {
    // L'emprunt du formatter reste local et interdit de conserver une reference vers le draft apres l'appel.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Le builder manuel choisit une allowlist de champs plutot que le derive `Debug` exhaustif.
        formatter
            .debug_struct("CalendarSummaryDraft")
            // Version et jour permettent le diagnostic de correlation sans afficher les trois textes IA.
            .field("contract_version", &self.contract_version)
            // Le jour est le dernier identifiant textuel autorise dans cette representation redigee.
            .field("local_day", &self.local_day)
            // Les deux compteurs prouvent la couverture du resume sans recopier les items d'entree.
            .field("input_item_count", &self.input_item_count)
            // Le compteur high est copie comme scalaire et ne permet pas de reconstruire les titres sources.
            .field("input_high_item_count", &self.input_high_item_count)
            // `finish_non_exhaustive` signale volontairement que des champs sensibles ont ete omis.
            .finish_non_exhaustive()
    }
}

/// Erreurs redigees du pipeline pur. Aucun contenu source/IA n'est embarque.
// `thiserror::Error` genere l'affichage; `PartialEq/Eq` permet de tester les variantes sans texte libre.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CalendarOrchestrationError {
    // Chaque attribut `error` est un message statique ou compose de scalaires, jamais d'une entree sensible.
    #[error("calendar local day is invalid")]
    InvalidLocalDay,
    #[error("calendar source is not authorized for native collection")]
    UnsupportedSource,
    #[error("calendar source snapshot does not contain the target local day")]
    TargetDayMissing,
    #[error("calendar source coverage metadata is inconsistent")]
    SourceCoverageMismatch,
    #[error("calendar covered source binding is invalid")]
    InvalidCoveredSourceBinding,
    #[error("calendar covered source binding is duplicated")]
    DuplicateCoveredSourceBinding,
    #[error("calendar item provenance is missing")]
    MissingItemProvenance,
    #[error("calendar item provenance source is unknown")]
    UnknownItemProvenance,
    #[error("calendar item provenance source was not covered")]
    UncoveredItemProvenance,
    #[error("calendar item provenance URL was supplied by the model")]
    ModelItemProvenanceUrl,
    #[error("calendar item provenance metadata conflicts")]
    ConflictingItemProvenance,
    #[error("calendar item provenance metadata is unsafe")]
    UnsafeItemProvenanceMetadata,
    #[error("calendar model output is empty")]
    EmptyModelOutput,
    #[error("calendar model output is invalid JSON")]
    InvalidModelJson,
    #[error("calendar model output does not match the ingestion contract")]
    InvalidIngestionShape,
    #[error("calendar ingestion contains no canonical item")]
    EmptyIngestion,
    #[error("calendar contract limit exceeded for {kind}")]
    // `&'static str` limite `kind` a une etiquette compilee dans le binaire plutot qu'a du contenu utilisateur.
    ContractLimitExceeded { kind: &'static str },
    #[error("calendar item {item_index} is missing required field {field}")]
    MissingVisibleField {
        // L'index numerique aide le diagnostic sans recopier l'item fautif.
        item_index: usize,
        // Le nom de champ statique ne peut pas contenir la reponse IA.
        field: &'static str,
    },
    #[error("calendar item {item_index} contains non-French field {field}")]
    NonFrenchVisibleField {
        item_index: usize,
        field: &'static str,
    },
    #[error("calendar item {item_index} contains an invalid RFC3339 event time")]
    InvalidEventTime { item_index: usize },
    #[error("calendar item {item_index} does not belong to the target local day")]
    ItemOutsideTargetDay { item_index: usize },
    #[error("calendar stage day does not match its inputs")]
    StageDayMismatch,
    #[error("calendar summary contains no input item")]
    EmptySummaryInput,
    #[error("calendar summary item count mismatch: expected={expected}, received={received}")]
    SummaryItemCountMismatch { expected: usize, received: usize },
    #[error("calendar summary high count mismatch: expected={expected}, received={received}")]
    SummaryHighCountMismatch { expected: usize, received: usize },
    #[error("calendar summary high ids mismatch")]
    SummaryHighIdsMismatch,
    #[error("calendar summary contradicts the available high items")]
    SummaryClaimsNoHigh,
    #[error("calendar canonical serialization failed")]
    CanonicalSerialization,
}

/// Parse, normalise, valide puis dedoublonne la sortie IA d'ingestion.
// Les trois arguments sont empruntes: cette fonction pure ne prend possession ni des snapshots ni du texte IA.
pub fn canonicalize_ingestion_output(
    local_day: &str,
    snapshots: &[CalendarSourceSnapshot],
    model_output: &str,
) -> Result<CalendarIngestionDraft, CalendarOrchestrationError> {
    crate::features::calendar_logs::sync_operation("canonicalize_ingestion_output", || {
        // La date est validee avant toute lecture de snapshot ou parsing potentiellement plus couteux.
        validate_local_day(local_day)?;
        // `str::len` mesure les octets sans parcourir ni allouer la reponse complete.
        if model_output.len() > MAX_MODEL_OUTPUT_BYTES {
            return Err(CalendarOrchestrationError::ContractLimitExceeded {
                kind: "model_output_bytes",
            });
        }
        // `iter().any` court-circuite au premier snapshot couvert sans cloner le tableau ni son texte prive.
        if !snapshots.iter().any(|snapshot| {
            snapshot.local_day == local_day
                && snapshot.target_day_found
                && !snapshot.prompt_text().is_empty()
        }) {
            return Err(CalendarOrchestrationError::TargetDayMissing);
        }
        if model_output.trim().is_empty() {
            return Err(CalendarOrchestrationError::EmptyModelOutput);
        }

        // Le `Value` permissif ne vit que dans cette frontiere; `?` redige tout echec de parsing final.
        let parsed = parse_maybe_calendar_json(model_output)?;
        // Cette chaine d'`Option` emprunte successivement l'objet puis son tableau `items` sans copie.
        let items = parsed
            .as_object()
            .and_then(|object| object.get("items"))
            .and_then(Value::as_array)
            // `ok_or(...)?` transforme une absence structurelle en erreur metier typee puis la propage.
            .ok_or(CalendarOrchestrationError::InvalidIngestionShape)?;
        if items.is_empty() {
            return Err(CalendarOrchestrationError::EmptyIngestion);
        }
        if items.len() > MAX_CALENDAR_ITEMS {
            return Err(CalendarOrchestrationError::ContractLimitExceeded {
                kind: "calendar_items",
            });
        }

        // Le resultat final et la table d'index sont mutables uniquement pendant cette boucle locale.
        let mut canonical_items: Vec<CalendarItemDraft> = Vec::new();
        // `HashMap` offre une recherche moyenne O(1); son ordre n'affecte jamais la sortie canonique.
        let mut identity_indexes: HashMap<String, usize> = HashMap::new();
        // `enumerate` fournit l'index zero-based; `index + 1` produit un diagnostic humain one-based.
        for (index, raw_item) in items.iter().enumerate() {
            // `raw_item` est emprunte au DOM; le normaliseur retourne un item Rust entierement possede.
            let mut item = normalize_item(local_day, raw_item, index + 1)?;
            // L'identite emprunte l'item avant son deplacement eventuel dans le vecteur canonique.
            let identity = item_identity(local_day, &item)?;
            // `get` emprunte la cle; `copied` extrait le petit index `usize` hors de la table.
            if let Some(existing_index) = identity_indexes.get(&identity).copied() {
                // Un doublon editorial remplace ses champs visibles selon la precedence
                // finale, mais unit les provenances des deux sources plutot que de
                // laisser le dernier item effacer l'autre couverture.
                merge_canonical_item_sources(
                    &mut item.sources,
                    &canonical_items[existing_index].sources,
                )?;
                canonical_items[existing_index] = item;
            } else {
                // La `String` d'identite est deplacee dans la map; sa position est capturee avant le `push`.
                identity_indexes.insert(identity, canonical_items.len());
                // L'item possede est deplace dans le vecteur, sans clone de ses textes/provenances.
                canonical_items.push(item);
            }
        }

        // La table de deduplication locale est abandonnee; seule la sequence stable devient le contrat retourne.
        Ok(CalendarIngestionDraft {
            contract_version: CALENDAR_NATIVE_CONTRACT_VERSION.to_owned(),
            local_day: local_day.to_owned(),
            items: canonical_items,
        })
    })
}

/// Unit les provenances de deux items editoriaux equivalentes dans un ordre
/// deterministe. La validation stricte (source couverte, URL canonique,
/// metadonnees compactes) intervient ensuite lors de la projection native.
fn merge_canonical_item_sources(
    current: &mut Vec<CalendarItemSourceDraft>,
    previous: &[CalendarItemSourceDraft],
) -> Result<(), CalendarOrchestrationError> {
    let mut merged: BTreeMap<CalendarSourceKind, CalendarItemSourceDraft> = BTreeMap::new();
    for source in previous.iter().chain(current.iter()) {
        // L'item le plus recent conserve la precedence editoriale pour une
        // meme source, tandis que les provenances de sources distinctes sont
        // toutes retenues jusqu'a la projection typee.
        merged.insert(source.source.clone(), source.clone());
    }
    if merged.len() > MAX_ITEM_SOURCES {
        return Err(CalendarOrchestrationError::ContractLimitExceeded {
            kind: "item_sources",
        });
    }
    *current = merged.into_values().collect();
    Ok(())
}

/// Parse et valide la sortie IA de resume avec les compteurs/IDs derives des items types.
// Le slice d'entree est emprunte pour proteger les items persistants de toute mutation par cette etape pure.
pub fn canonicalize_summary_output(
    local_day: &str,
    input_items: &[CalendarSummaryInputItem],
    model_output: &str,
) -> Result<CalendarSummaryDraft, CalendarOrchestrationError> {
    crate::features::calendar_logs::sync_operation("canonicalize_summary_output", || {
        validate_local_day(local_day)?;
        if model_output.len() > MAX_MODEL_OUTPUT_BYTES {
            return Err(CalendarOrchestrationError::ContractLimitExceeded {
                kind: "model_output_bytes",
            });
        }
        if input_items.is_empty() {
            return Err(CalendarOrchestrationError::EmptySummaryInput);
        }
        if input_items.len() > MAX_CALENDAR_ITEMS {
            return Err(CalendarOrchestrationError::ContractLimitExceeded {
                kind: "calendar_items",
            });
        }

        // `to_vec` clone volontairement les petits items afin de les trier sans muter l'ordre fourni par l'appelant.
        let mut ordered_items = input_items.to_vec();
        // `sort_by` reemploie le fallback `created_at` sans cloner les chaines.
        ordered_items.sort_by(|left, right| {
            left.sort_time()
                .cmp(right.sort_time())
                // `then` n'evalue la comparaison des IDs que lorsque les horaires sont egaux.
                .then(left.id.cmp(&right.id))
        });
        // Ce vecteur contient des references vers `ordered_items`, pas des copies de titres ou d'items.
        let high_items = ordered_items
            .iter()
            .filter(|item| item.final_importance == CalendarImportance::High)
            .collect::<Vec<_>>();
        // `i64` est `Copy`: seuls les identifiants deviennent un nouveau vecteur possede dans le draft.
        let input_high_item_ids = high_items.iter().map(|item| item.id).collect::<Vec<_>>();

        // Un JSON de resume invalide devient un objet vide local, ce qui active les fallbacks de la baseline.
        let parsed = parse_simple_json(model_output).unwrap_or_else(|| Value::Object(Map::new()));
        // `as_object` fournit une `Option<&Map>` empruntee utilisee par toutes les lectures suivantes.
        let object = parsed.as_object();
        // Les combinateurs d'`Option` parcourent l'alias puis convertissent le scalaire sans `unwrap` panique.
        let title = object
            .and_then(|value| alias_value(value, &["title"]))
            .and_then(value_as_text)
            .unwrap_or_else(|| format!("Calendrier Filfinance du {local_day}"));
        // Le fallback est construit seulement une fois et devient possede par le resume s'il est necessaire.
        let fallback_summary = if high_items.is_empty() {
            "Aucun evenement high du calendrier economique n est disponible pour le moment."
                .to_owned()
        } else {
            // L'iterateur emprunte les items importants puis construit uniquement les lignes de texte finales.
            high_items
                .iter()
                .map(|item| {
                    format!(
                        "{}: {}",
                        item.event_time.as_deref().unwrap_or("horaire non precise"),
                        item.translated_title
                    )
                })
                // Un `Vec<String>` intermediaire permet le `join` avec un separateur de lignes explicite.
                .collect::<Vec<_>>()
                .join("\n")
        };
        // `unwrap_or` deplace le fallback deja construit uniquement si aucun texte valide n'est present.
        let summary_text = object
            .and_then(|value| alias_value(value, &["summaryText", "summary_text"]))
            .and_then(value_as_text)
            .unwrap_or(fallback_summary);
        // `unwrap_or_else` retarde l'allocation de la narration par defaut jusqu'a son besoin reel.
        let narration_text = object
            .and_then(|value| alias_value(value, &["narrationText", "narration_text"]))
            .and_then(value_as_text)
            .unwrap_or_else(|| format!("{title}. {summary_text}"));

        // Les `String` locales sont deplacees dans le draft; elles ne sont ni clonees ni gardees ailleurs.
        let draft = CalendarSummaryDraft {
            contract_version: CALENDAR_NATIVE_CONTRACT_VERSION.to_owned(),
            local_day: local_day.to_owned(),
            title,
            summary_text,
            narration_text,
            input_item_count: ordered_items.len(),
            input_high_item_count: high_items.len(),
            input_high_item_ids,
        };
        // Le draft est emprunte pour sa gate finale; `?` empeche de retourner un resume incoherent.
        validate_summary_consistency(input_items, &draft)?;
        Ok(draft)
    })
}

/// Valide les preuves `high` transmises entre les etapes summary et callback.
// Le `Result<(), _>` indique un controle sans valeur de succes, uniquement une erreur redigee possible.
pub fn validate_summary_consistency(
    input_items: &[CalendarSummaryInputItem],
    summary: &CalendarSummaryDraft,
) -> Result<(), CalendarOrchestrationError> {
    crate::features::calendar_logs::sync_operation("validate_summary_consistency", || {
        if summary.input_item_count != input_items.len() {
            return Err(CalendarOrchestrationError::SummaryItemCountMismatch {
                expected: input_items.len(),
                received: summary.input_item_count,
            });
        }
        // Le vecteur attendu est possede localement car il doit etre trie sans modifier les items appeles.
        let mut expected_ids = input_items
            .iter()
            .filter(|item| item.final_importance == CalendarImportance::High)
            .map(|item| item.id)
            .collect::<Vec<_>>();
        // `sort_unstable` evite le cout de stabilite puisque seuls les ensembles d'identifiants sont compares.
        expected_ids.sort_unstable();
        if summary.input_high_item_count != expected_ids.len() {
            return Err(CalendarOrchestrationError::SummaryHighCountMismatch {
                expected: expected_ids.len(),
                received: summary.input_high_item_count,
            });
        }
        // Le clone est volontaire pour trier la preuve recue sans muter le summary emprunte.
        let mut received_ids = summary.input_high_item_ids.clone();
        received_ids.sort_unstable();
        if received_ids != expected_ids {
            return Err(CalendarOrchestrationError::SummaryHighIdsMismatch);
        }
        // La recherche textuelle n'est utile que si des items `high` existent reellement dans les entrees.
        if !expected_ids.is_empty()
            && summary_claims_no_high(&format!(
                "{}\n{}",
                summary.summary_text, summary.narration_text
            ))
        {
            return Err(CalendarOrchestrationError::SummaryClaimsNoHigh);
        }
        // L'unite `()` signifie que tous les invariants ont ete verifies sans produire de nouvelle donnee.
        Ok(())
    })
}

// Cette frontiere convertit un objet JSON permissif en un item Rust complet.
fn normalize_item(
    // Le jour et le DOM sont empruntes; seul le resultat canonique possedera ses chaines.
    local_day: &str,
    value: &Value,
    // L'index est passe par valeur car `usize` est `Copy` et sert seulement aux erreurs redigees.
    item_index: usize,
) -> Result<CalendarItemDraft, CalendarOrchestrationError> {
    crate::features::calendar_logs::sync_operation("normalize_item", || {
        // `as_object` ne copie pas le JSON; `ok_or(...)?` rejette immediatement une forme non objet.
        let object = value
            .as_object()
            .ok_or(CalendarOrchestrationError::InvalidIngestionShape)?;
        // Le titre source accepte les alias historiques et alloue son fallback seulement s'ils sont tous absents.
        let source_title = text_alias(object, &["sourceTitle", "source_title", "title"])
            .unwrap_or_else(|| "Evenement calendrier".to_owned());
        // Le `?` garantit que tout item retourne possede deja un titre francais valide.
        let translated_title = required_french_text(
            object,
            &["translatedTitle", "translated_title"],
            item_index,
            "translatedTitle",
        )?;
        // Le texte TTS traverse la meme gate avant de pouvoir atteindre Piper dans une phase ulterieure.
        let tts_text =
            required_french_text(object, &["ttsText", "tts_text"], item_index, "ttsText")?;
        // `Option<String>` possede le raisonnement s'il existe; `as_deref` le reemprunte ensuite en `Option<&str>`.
        let raw_reasoning = text_alias(object, &["reasoning"]);
        let reasoning = normalize_reasoning(raw_reasoning.as_deref());
        if reasoning.is_empty() {
            return Err(CalendarOrchestrationError::MissingVisibleField {
                item_index,
                field: "reasoning",
            });
        }
        if !is_french_visible_text(&reasoning) {
            return Err(CalendarOrchestrationError::NonFrenchVisibleField {
                item_index,
                field: "reasoning",
            });
        }
        if normalize_language_text(&source_title) == normalize_language_text(&translated_title)
            && looks_english(&source_title)
        {
            return Err(CalendarOrchestrationError::NonFrenchVisibleField {
                item_index,
                field: "translatedTitle",
            });
        }

        // Les enums `Copy` permettent d'utiliser chaque importance comme fallback sans clone ni mouvement genant.
        let source_importance = CalendarImportance::parse(
            alias_value(object, &["sourceImportance", "source_importance"]),
            CalendarImportance::Medium,
        );
        let ai_importance = CalendarImportance::parse(
            alias_value(object, &["aiImportance", "ai_importance"]),
            source_importance,
        );
        let declared_final = CalendarImportance::parse(
            alias_value(object, &["finalImportance", "final_importance"]),
            ai_importance,
        );
        let final_importance = final_importance(source_importance, ai_importance, declared_final);

        // `Number(...)` accepte notamment `null`, `false` et une chaine vide comme zero en JavaScript.
        let confidence = object
            .get("confidence")
            .and_then(js_number_as_f64)
            .unwrap_or(0.5)
            .clamp(0.0, 1.0);
        // La provenance est convertie en contrats Rust possedes; `?` propage tout depassement de borne.
        let sources = normalize_sources(alias_value(object, &["sources"]))?;
        // L'horaire reste optionnel et possede; son contenu est reemprunte seulement pour le parsing.
        let event_time = text_alias(object, &["eventTime", "event_time"]);
        // `if let` n'entre dans la validation RFC3339 que lorsque l'`Option` contient une chaine.
        if let Some(event_time) = event_time.as_deref() {
            // `map_err` remplace l'erreur technique du crate `time` par le contrat redige du pipeline.
            let parsed = OffsetDateTime::parse(event_time, &Rfc3339)
                .map_err(|_| CalendarOrchestrationError::InvalidEventTime { item_index })?;
            // Le jour produit est Europe/Paris, quelle que soit la zone declaree par la source.
            let event_local_day = crate::shared::time::europe_paris_day_for_utc(parsed)
                .ok_or(CalendarOrchestrationError::InvalidEventTime { item_index })?;
            // Cette seconde validation produit la date cible typee et reutilise la regle ouvrable.
            let target_day = validate_local_day(local_day)?;
            if event_local_day != target_day {
                return Err(CalendarOrchestrationError::ItemOutsideTargetDay { item_index });
            }
        }

        // Les variables possedees sont deplacees dans l'item final; les champs optionnels sont lus une seule fois.
        Ok(CalendarItemDraft {
            event_time,
            source_title,
            translated_title,
            tts_text,
            country: text_alias(object, &["country"]),
            currency: text_alias(object, &["currency"]),
            forecast_value: text_alias(object, &["forecastValue", "forecast_value"]),
            previous_value: text_alias(object, &["previousValue", "previous_value"]),
            actual_value: text_alias(object, &["actualValue", "actual_value"]),
            source_importance,
            ai_importance,
            final_importance,
            category: CalendarCategory::parse(alias_value(object, &["category"])),
            reasoning,
            confidence,
            sources,
        })
    })
}

/// Regle canonique: une importance source ou IA `high` domine toute valeur declaree.
// Les trois enums sont `Copy`: le passage par valeur rend cette fonction pure et triviale a appeler.
pub fn final_importance(
    source: CalendarImportance,
    ai: CalendarImportance,
    declared: CalendarImportance,
) -> CalendarImportance {
    // La condition conserve la priorite de securite; le `else` garde la decision declaree sinon.
    if source == CalendarImportance::High || ai == CalendarImportance::High {
        CalendarImportance::High
    } else {
        declared
    }
}

// Les trois champs visibles partagent le meme rejet redige et indexe.
fn required_french_text(
    // La map et la liste d'alias sont empruntees: ce validateur ne conserve aucune reference en sortie.
    object: &Map<String, Value>,
    aliases: &[&str],
    item_index: usize,
    // Le champ est statique pour interdire qu'un texte modele soit embarque dans l'erreur.
    field: &'static str,
) -> Result<String, CalendarOrchestrationError> {
    // `ok_or(...)?` convertit l'absence d'alias utile en erreur indexee, sans `panic`.
    let value = text_alias(object, aliases)
        .ok_or(CalendarOrchestrationError::MissingVisibleField { item_index, field })?;
    if !is_french_visible_text(&value) {
        return Err(CalendarOrchestrationError::NonFrenchVisibleField { item_index, field });
    }
    // La `String` validee est deplacee vers l'appelant, qui en devient proprietaire.
    Ok(value)
}

// Les traductions de secours corrigent les marqueurs anglais les plus frequents.
fn normalize_reasoning(value: Option<&str>) -> String {
    // Le `let ... else` extrait une tranche non vide ou retourne tot une `String` vide possedee.
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return String::new();
    };
    // Une minuscule possedee sert aux recherches sans alterer la casse du texte final d'origine.
    let normalized = value.to_ascii_lowercase();
    if normalized.contains("actual significantly above forecast") {
        return "Le chiffre publie est nettement superieur aux previsions, ce qui signale une demande plus forte.".to_owned();
    }
    if normalized.contains("actual above forecast") {
        return "Le chiffre publie est superieur aux previsions.".to_owned();
    }
    if normalized.contains("actual significantly below forecast") {
        return "Le chiffre publie est nettement inferieur aux previsions, ce qui signale une demande plus faible.".to_owned();
    }
    if normalized.contains("actual below forecast") {
        return "Le chiffre publie est inferieur aux previsions.".to_owned();
    }
    if normalized.contains("stronger demand") {
        return "Le signal pointe vers une demande plus forte.".to_owned();
    }
    if normalized.contains("weaker demand") {
        return "Le signal pointe vers une demande plus faible.".to_owned();
    }
    // Le tableau compile les marqueurs anglais; les iterateurs court-circuitent a la premiere correspondance.
    if [
        "actual",
        "forecast",
        "indicating",
        "demand",
        "stronger",
        "weaker",
        "above",
        "below",
        "significantly",
    ]
    .iter()
    .any(|token| normalized.split_whitespace().any(|part| part == *token))
    {
        return "Le detail economique a ete normalise; surveiller l ecart entre le chiffre publie et les attentes.".to_owned();
    }
    // En l'absence de marqueur connu, le texte nettoye est copie car la sortie doit survivre a l'emprunt.
    value.trim().to_owned()
}

// Le tableau de provenance est borne avant toute allocation du vecteur canonique.
fn normalize_sources(
    // Une `Option<&Value>` represente le champ absent sans cloner le DOM complet.
    value: Option<&Value>,
) -> Result<Vec<CalendarItemSourceDraft>, CalendarOrchestrationError> {
    // La chaine d'`Option` n'accepte qu'un tableau non vide; le fallback conserve une source inconnue tracee.
    let values = value
        .and_then(Value::as_array)
        .filter(|values| !values.is_empty())
        // Le clone rend ce petit tableau local possede avant sa conversion; aucune reference n'echappe ensuite.
        .cloned()
        .unwrap_or_else(|| vec![Value::Null]);
    if values.len() > MAX_ITEM_SOURCES {
        return Err(CalendarOrchestrationError::ContractLimitExceeded {
            kind: "item_sources",
        });
    }
    // `map` retourne un `Result` par source; `collect` s'arrete au premier `Err` et sinon construit le `Vec`.
    values
        .iter()
        .enumerate()
        .map(|(index, value)| normalize_source(value, index + 1))
        .collect()
}

// Les variantes string/objet restent acceptees, puis `raw` devient un enum Rust.
fn normalize_source(
    // Le `Value` brut est emprunte pendant la conversion et ne figure jamais dans le type retourne.
    value: &Value,
    // Le prefixe `_` documente que l'index est reserve au diagnostic futur sans avertissement compilateur.
    _source_index: usize,
) -> Result<CalendarItemSourceDraft, CalendarOrchestrationError> {
    crate::features::calendar_logs::sync_operation("normalize_source", || {
        // `if let Some` traite d'abord la forme chaine, la plus compacte du contrat historique.
        if let Some(source) = value.as_str() {
            return Ok(CalendarItemSourceDraft {
                source: parse_source_kind(Some(source))?,
                external_id: None,
                source_url: None,
                raw_title: None,
                raw: CalendarCompactValue::Null,
            });
        }
        // Pour les autres formes, `Option<&Map>` maintient l'absence explicite d'objet sans allocation.
        let object = value.as_object();
        let source = object
            .and_then(|value| text_alias(value, &["source", "name"]))
            .unwrap_or_else(|| "unknown".to_owned());
        // Cet objet vide possede localement fournit une reference valide lorsque l'entree n'est pas un objet.
        let empty_raw = Value::Object(Map::new());
        // `raw_value` est toujours une reference: la selection ne duplique pas encore le sous-arbre JSON.
        let raw_value = object
            .and_then(|value| value.get("raw"))
            .filter(|value| js_truthy(value))
            .unwrap_or_else(|| if object.is_some() { value } else { &empty_raw });
        // La serialisation temporaire mesure exactement les octets JSON avant toute conversion recursive.
        let raw_size = serde_json::to_vec(raw_value)
            .map_err(|_| CalendarOrchestrationError::CanonicalSerialization)?
            .len();
        if raw_size > MAX_RAW_SOURCE_BYTES {
            return Err(CalendarOrchestrationError::ContractLimitExceeded {
                kind: "source_raw_bytes",
            });
        }
        // Apres la gate de taille, le DOM emprunte devient un enum recursif entierement possede et ordonne.
        let raw = CalendarCompactValue::from_json(raw_value);
        // Chaque champ textuel est extrait en `String`; le resultat ne depend plus de `value` ni `empty_raw`.
        Ok(CalendarItemSourceDraft {
            source: parse_source_kind(Some(&source))?,
            external_id: object
                .and_then(|value| text_alias(value, &["externalId", "external_id", "id"])),
            source_url: object
                .and_then(|value| text_alias(value, &["sourceUrl", "source_url", "url"])),
            raw_title: object
                .and_then(|value| text_alias(value, &["rawTitle", "raw_title", "title"])),
            raw,
        })
    })
}

fn parse_source_kind(
    value: Option<&str>,
) -> Result<CalendarSourceKind, CalendarOrchestrationError> {
    value
        .and_then(CalendarSourceKind::from_name)
        .ok_or(CalendarOrchestrationError::UnknownItemProvenance)
}

/// Verifie que l'URL fournie par le collecteur est bien la page fermee de la source pour ce jour.
///
/// Les parametres sont compares dans leur ordre canonique pour interdire les credentials, fragments
/// ou marqueurs arbitraires dans les URLs qui pourront etre rendues par les routes existantes.
fn is_canonical_collector_url(local_day: &str, source: &CalendarSourceKind, url: &Url) -> bool {
    if validate_local_day(local_day).is_err() {
        return false;
    }
    if url.scheme() != "https"
        || url.username() != ""
        || url.password().is_some()
        || url.port().is_some()
        || url.fragment().is_some()
    {
        return false;
    }

    match source {
        CalendarSourceKind::Feed(source) => return url.as_str() == source.endpoint(),
    }
}

// L'identite reprend les champs editoriaux de l'upsert SQLite. Les sources ne
// la composent pas: deux fournisseurs peuvent decrire le meme evenement et
// leurs provenances doivent alors etre unies, non ecrasees.
fn item_identity(
    // Les entrees sont empruntees pendant la serialisation; aucun item n'est clone pour calculer l'identite.
    local_day: &str,
    item: &CalendarItemDraft,
) -> Result<String, CalendarOrchestrationError> {
    // Le derive local n'expose ce type technique qu'a cette fonction et genere sa serialisation deterministe.
    #[derive(Serialize)]
    // La forme camelCase reste coherente avec le reste des contrats hashes.
    #[serde(rename_all = "camelCase")]
    // `'a` impose que toutes les references de l'identite ne survivent pas aux donnees qu'elles empruntent.
    struct Identity<'a> {
        // Les tranches et references evitent de recopier tous les champs textuels pour un simple hash.
        local_day: &'a str,
        event_time: &'a Option<String>,
        title: &'a str,
        country: &'a Option<String>,
        currency: &'a Option<String>,
        forecast: &'a Option<String>,
        previous: &'a Option<String>,
        actual: &'a Option<String>,
    }
    // La structure temporaire ne possede rien; `canonical_hash` la consomme par emprunt avant sa disparition.
    canonical_hash(&Identity {
        local_day,
        event_time: &item.event_time,
        title: &item.translated_title,
        country: &item.country,
        currency: &item.currency,
        forecast: &item.forecast_value,
        previous: &item.previous_value,
        actual: &item.actual_value,
    })
}

// Le generique accepte chaque contrat Serde sans repasser par un `Value` non type.
// `T: Serialize` est une borne compile-time: seuls les contrats dont la forme est explicite peuvent etre hashes.
fn canonical_hash<T: Serialize>(value: &T) -> Result<String, CalendarOrchestrationError> {
    // `to_vec` produit les octets JSON possedes; `map_err` redige toute erreur technique Serde.
    let bytes = serde_json::to_vec(value)
        .map_err(|_| CalendarOrchestrationError::CanonicalSerialization)?;
    // Le slice `&bytes` est emprunte pendant le hash, puis le tampon temporaire est libere.
    Ok(sha256_hex(&bytes))
}

// SHA-256 produit un identifiant stable sans rendre le contenu visible dans les traces ordinaires.
fn sha256_hex(bytes: &[u8]) -> String {
    // `mut` est requis par l'API incrementale du digest, mais le hacheur reste strictement local.
    let mut hasher = Sha256::new();
    // Le slice d'octets est lu sans copie; aucune entree n'est stockee dans la `String` de sortie.
    hasher.update(bytes);
    // `finalize` consomme le hacheur et `format!` possede la representation hexadecimale finale.
    format!("{:x}", hasher.finalize())
}

// Le parseur reproduit l'ordre direct, fence puis objet equilibre de la baseline.
fn parse_maybe_calendar_json(value: &str) -> Result<Value, CalendarOrchestrationError> {
    crate::features::calendar_logs::sync_operation("parse_maybe_calendar_json", || {
        // `trim` retourne un `&str` emprunte et evite d'allouer une copie seulement pour retirer les marges.
        let text = value.trim();
        // Le pattern `if let` extrait le JSON direct tout en laissant la strategie suivante traiter `None`.
        if let Some(parsed) = try_parse_json(text)
            && is_js_truthy_json(&parsed)
        {
            return Ok(parsed);
        }

        // Le litteral statique est reemploye pour borner les blocs Markdown sans regex ni allocation.
        let fence = "```";
        // `remainder` est une tranche mutable: on deplace la fenetre de lecture, jamais les octets d'origine.
        let mut remainder = text;
        // La baseline garde le premier JSON parseable non-calendrier rencontre dans une fence.
        let mut first_parsed = None;
        // `while let` poursuit tant qu'une fence existe dans la tranche restante.
        while let Some(start) = remainder.find(fence) {
            // Le slicing reste sur des indices ASCII de fence, donc toujours sur des frontieres UTF-8 valides.
            let after_start = &remainder[start + fence.len()..];
            // `let ... else` abandonne la recherche de fences lorsqu'aucune fermeture n'est disponible.
            let Some(end) = after_start.find(fence) else {
                break;
            };
            // Seule la reference candidate est reassignee; le texte du modele demeure immuable.
            let mut candidate = after_start[..end].trim();
            if candidate
                .get(..4)
                .is_some_and(|prefix| prefix.eq_ignore_ascii_case("json"))
            {
                candidate = candidate[4..].trim();
            }
            // Le `let` chaine parsing et verite JavaScript avant d'appliquer la priorite de la baseline.
            if let Some(parsed) = try_parse_json(candidate)
                && is_js_truthy_json(&parsed)
            {
                if is_calendar_payload(&parsed) {
                    return Ok(parsed);
                }
                // `get_or_insert` conserve la priorite de la toute premiere fence parseable.
                first_parsed.get_or_insert(parsed);
            }
            // La prochaine iteration emprunte uniquement la suffixe encore non inspectee.
            remainder = &after_start[end + fence.len()..];
        }

        // Le premier candidat precede le fallback d'objets equilibres; cet ordre est contractuel.
        if let Some(parsed) = first_parsed {
            return Ok(parsed);
        }

        // Le dernier fallback parcourt des tranches equilibrees sans copier la reponse complete.
        let mut first_parsed = None;
        for candidate in balanced_json_objects(text) {
            if let Some(parsed) = try_parse_json(candidate)
                && is_js_truthy_json(&parsed)
            {
                if is_calendar_payload(&parsed) {
                    return Ok(parsed);
                }
                // Le fallback baseline ne rend un objet non-calendrier qu'apres avoir cherche tous les payloads.
                first_parsed.get_or_insert(parsed);
            }
        }
        first_parsed.ok_or(CalendarOrchestrationError::InvalidModelJson)
    })
}

// Le predicat centralise la seule forme racine utile sans deserialiser un second contrat concurrent.
fn is_calendar_payload(value: &Value) -> bool {
    value
        .as_object()
        .and_then(|object| object.get("items"))
        .and_then(Value::as_array)
        .is_some()
}

// JavaScript ignore les valeurs JSON falsy dans les branches `if (parsed)` du parseur canonique.
fn is_js_truthy_json(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_f64().is_some_and(|number| number != 0.0),
        Value::String(value) => !value.is_empty(),
        // Comme en JavaScript, tableaux et objets restent truthy, meme lorsqu'ils sont vides.
        Value::Array(_) | Value::Object(_) => true,
    }
}

// `Option` distingue ici un candidat inutilisable d'une erreur metier finale redigee.
fn try_parse_json(value: &str) -> Option<Value> {
    // `.ok()` efface volontairement le detail Serde pour permettre la reparation controlee suivante.
    serde_json::from_str(value).ok().or_else(|| {
        // La `String` reparee n'est allouee que lorsque le parsing direct a echoue.
        let repaired = repair_missing_item_boundary(value);
        // `then` ne reparsse que si le texte a change; `flatten` ramene `Option<Option<Value>>` a un niveau.
        (repaired != value)
            .then(|| serde_json::from_str(&repaired).ok())
            .flatten()
    })
}

// Le resume ne cherche qu'un JSON direct ou entoure d'une unique fence.
fn parse_simple_json(value: &str) -> Option<Value> {
    let fence = "```";
    // La tranche locale est reassignee pour retirer les fences sans muter ni recopier l'entree.
    let mut text = value.trim();
    if text
        .get(..7)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("```json"))
    {
        text = text[7..].trim();
    } else if text.starts_with(fence) {
        text = text[fence.len()..].trim();
    }
    if text.ends_with(fence) {
        text = text[..text.len() - fence.len()].trim();
    }
    // Une erreur devient `None` afin que l'appelant active les fallbacks de resume bornes.
    serde_json::from_str(text).ok()
}

// Cette reparation cible l'unique frontiere mal formee explicitement toleree par le contrat.
fn repair_missing_item_boundary(value: &str) -> String {
    // La vue en octets permet de reconnaitre la sentinelle ASCII tout en preservant les caracteres UTF-8.
    let bytes = value.as_bytes();
    // La capacite anticipe la petite accolade ajoutee et limite les reallocations du buffer de sortie.
    let mut output = String::with_capacity(value.len() + 4);
    // `cursor` est mutable car la boucle consomme soit un caractere UTF-8, soit la sentinelle complete.
    let mut cursor = 0;
    while cursor < bytes.len() {
        // La reconnaissance tolere des espaces entre `}]` et la virgule de la frontiere cassee.
        if bytes[cursor..].starts_with(b"}]") {
            // `comma` avance sur les espaces sans encore consommer le texte original.
            let mut comma = cursor + 2;
            while comma < bytes.len() && bytes[comma].is_ascii_whitespace() {
                comma += 1;
            }
            // La suite n'est inspectee que si une virgule separe bien deux candidats item.
            if bytes.get(comma) == Some(&b',') {
                // Les espaces entre cette virgule et l'objet item suivant sont aussi acceptes.
                let mut probe = comma + 1;
                while probe < bytes.len() && bytes[probe].is_ascii_whitespace() {
                    probe += 1;
                }
                // Le prochain item doit commencer par une accolade, eventTime restant sa sentinelle.
                if bytes.get(probe) == Some(&b'{') {
                    probe += 1;
                    // Les espaces internes emis par certains modeles sont sans valeur semantique.
                    while probe < bytes.len() && bytes[probe].is_ascii_whitespace() {
                        probe += 1;
                    }
                    if bytes[probe..].starts_with(b"\"eventTime\"") {
                        probe += b"\"eventTime\"".len();
                        // Les espaces avant les deux-points de la cle sont aussi acceptes.
                        while probe < bytes.len() && bytes[probe].is_ascii_whitespace() {
                            probe += 1;
                        }
                        if bytes.get(probe) == Some(&b':') {
                            // L'accolade manquante ferme l'item precedent; le texte suivant est preserve.
                            output.push_str("}]},");
                            // Cette affectation consomme `}]`, les marges et la virgule deja re-emise.
                            cursor = comma + 1;
                            continue;
                        }
                    }
                }
            }
        }
        // Hors reparation, `chars` extrait un caractere complet; l'`expect` repose sur la borne de boucle.
        let character = value[cursor..].chars().next().expect("cursor is in bounds");
        output.push(character);
        cursor += character.len_utf8();
    }
    output
}

// Les tranches empruntees evitent de cloner toute la reponse IA pendant la recherche.
fn balanced_json_objects(value: &str) -> Vec<&str> {
    // Les indices sont calcules en octets parce que les delimitateurs JSON recherches sont ASCII.
    let bytes = value.as_bytes();
    // Le vecteur borne possede seulement des references liees a la duree de vie implicite de `value`.
    let mut candidates = Vec::new();
    // L'automate redemarre a chaque accolade si le candidat precedent ne se ferme pas.
    let mut cursor = 0usize;
    let mut inspected = 0usize;
    while cursor < bytes.len()
        && candidates.len() < MAX_BALANCED_JSON_CANDIDATES
        && inspected < MAX_BALANCED_JSON_SCAN_STEPS
    {
        // La recherche ne tient volontairement pas compte d'une quote ouverte avant le candidat.
        let Some(relative_start) = bytes[cursor..].iter().position(|byte| *byte == b'{') else {
            break;
        };
        let start = cursor + relative_start;
        let mut depth = 0usize;
        let mut in_string = false;
        let mut escaped = false;
        let mut end = None;

        for (relative_index, byte) in bytes[start..].iter().copied().enumerate() {
            if inspected >= MAX_BALANCED_JSON_SCAN_STEPS {
                break;
            }
            inspected += 1;
            if in_string {
                if escaped {
                    escaped = false;
                } else if byte == b'\\' {
                    escaped = true;
                } else if byte == b'"' {
                    in_string = false;
                }
                continue;
            }
            if byte == b'"' {
                in_string = true;
            } else if byte == b'{' {
                depth += 1;
            } else if byte == b'}' {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    end = Some(start + relative_index);
                    break;
                }
            }
        }

        if let Some(end) = end {
            candidates.push(&value[start..=end]);
            // Ce saut ignore les accolades internes apres la fermeture d'un candidat.
            cursor = end + 1;
        } else {
            // Un candidat incomplet laisse la prochaine accolade tenter une resynchronisation propre.
            cursor = start + 1;
        }
    }
    candidates
}

// Le HTML est reduit et tronque avant d'entrer dans le snapshot partage avec l'IA.
fn compact_source_text(value: &str) -> String {
    // Chaque etape possede sa sortie car la suivante doit pouvoir lire un buffer distinct et immutable.
    let without_script = strip_block(value, "script");
    // `&without_script` est un emprunt temporaire; la premiere `String` est liberee apres cette fonction.
    let without_style = strip_block(&without_script, "style");
    // La capacite reservee limite les reallocations sans modifier la borne finale du prompt.
    let mut plain = String::with_capacity(without_style.len());
    let mut inside_tag = false;
    // `chars` respecte les frontieres Unicode pendant que le petit automate retire les balises.
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
    // Le node baseline borne avec JavaScript, donc en unites UTF-16 et non en octets UTF-8.
    truncate_utf16_units(&normalize_whitespace(&plain), MAX_SOURCE_TEXT_CHARS)
}

// Rust interdit les demi-surrogates: la borne UTF-16 s'arrete avant un scalar
// qui franchirait la limite, tout en reproduisant exactement JS pour les textes
// usuels et en garantissant un `String` toujours valide.
fn truncate_utf16_units(value: &str, max_units: usize) -> String {
    let mut used_units = 0usize;
    value
        .chars()
        .take_while(|character| {
            let character_units = character.len_utf16();
            if used_units + character_units > max_units {
                return false;
            }
            used_units += character_units;
            true
        })
        .collect()
}

// Le retrait script/style reste volontairement textuel et n'essaie pas de contourner un site.
fn strip_block(value: &str, tag: &str) -> String {
    // La copie minuscule sert uniquement aux recherches insensibles a la casse; `value` fournit la sortie exacte.
    let lower = value.to_ascii_lowercase();
    let open = format!("<{tag}");
    let close = format!("</{tag}>");
    let mut output = String::with_capacity(value.len());
    let mut cursor = 0;
    // `while let` avance de bloc en bloc et s'arrete naturellement quand le tag ouvrant disparait.
    while let Some(relative_start) = lower[cursor..].find(&open) {
        let start = cursor + relative_start;
        output.push_str(&value[cursor..start]);
        // Un bloc non ferme consomme prudemment la fin plutot que de reinjecter son contenu dans le prompt.
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

// La couverture exige une date explicite, pas seulement une reponse HTTP reussie.
fn contains_target_day(text: &str, local_day: &str) -> Result<bool, CalendarOrchestrationError> {
    // La normalisation alloue une forme comparable unique pour le texte et toutes les aiguilles.
    let normalized = normalize_language_text(text);
    // `?` propage une date invalide; `any` s'arrete a la premiere forme de date trouvee.
    Ok(target_day_needles(local_day)?
        .iter()
        .any(|needle| normalized.contains(needle)))
}

// Les aiguilles couvrent les formats US, FR, ISO et les noms de mois usuels.
fn target_day_needles(local_day: &str) -> Result<Vec<String>, CalendarOrchestrationError> {
    // Le `Date` type empeche de fabriquer des variantes depuis une date calendaire impossible.
    let date = validate_local_day(local_day)?;
    let year = date.year();
    let month = u8::from(date.month());
    let day = date.day();
    let month_padded = format!("{month:02}");
    let day_padded = format!("{day:02}");
    // Le vecteur est mutable car les variantes textuelles de mois sont ajoutees dans la boucle suivante.
    let mut values = vec![
        local_day.to_owned(),
        format!("{month_padded}/{day_padded}/{year}"),
        format!("{month}/{day}/{year}"),
        format!("{day_padded}/{month_padded}/{year}"),
        format!("{day}/{month}/{year}"),
        format!("{year} {month_padded} {day_padded}"),
    ];
    for month_name in month_names(date.month()) {
        values.push(format!("{month_name} {day}"));
        values.push(format!("{month_name} {day} {year}"));
        values.push(format!("{day} {month_name}"));
        values.push(format!("{day} {month_name} {year}"));
    }
    // Le tri rend l'ordre reproductible et permet a `dedup` de supprimer les doublons adjacents.
    values.sort();
    values.dedup();
    Ok(values
        // `into_iter` consomme les `String` temporaires; aucun clone n'est requis avant leur normalisation.
        .into_iter()
        .map(|value| normalize_language_text(&value))
        .filter(|value| !value.is_empty())
        .collect())
}

// Les tranches statiques n'allouent aucune table de mois par snapshot.
fn month_names(month: Month) -> &'static [&'static str] {
    // Le retour a duree `'static` pointe vers des tableaux compiles dans le binaire et partageables en lecture.
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

// Le parseur strict rejette aussi le week-end, conformement au scheduler existant.
fn validate_local_day(local_day: &str) -> Result<Date, CalendarOrchestrationError> {
    // L'iterateur mutable est consomme composant par composant sans allouer un tableau de segments.
    let mut parts = local_day.split('-');
    // Chaque `and_then` transforme un segment absent ou non numerique en `None`, sans panic de parsing.
    let year = parts.next().and_then(|value| value.parse::<i32>().ok());
    let month = parts.next().and_then(|value| value.parse::<u8>().ok());
    let day = parts.next().and_then(|value| value.parse::<u8>().ok());
    if parts.next().is_some() || local_day.len() != 10 {
        return Err(CalendarOrchestrationError::InvalidLocalDay);
    }
    // Cette variable masque volontairement l'`Option<u8>` precedente par un `Month` valide et type.
    let month = month
        .and_then(|value| Month::try_from(value).ok())
        .ok_or(CalendarOrchestrationError::InvalidLocalDay)?;
    // Les `ok_or(...)?` convertissent les composants absents en une seule erreur de contrat redigee.
    let date = Date::from_calendar_date(
        year.ok_or(CalendarOrchestrationError::InvalidLocalDay)?,
        month,
        day.ok_or(CalendarOrchestrationError::InvalidLocalDay)?,
    )
    .map_err(|_| CalendarOrchestrationError::InvalidLocalDay)?;
    if matches!(date.weekday(), Weekday::Saturday | Weekday::Sunday) {
        return Err(CalendarOrchestrationError::InvalidLocalDay);
    }
    Ok(date)
}

// La duree de vie retournee lie la valeur a l'objet emprunte sans clone intermediaire.
fn alias_value<'a>(object: &'a Map<String, Value>, aliases: &[&str]) -> Option<&'a Value> {
    // L'iterateur d'alias est emprunte; `filter_map` ignore les cles absentes avant le test JavaScript de verite.
    aliases
        .iter()
        .filter_map(|alias| object.get(*alias))
        .find(|value| js_truthy(value))
}

// Chaque alias vide/null cede la place au suivant comme l'operateur `||` JavaScript.
fn text_alias(object: &Map<String, Value>, aliases: &[&str]) -> Option<String> {
    // `find_map` court-circuite au premier scalaire utile et retourne sa `String` possedee.
    aliases
        .iter()
        .find_map(|alias| object.get(*alias).and_then(value_as_text))
}

// Seuls les scalaires deviennent du texte; un objet implicite serait un contrat ambigu.
fn value_as_text(value: &Value) -> Option<String> {
    // Le `match` construit une chaine possedee uniquement pour les variantes scalaires acceptees.
    let value = match value {
        Value::Null => return None,
        // Le clone est necessaire car la `String` retournee doit survivre au `Value` emprunte.
        Value::String(value) => value.clone(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::Array(_) | Value::Object(_) => return None,
    };
    // Le renommage local remplace la chaine brute par sa version nettoyee sans conserver deux noms ambigus.
    let value = value.trim().to_owned();
    (!value.is_empty()).then_some(value)
}

// Les enums passent par `String(value || '').toLowerCase()` et ne retirent pas les marges.
fn value_as_enum_text(value: &Value) -> Option<String> {
    // Les variantes scalaires reproduisent `String(...)`; les conteneurs invalides gardent le fallback.
    match value {
        // `null` est falsy en JavaScript et devient la chaine vide, equivalente a une absence.
        Value::Null => None,
        // Le clone sans `trim` garantit que `" high "` reste volontairement invalide.
        Value::String(value) => Some(value.clone()),
        // Booleens et nombres ne correspondent a aucun enum mais leur conversion reste explicite.
        Value::Bool(value) => Some(value.to_string()),
        Value::Number(value) => Some(value.to_string()),
        // Le schema IA exige un scalaire; tableau/objet activent le fallback canonique.
        Value::Array(_) | Value::Object(_) => None,
    }
}

// Cette petite table reproduit la selection d'alias `camelCase || snake_case`.
fn js_truthy(value: &Value) -> bool {
    // Le `match` exhaustif rend visible toute divergence volontaire avec les coercitions JavaScript.
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_f64().is_some_and(|value| value != 0.0),
        Value::String(value) => !value.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

// Cette coercition reproduit les scalaires utiles de `Number(value)` en JavaScript.
fn js_number_as_f64(value: &Value) -> Option<f64> {
    // Chaque branche reste explicite pour ne pas confondre absence (`None`) et zero JavaScript.
    let number = match value {
        // JavaScript convertit `null` en zero.
        Value::Null => 0.0,
        // JavaScript convertit respectivement `false` et `true` en zero et un.
        Value::Bool(value) => u8::from(*value) as f64,
        // Un nombre JSON doit rester fini avant d'entrer dans le contrat Rust.
        Value::Number(value) => value.as_f64()?,
        // Une chaine vide/blanche vaut zero; une autre chaine doit etre numerique.
        Value::String(value) if value.trim().is_empty() => 0.0,
        Value::String(value) => value.trim().parse::<f64>().ok()?,
        // Tableaux et objets ne font pas partie du scalaire de confiance type.
        Value::Array(_) | Value::Object(_) => return None,
    };
    // `Number("NaN")` et les infinis ne doivent jamais atteindre le hash canonique.
    number.is_finite().then_some(number)
}

// Le compactage HTML a besoin d'un espace unique, contrairement aux champs item preserves.
fn normalize_whitespace(value: &str) -> String {
    // `split_whitespace` traite les espaces Unicode; `collect` possede les tranches avant leur `join` final.
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

// Le garde-fou linguistique est volontairement local au contrat afin qu'il soit testable et reutilisable
// sans dependre d'un handler, d'un callback ou d'un autre module d'execution.
fn is_french_visible_text(value: &str) -> bool {
    let normalized = normalize_language_text(value);
    if normalized.len() < 3 {
        return false;
    }
    if language_has_strong_english_signal(&normalized) {
        return false;
    }
    if language_has_french_signal(&normalized) {
        return true;
    }
    has_french_diacritic(value) && !looks_english(value)
}

// Le meme vocabulaire classe un titre identique ou visiblement anglais avant de le rendre public.
fn looks_english(value: &str) -> bool {
    let normalized = normalize_language_text(value);
    language_has_strong_english_signal(&normalized)
        || (language_has_english_tokens(&normalized) && !language_has_french_signal(&normalized))
}

// Cette forme ASCII stable sert a la fois aux controles de langue et aux dates textuelles collectees.
fn normalize_language_text(value: &str) -> String {
    fold_french_ascii(value)
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

// Deux termes francais evitent de classifier une phrase mixte sur un simple mot frequent.
fn language_has_french_signal(normalized: &str) -> bool {
    const FRENCH_TERMS: &[&str] = &[
        "a",
        "affaires",
        "allemand",
        "allemande",
        "annuel",
        "annonce",
        "au",
        "aux",
        "avec",
        "banque",
        "balance",
        "brut",
        "calendrier",
        "central",
        "centrale",
        "chiffre",
        "chinois",
        "chinoise",
        "chomage",
        "commercial",
        "commerciale",
        "communique",
        "compte",
        "confiance",
        "conference",
        "consommateur",
        "consommateurs",
        "consommation",
        "construire",
        "construction",
        "constructions",
        "courant",
        "dans",
        "de",
        "demande",
        "demandeur",
        "demandeurs",
        "demandes",
        "des",
        "detail",
        "decision",
        "directeur",
        "directrice",
        "du",
        "emploi",
        "emprunt",
        "en",
        "entreprises",
        "est",
        "et",
        "euro",
        "europeen",
        "europeenne",
        "evenement",
        "exportations",
        "fixes",
        "francais",
        "francaise",
        "hausse",
        "hebdomadaire",
        "immobilier",
        "immobilisations",
        "importations",
        "indice",
        "industrielle",
        "inflation",
        "inscriptions",
        "investissement",
        "ipc",
        "ipp",
        "italien",
        "italienne",
        "japonais",
        "japonaise",
        "la",
        "le",
        "les",
        "logements",
        "machines",
        "manufacturier",
        "marche",
        "mensuel",
        "monetaire",
        "permis",
        "petrole",
        "pib",
        "politique",
        "pour",
        "ppi",
        "presse",
        "precedent",
        "prevision",
        "prix",
        "proces",
        "production",
        "publication",
        "ralentissement",
        "reunion",
        "resume",
        "services",
        "secteur",
        "stocks",
        "sous",
        "superieur",
        "sur",
        "taux",
        "trimestriel",
        "ventes",
        "verbal",
        "britannique",
        "zone",
    ];

    normalized
        .split_whitespace()
        .filter(|token| FRENCH_TERMS.contains(token))
        .take(2)
        .count()
        >= 2
}

// Les expressions metier completes sont des preuves plus fortes qu'un mot isole en anglais.
fn language_has_strong_english_signal(normalized: &str) -> bool {
    const ENGLISH_PHRASES: &[&str] = &[
        "actual above forecast",
        "actual below forecast",
        "actual significantly above",
        "actual significantly below",
        "building permits",
        "businessnz services index",
        "claimant count",
        "consumer confidence",
        "core machinery orders",
        "crude oil inventories",
        "existing home sales",
        "federal funds rate",
        "fixed asset investment",
        "gdt price index",
        "goods trade balance",
        "housing starts",
        "import prices",
        "industrial production",
        "initial jobless claims",
        "mi leading index",
        "monetary policy statement",
        "nbs press conference",
        "manufacturing pmi",
        "non farm",
        "policy statement",
        "policy rate",
        "press conference",
        "rate statement",
        "retail sales",
        "rightmove hpi",
        "services index",
        "services pmi",
        "trade balance",
        "unemployment claims",
        "unemployment rate",
    ];

    ENGLISH_PHRASES
        .iter()
        .any(|phrase| normalized.contains(phrase))
}

// Les termes isoles ne suffisent qu'en l'absence de signal francais concurrent.
fn language_has_english_tokens(normalized: &str) -> bool {
    const ENGLISH_TOKENS: &[&str] = &[
        "above",
        "actual",
        "below",
        "claims",
        "confidence",
        "demand",
        "employment",
        "forecast",
        "housing",
        "jobless",
        "payroll",
        "previous",
        "retail",
        "sales",
        "significantly",
        "stronger",
        "trade",
        "unemployment",
        "weaker",
    ];

    normalized
        .split_whitespace()
        .any(|token| ENGLISH_TOKENS.contains(&token))
}

// Les diacritiques permettent de reconnaitre un court texte francais ne contenant pas deux mots du lexique.
fn has_french_diacritic(value: &str) -> bool {
    value.chars().any(|character| {
        matches!(
            character,
            'à' | 'â'
                | 'ä'
                | 'ç'
                | 'é'
                | 'è'
                | 'ê'
                | 'ë'
                | 'î'
                | 'ï'
                | 'ô'
                | 'ù'
                | 'û'
                | 'ü'
                | 'À'
                | 'Â'
                | 'Ä'
                | 'Ç'
                | 'É'
                | 'È'
                | 'Ê'
                | 'Ë'
                | 'Î'
                | 'Ï'
                | 'Ô'
                | 'Ù'
                | 'Û'
                | 'Ü'
        )
    })
}

// Une synthese ne peut pas declarer un calendrier calme quand les entrees contiennent un evenement high.
fn summary_claims_no_high(value: &str) -> bool {
    let normalized = fold_french_ascii(value);
    let no_event = normalized.contains("aucun evenement")
        || normalized.contains("aucune annonce")
        || normalized.contains("aucun indicateur")
        || normalized.contains("pas d evenement")
        || normalized.contains("pas d'annonce")
        || normalized.contains("pas d indicateur");
    let high_signal = normalized.contains("fort impact")
        || normalized.contains("forte importance")
        || normalized.contains("high");
    (no_event && high_signal) || normalized.contains("calendrier economique reste calme")
}

// Le pliage conserve une comparaison deterministe des accents sans importer de dependance de translitteration.
fn fold_french_ascii(value: &str) -> String {
    value
        .to_lowercase()
        .chars()
        .map(|character| match character {
            'à' | 'á' | 'â' | 'ä' | 'ã' | 'å' => 'a',
            'ç' => 'c',
            'è' | 'é' | 'ê' | 'ë' => 'e',
            'ì' | 'í' | 'î' | 'ï' => 'i',
            'ò' | 'ó' | 'ô' | 'ö' | 'õ' => 'o',
            'ù' | 'ú' | 'û' | 'ü' => 'u',
            'ý' | 'ÿ' => 'y',
            other => other,
        })
        .collect()
}

// `cfg(test)` exclut tout ce montage du binaire de production et ne le compile que pour `cargo test`.
#[cfg(test)]
mod tests {
    // Le glob importe les contrats prives du module parent afin de tester aussi ses invariants internes.
    use super::*;

    // `include_str!` incorpore la fixture a la compilation: les tests restent hors reseau et reproductibles.
    // `concat!` assemble le chemin compile-time a partir de la racine Cargo, sans dependre du cwd d'execution.
    const VALID_SOURCE: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/calendar_day/source-valid.json"
    ));
    // Cette page syntaxiquement valide omet volontairement la date cible pour exercer la gate de couverture.
    const MISSING_DAY_SOURCE: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/calendar_day/source-missing-day.json"
    ));
    // La variante BEA protege la reconnaissance des dates francaises de la seconde source autorisee.
    const BEA_SOURCE: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/calendar_day/source-bea-valid.json"
    ));
    // La reponse d'ingestion nominale sert de base immuable aux montages et mutations locales des tests.
    const VALID_INGESTION: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/calendar_day/ingestion-valid-response.txt"
    ));
    // Le golden versionne fige la serialisation complete attendue du contrat Rust canonique.
    const VALID_INGESTION_GOLDEN: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/calendar_day/ingestion-valid-golden.json"
    ));
    // Deux items identiques verifient la table de deduplication sans faire varier d'autres champs.
    const DUPLICATE_INGESTION: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/calendar_day/ingestion-duplicate-response.json"
    ));
    // Les champs visibles anglais isolent la gate de langue et la redaction de l'erreur retournee.
    const ENGLISH_INGESTION: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/calendar_day/ingestion-english-response.json"
    ));
    // Cette fixture couvre les alias camel/snake case et les formes chaine/objet des provenances historiques.
    const ALIAS_SOURCES_INGESTION: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/calendar_day/ingestion-alias-sources-response.json"
    ));
    // Une fixture blanche distingue une reponse absente d'un JSON present mais invalide.
    const EMPTY_MODEL_OUTPUT: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/calendar_day/model-output-empty.txt"
    ));
    // Cette entree non JSON exerce l'erreur redigee apres epuisement des parseurs tolerants.
    const INVALID_MODEL_OUTPUT: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/calendar_day/model-output-invalid.txt"
    ));
    // La sortie summary nominale alimente la construction typee avant les mutations d'incoherence.
    const VALID_SUMMARY: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/calendar_day/summary-valid-response.json"
    ));
    // Le golden summary fige textes et preuves quantitatives dans leur forme camelCase publique interne.
    const VALID_SUMMARY_GOLDEN: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/calendar_day/summary-valid-golden.json"
    ));
    // Cette preuve incoherente verifie qu'un payload deserialisable ne contourne pas la validation metier.
    const INCONSISTENT_HIGH_SUMMARY: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/calendar_day/summary-high-inconsistent.json"
    ));

    // `Deserialize` genere le lecteur JSON de test; la fixture possede ses chaines apres le parsing.
    #[derive(Deserialize)]
    // Le renommage reproduit les cles camelCase des donnees publiees tout en gardant les champs Rust idiomatiques.
    #[serde(rename_all = "camelCase")]
    struct SourceFixture {
        // Les `String` sont possedees afin que le snapshot puisse ensuite les emprunter independamment du JSON.
        local_day: String,
        // L'enum custom confirme aussi que sa deserialisation filaire est exercee par chaque fixture source.
        source: CalendarSourceKind,
        url: String,
        // Le code HTTP tient dans `u16` et est copie par valeur vers le constructeur reel.
        status: u16,
        body: String,
    }

    // Le helper oblige chaque scenario source a traverser le constructeur public reel.
    fn snapshot(fixture: &str) -> CalendarSourceSnapshot {
        // Le `&str` compile-time est emprunte; Serde construit ensuite un montage entierement possede.
        let fixture: SourceFixture = serde_json::from_str(fixture).expect("valid source fixture");
        // Les chaines restent empruntees pendant l'appel, tandis que l'enum `source` est deplace dans le snapshot.
        CalendarSourceSnapshot::from_response_text(
            &fixture.local_day,
            fixture.source,
            &fixture.url,
            fixture.status,
            &fixture.body,
        )
        // `expect` est adapte au montage nominal: un echec signale une fixture invalide, pas un cas metier attendu.
        .expect("source snapshot")
    }

    // Ce helper reconstruit l'objet SQLite complet place dans `allItems` et `highItems`.
    fn summary_input(
        id: i64,
        event_time: Option<&str>,
        translated_title: &str,
        final_importance: CalendarImportance,
    ) -> CalendarSummaryInputItem {
        // Chaque chaine possedee permet aux tests de muter ou trier les items sans duree de vie artificielle.
        CalendarSummaryInputItem {
            id,
            local_day: "2026-06-15".to_owned(),
            event_time: event_time.map(str::to_owned),
            source_title: translated_title.to_owned(),
            translated_title: translated_title.to_owned(),
            tts_text: format!("{translated_title}."),
            country: Some("Zone euro".to_owned()),
            currency: Some("EUR".to_owned()),
            forecast_value: Some("2,0 %".to_owned()),
            previous_value: Some("1,9 %".to_owned()),
            actual_value: None,
            source_importance: Some("high".to_owned()),
            ai_importance: Some("medium".to_owned()),
            final_importance,
            category: "macro".to_owned(),
            reasoning: Some("Cette publication peut influencer les marches.".to_owned()),
            confidence: Some(0.9),
            content_hash: format!("fixture-hash-{id}"),
            sources_json: "[]".to_owned(),
            created_at: "2026-06-15T06:00:00Z".to_owned(),
            updated_at: "2026-06-15T06:00:00Z".to_owned(),
        }
    }

    // Ce minifieur de test preserve l'ordre des cles pour reproduire exactement le cas de reparation.
    fn minify_fixture(value: &str) -> String {
        // La capacite initiale egale a l'entree evite les reallocations sans changer le resultat attendu.
        let mut output = String::with_capacity(value.len());
        // Les deux booleens mutables forment l'etat minimal pour ne pas supprimer les espaces dans les chaines.
        let mut in_string = false;
        let mut escaped = false;
        // `chars` respecte l'UTF-8 de la fixture; aucun index d'octet manuel ne peut couper un caractere.
        for character in value.chars() {
            // Dans une chaine JSON, tous les caracteres sont recopies avant de mettre a jour l'etat d'echappement.
            if in_string {
                output.push(character);
                if escaped {
                    escaped = false;
                } else if character == '\\' {
                    escaped = true;
                } else if character == '"' {
                    in_string = false;
                }
            // Hors chaine, un guillemet ouvre une zone dont les espaces doivent etre preserves.
            } else if character == '"' {
                in_string = true;
                output.push(character);
            // Seuls les blancs structuraux hors chaine sont retires pour produire la forme compacte ciblee.
            } else if !character.is_whitespace() {
                output.push(character);
            }
        }
        // La `String` possedee est deplacee vers l'appelant pour ses remplacements de frontiere.
        output
    }

    // Scenario: une page ForexFactory valide est bornee et redigee; risque: fuite via Serde/Debug.
    #[test]
    fn source_snapshot_is_bounded_and_finds_target_day() {
        // Le helper retourne un snapshot possede; le test peut ensuite en emprunter le texte prive plusieurs fois.
        let snapshot = snapshot(VALID_SOURCE);

        // Ces assertions prouvent d'abord les metadonnees de collecte et la borne appliquee avant toute IA.
        assert!(snapshot.ok);
        assert!(snapshot.target_day_found);
        assert!(snapshot.text_length <= MAX_SOURCE_TEXT_CHARS);
        assert!(snapshot.prompt_text().contains("script payload"));
        // Serde emprunte le snapshot; la `String` serialisee permet de chercher une fuite sans le consommer.
        let serialized = serde_json::to_string(&snapshot).expect("snapshot serialization");
        // Le formatage emprunte egalement le snapshot et exerce l'implementation `Debug` redigee manuelle.
        let debug = format!("{snapshot:?}");
        // Les assertions negatives protegent les deux sorties accidentellement journalisables.
        assert!(!serialized.contains("Decision de taux"));
        assert!(!debug.contains("Decision de taux"));
        assert!(debug.contains("text_length"));

        // Les flux autorises restent intacts jusqu'a leur propre parseur type, sans troncature HTML legacy.
        // `repeat` construit un corps possede qui depasse volontairement l'ancienne limite sans dependance externe.
        let oversized_body = format!("15 juin 2026 {}", "x".repeat(MAX_SOURCE_TEXT_CHARS + 100));
        // Le corps est reemprunte: le constructeur produit sa propre copie bornee dans `truncated`.
        let truncated = CalendarSourceSnapshot::from_response_text(
            "2026-06-15",
            CalendarSourceKind::Feed(
                crate::features::calendar_day::native::feeds::CalendarFeedSource::BeaJson,
            ),
            "https://apps.bea.gov/API/signup/release_dates.json",
            200,
            &oversized_body,
        )
        .expect("bounded source snapshot");
        assert_eq!(truncated.text_length, oversized_body.encode_utf16().count());
        assert_eq!(truncated.prompt_text(), oversized_body);
    }

    // Scenario: la page repond mais ne cite pas le jour cible; risque: inventer un calendrier absent.
    #[test]
    fn source_snapshot_without_target_day_is_rejected() {
        // Le snapshot est valide structurellement mais conserve `target_day_found = false` depuis la fixture.
        let snapshot = snapshot(MISSING_DAY_SOURCE);
        // Le clone du jour est necessaire ici car le snapshot est deplace juste apres dans le slice temporaire.
        let error = canonicalize_ingestion_output(
            &snapshot.local_day.clone(),
            &[snapshot],
            VALID_INGESTION,
        )
        .expect_err("missing target day must fail");

        // L'egalite de variante garantit une erreur redigee specifique, et non un faux succes vide.
        assert_eq!(error, CalendarOrchestrationError::TargetDayMissing);
    }

    // Scenario: BEA exprime la date en francais; risque: perdre une source nominale valide.
    #[test]
    fn bea_source_snapshot_finds_french_target_day() {
        // La fixture distincte traverse la meme deserialisation et le meme constructeur que Forex Factory.
        let snapshot = snapshot(BEA_SOURCE);

        // La premiere assertion protege le mapping Serde; la seconde la detection du jour en francais.
        assert_eq!(
            snapshot.source,
            CalendarSourceKind::Feed(
                crate::features::calendar_day::native::feeds::CalendarFeedSource::BeaJson
            )
        );
        assert!(snapshot.target_day_found);
    }

    // Scenario: un samedi respecte le format ISO mais reste hors calendrier; risque: run produit invalide.
    #[test]
    fn weekend_local_day_is_rejected() {
        // Le montage appelle directement le constructeur pour isoler la validation du calendrier ouvrable.
        let error = CalendarSourceSnapshot::from_response_text(
            "2026-06-13",
            CalendarSourceKind::Feed(
                crate::features::calendar_day::native::feeds::CalendarFeedSource::BeaJson,
            ),
            "https://apps.bea.gov/API/signup/release_dates.json",
            200,
            "13 juin 2026",
        )
        .expect_err("weekend must fail");

        // `expect_err` puis l'egalite de variante prouvent que le format ISO seul ne suffit pas.
        assert_eq!(error, CalendarOrchestrationError::InvalidLocalDay);
    }

    // Scenario: la sortie nominale doit rester byte-stable; risque: derive silencieuse du contrat.
    #[test]
    fn valid_ingestion_matches_versioned_golden() {
        // Le snapshot nominal est deplace dans un slice d'un element apres avoir clone son jour pour l'emprunt.
        let snapshot = snapshot(VALID_SOURCE);
        let draft = canonicalize_ingestion_output(
            &snapshot.local_day.clone(),
            &[snapshot],
            VALID_INGESTION,
        )
        .expect("valid ingestion");
        // La serialisation pretty produit une `String` comparable au fichier golden sans muter le draft.
        let actual = serde_json::to_string_pretty(&draft).expect("golden serialization");
        // Le `Debug` redige est teste separement de Serde car ces deux chemins ont des implementations distinctes.
        let debug = format!("{draft:?}");

        // L'egalite textuelle fige schema, noms de champs, ordre canonique et valeurs du contrat versionne.
        assert_eq!(actual.trim(), VALID_INGESTION_GOLDEN.trim());
        // Les recherches negatives empechent les titres et raisonnements d'etre journalises par `Debug`.
        assert!(!debug.contains("Public Sector"));
        assert!(!debug.contains("Emprunt"));
        // Le hash pince aussi la representation compacte independamment du formatage pretty du golden.
        assert_eq!(
            draft.canonical_hash().expect("canonical hash"),
            "fee8c184eb5fc551bf665effcd2c2fbcb32288639fa14612eb94183cfb937b9d"
        );
        // Ces assertions ciblees rendent lisibles les fallbacks/dominances que le golden protege implicitement.
        assert_eq!(draft.items[0].final_importance, CalendarImportance::High);
        assert_eq!(draft.items[0].confidence, 1.0);
        assert_eq!(draft.items[0].category, CalendarCategory::Macro);
    }

    // Scenario: deux items strictement identiques arrivent; risque: doublon SQLite/audio ulterieur.
    #[test]
    fn exact_duplicate_items_are_canonicalized_once() {
        // Le snapshot et la fixture double sont les seules differences avec le scenario nominal.
        let snapshot = snapshot(VALID_SOURCE);
        let draft = canonicalize_ingestion_output(
            &snapshot.local_day.clone(),
            &[snapshot],
            DUPLICATE_INGESTION,
        )
        .expect("duplicate fixture");

        // Le compteur final prouve que la `HashMap` de deduplication replie les identites exactes.
        assert_eq!(draft.items.len(), 1);
    }

    // Scenario: reponse vide, JSON sans item et JSON casse; risque: accepter un faux succes IA.
    #[test]
    fn empty_and_invalid_model_outputs_are_rejected() {
        // Le jour est clone une seule fois pour rester disponible apres les appels qui empruntent le snapshot.
        let source_snapshot = snapshot(VALID_SOURCE);
        let local_day = source_snapshot.local_day.clone();
        // `from_ref` fabrique un slice emprunte sans cloner ni deplacer le snapshot au premier sous-scenario.
        assert_eq!(
            canonicalize_ingestion_output(
                &local_day,
                std::slice::from_ref(&source_snapshot),
                EMPTY_MODEL_OUTPUT,
            )
            .expect_err("empty output"),
            CalendarOrchestrationError::EmptyModelOutput
        );
        // Le second sous-scenario distingue un JSON structurellement valide d'une ingestion effectivement vide.
        assert_eq!(
            canonicalize_ingestion_output(
                &local_day,
                std::slice::from_ref(&source_snapshot),
                r#"{"items":[]}"#,
            )
            .expect_err("empty item array"),
            CalendarOrchestrationError::EmptyIngestion
        );
        // Le dernier appel peut deplacer le snapshot dans le tableau temporaire puisqu'il n'est plus reutilise.
        assert_eq!(
            canonicalize_ingestion_output(&local_day, &[source_snapshot], INVALID_MODEL_OUTPUT)
                .expect_err("invalid output"),
            CalendarOrchestrationError::InvalidModelJson
        );
    }

    // Scenario: les champs visibles restent anglais; risque: exposition produit et Piper non francais.
    #[test]
    fn english_visible_text_is_rejected_without_leaking_it_in_error() {
        // La fixture anglaise est traitee avec une couverture source nominale pour isoler la gate linguistique.
        let snapshot = snapshot(VALID_SOURCE);
        let error = canonicalize_ingestion_output(
            &snapshot.local_day.clone(),
            &[snapshot],
            ENGLISH_INGESTION,
        )
        .expect_err("English visible fields must fail");

        // La variante et l'index identifient exactement le premier champ visible bloque.
        assert_eq!(
            error,
            CalendarOrchestrationError::NonFrenchVisibleField {
                item_index: 1,
                field: "translatedTitle",
            }
        );
        // L'affichage de l'erreur est possede temporairement puis inspecte pour interdire la fuite du titre.
        assert!(!error.to_string().contains("Retail Sales"));
    }

    // Les noms libres issus du modele ne traversent plus le contrat natif.
    #[test]
    fn unknown_source_shapes_are_rejected() {
        let snapshot = snapshot(VALID_SOURCE);
        assert_eq!(
            canonicalize_ingestion_output(
                &snapshot.local_day.clone(),
                &[snapshot],
                ALIAS_SOURCES_INGESTION
            )
            .expect_err("unknown sources must fail closed"),
            CalendarOrchestrationError::UnknownItemProvenance
        );
    }

    #[test]
    fn native_provenance_projection_keeps_only_covered_metadata_and_canonical_url() {
        let html_title_snapshot = snapshot(VALID_SOURCE);
        let local_day = html_title_snapshot.local_day.clone();
        let mut draft =
            canonicalize_ingestion_output(&local_day, &[html_title_snapshot], VALID_INGESTION)
                .expect("valid ingestion draft");
        let binding = CalendarCoveredSourceBinding::for_local_day(
            "2026-06-15",
            CalendarSourceKind::Feed(crate::features::calendar_day::native::feeds::CalendarFeedSource::ForexFactoryExport),
            Url::parse("https://nfs.faireconomy.media/ff_calendar_thisweek.json")
                .expect("valid canonical URL"),
        )
        .expect("valid covered binding");

        // Une URL issue du modele est toujours rejetee avant toute projection.
        assert_eq!(
            project_native_ingestion_provenance(&mut draft, std::slice::from_ref(&binding))
                .expect_err("model source URL must be rejected"),
            CalendarOrchestrationError::ModelItemProvenanceUrl
        );

        // Le caller retire la valeur IA, puis la projection ecrit la seule URL autorisee.
        draft.items[0].sources[0].source_url = None;
        let duplicate = draft.items[0].sources[0].clone();
        draft.items[0].sources.push(duplicate);
        project_native_ingestion_provenance(&mut draft, &[binding])
            .expect("covered provenance projection");
        let sources = &draft.items[0].sources;
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].source, CalendarSourceKind::Feed(crate::features::calendar_day::native::feeds::CalendarFeedSource::ForexFactoryExport));
        assert_eq!(
            sources[0].source_url.as_deref(),
            Some("https://nfs.faireconomy.media/ff_calendar_thisweek.json")
        );
        assert_eq!(
            sources[0].external_id.as_deref(),
            Some("gbp-public-borrowing-2026-06-15")
        );
        assert_eq!(
            sources[0].raw_title.as_deref(),
            Some("GBP Public Sector Net Borrowing")
        );
        assert!(matches!(sources[0].raw, CalendarCompactValue::Null));
    }

    #[test]
    fn native_provenance_projection_rejects_html_challenge_and_oversized_metadata_atomically() {
        let source_snapshot = snapshot(VALID_SOURCE);
        let local_day = source_snapshot.local_day.clone();
        let binding = CalendarCoveredSourceBinding::for_local_day(
            "2026-06-15",
            CalendarSourceKind::Feed(crate::features::calendar_day::native::feeds::CalendarFeedSource::ForexFactoryExport),
            Url::parse("https://nfs.faireconomy.media/ff_calendar_thisweek.json")
                .expect("valid canonical URL"),
        )
        .expect("valid covered binding");

        let mut html_title =
            canonicalize_ingestion_output(&local_day, &[source_snapshot], VALID_INGESTION)
                .expect("valid ingestion draft");
        html_title.items[0].sources[0].source_url = None;
        html_title.items[0].sources[0].raw_title =
            Some("<html>Just a moment... Cloudflare challenge</html>".to_owned());
        let before_html = html_title.clone();
        assert_eq!(
            project_native_ingestion_provenance(&mut html_title, std::slice::from_ref(&binding))
                .expect_err("HTML challenge must not become item metadata"),
            CalendarOrchestrationError::UnsafeItemProvenanceMetadata
        );
        assert_eq!(
            html_title, before_html,
            "a rejected provenance projection must not partially mutate the draft"
        );

        let oversized_snapshot = snapshot(VALID_SOURCE);
        let mut oversized =
            canonicalize_ingestion_output(&local_day, &[oversized_snapshot], VALID_INGESTION)
                .expect("valid ingestion draft");
        oversized.items[0].sources[0].source_url = None;
        oversized.items[0].sources[0].external_id =
            Some("x".repeat(MAX_PERSISTED_SOURCE_EXTERNAL_ID_BYTES + 1));
        assert_eq!(
            project_native_ingestion_provenance(&mut oversized, &[binding])
                .expect_err("oversized external id must not become item metadata"),
            CalendarOrchestrationError::UnsafeItemProvenanceMetadata
        );
    }

    #[test]
    fn multi_source_editorial_duplicate_unites_both_compact_provenances() {
        let mut payload: Value =
            serde_json::from_str(VALID_INGESTION).expect("valid multi-source fixture");
        let mut bea_item = payload["items"][0].clone();
        let mut ff_export_item = bea_item.clone();
        bea_item["sources"] = serde_json::json!([{
            "source": "bea_json",
            "externalId": "bea-ecb-rate-2026-06-15",
            "rawTitle": "ECB Rate Decision",
            "raw": {}
        }]);
        ff_export_item["sources"] = serde_json::json!([{
            "source": "forex_factory_export",
            "externalId": "ff_export-ecb-rate-2026-06-15",
            "rawTitle": "ECB Rate Decision",
            "raw": {}
        }]);
        payload["items"] = Value::Array(vec![bea_item, ff_export_item]);

        let bea_snapshot = CalendarSourceSnapshot::from_response_text(
            "2026-06-15",
            CalendarSourceKind::Feed(
                crate::features::calendar_day::native::feeds::CalendarFeedSource::BeaJson,
            ),
            "https://apps.bea.gov/API/signup/release_dates.json",
            200,
            "15 juin 2026",
        )
        .expect("BEA snapshot must cover the day");
        let ff_export_snapshot = CalendarSourceSnapshot::from_response_text(
            "2026-06-15",
            CalendarSourceKind::Feed(crate::features::calendar_day::native::feeds::CalendarFeedSource::ForexFactoryExport),
            "https://nfs.faireconomy.media/ff_calendar_thisweek.json",
            200,
            "15 juin 2026",
        )
        .expect("ForexFactory snapshot must cover the day");
        let mut draft = canonicalize_ingestion_output(
            "2026-06-15",
            &[bea_snapshot, ff_export_snapshot],
            &payload.to_string(),
        )
        .expect("equivalent events must canonicalize");
        assert_eq!(draft.items.len(), 1);
        assert_eq!(draft.items[0].sources.len(), 2);

        let bea = CalendarCoveredSourceBinding::for_local_day(
            "2026-06-15",
            CalendarSourceKind::Feed(
                crate::features::calendar_day::native::feeds::CalendarFeedSource::BeaJson,
            ),
            Url::parse("https://apps.bea.gov/API/signup/release_dates.json")
                .expect("valid BEA URL"),
        )
        .expect("valid BEA binding");
        let ff_export = CalendarCoveredSourceBinding::for_local_day(
            "2026-06-15",
            CalendarSourceKind::Feed(crate::features::calendar_day::native::feeds::CalendarFeedSource::ForexFactoryExport),
            Url::parse("https://nfs.faireconomy.media/ff_calendar_thisweek.json")
                .expect("valid ForexFactory URL"),
        )
        .expect("valid ForexFactory binding");
        project_native_ingestion_provenance(&mut draft, &[bea, ff_export])
            .expect("both covered provenances must be retained");
        assert_eq!(draft.items[0].sources.len(), 2);
        assert!(matches!(
            draft.items[0].sources.as_slice(),
            [
                CalendarItemSourceDraft {
                    source: CalendarSourceKind::Feed(crate::features::calendar_day::native::feeds::CalendarFeedSource::BeaJson),
                    raw: CalendarCompactValue::Null,
                    ..
                },
                CalendarItemSourceDraft {
                    source: CalendarSourceKind::Feed(crate::features::calendar_day::native::feeds::CalendarFeedSource::ForexFactoryExport),
                    raw: CalendarCompactValue::Null,
                    ..
                }
            ]
        ));
    }

    #[test]
    fn native_provenance_projection_rejects_unknown_or_uncovered_sources_without_partial_mutation()
    {
        let snapshot = snapshot(VALID_SOURCE);
        let local_day = snapshot.local_day.clone();
        let mut draft = canonicalize_ingestion_output(&local_day, &[snapshot], VALID_INGESTION)
            .expect("valid ingestion draft");
        draft.items[0].sources[0].source_url = None;
        let before = draft.clone();
        let ff_export_binding = CalendarCoveredSourceBinding::for_local_day(
            "2026-06-15",
            CalendarSourceKind::Feed(crate::features::calendar_day::native::feeds::CalendarFeedSource::ForexFactoryExport),
            Url::parse("https://nfs.faireconomy.media/ff_calendar_thisweek.json")
                .expect("valid canonical URL"),
        )
        .expect("valid covered binding");

        draft = before;
        draft.items[0].sources[0].source = CalendarSourceKind::Feed(
            crate::features::calendar_day::native::feeds::CalendarFeedSource::BeaJson,
        );
        assert_eq!(
            project_native_ingestion_provenance(&mut draft, &[ff_export_binding])
                .expect_err("uncovered source must be rejected"),
            CalendarOrchestrationError::UncoveredItemProvenance
        );
    }

    #[test]
    fn covered_source_binding_requires_the_closed_date_specific_collector_url() {
        assert_eq!(
            CalendarCoveredSourceBinding::for_local_day(
                "2026-06-15",
                CalendarSourceKind::Feed(
                    crate::features::calendar_day::native::feeds::CalendarFeedSource::BeaJson
                ),
                Url::parse("https://invalid.example/invalid").expect("valid URL syntax"),
            )
            .expect_err("wrong day must be rejected"),
            CalendarOrchestrationError::InvalidCoveredSourceBinding
        );
        assert!(CalendarSourceKind::from_name("partner").is_none());
    }

    // Scenario: importance/categorie contiennent des marges; risque: creer un faux high natif.
    #[test]
    fn importance_and_category_keep_no_trim_semantics() {
        // La fixture nominale est transformee en nouvelle `String`; la constante compile-time reste intacte.
        let snapshot = snapshot(VALID_SOURCE);
        // Les remplacements isolent les deux champs pour distinguer fallback et regle de dominance high.
        let output = VALID_INGESTION
            .replacen(
                "\"sourceImportance\": \"medium\"",
                "\"sourceImportance\": \" high \"",
                1,
            )
            .replacen("\"aiImportance\": \"high\"", "\"aiImportance\": \"low\"", 1)
            .replacen(
                "\"category\": \"unsupported\"",
                "\"category\": \" rates \"",
                1,
            );
        // `from_ref` permet d'emprunter le meme snapshot que son champ `local_day` pendant l'appel.
        let draft = canonicalize_ingestion_output(
            &snapshot.local_day,
            std::slice::from_ref(&snapshot),
            &output,
        )
        .expect("whitespace enum fallbacks");
        // L'emprunt de l'item suffit pour toutes les comparaisons d'enums `Copy` suivantes.
        let item = &draft.items[0];

        // Sans `trim`, le fallback medium/macro est applique et la valeur finale declaree reste low.
        assert_eq!(item.source_importance, CalendarImportance::Medium);
        assert_eq!(item.ai_importance, CalendarImportance::Low);
        assert_eq!(item.final_importance, CalendarImportance::Low);
        assert_eq!(item.category, CalendarCategory::Macro);
    }

    // Scenario: `Number` applique JavaScript aux scalaires falsy; risque: score natif different du contrat.
    #[test]
    fn confidence_js_coercion_and_blank_source_match_contract() {
        // Le snapshot reste possede hors de la boucle et est reemprunte par chaque sous-scenario.
        let snapshot = snapshot(VALID_SOURCE);
        // `null`, `false` et une chaine blanche deviennent tous zero dans le Code node actuel.
        // Le tableau de `Value` est consomme par la boucle; chaque variante est deplacee dans le DOM mutable.
        for confidence in [
            Value::Null,
            Value::Bool(false),
            Value::String("  ".to_owned()),
        ] {
            // Un DOM neuf par iteration evite qu'une mutation de confiance contamine la variante suivante.
            let mut output: Value =
                serde_json::from_str(VALID_INGESTION).expect("valid confidence fixture");
            // L'indexation mutable remplace uniquement le champ cible et transfere le `Value` de la boucle.
            output["items"][0]["confidence"] = confidence;
            let draft = canonicalize_ingestion_output(
                &snapshot.local_day,
                std::slice::from_ref(&snapshot),
                &output.to_string(),
            )
            .expect("JavaScript confidence coercion");
            // Chaque scalaire falsy doit converger vers exactement le meme flottant canonique.
            assert_eq!(draft.items[0].confidence, 0.0);
        }

        // Une propriete absente conserve le fallback canonique a 0,5.
        // Ce second DOM mutable isole l'absence de propriete des valeurs explicitement falsy precedentes.
        let mut without_confidence: Value =
            serde_json::from_str(VALID_INGESTION).expect("valid missing confidence fixture");
        without_confidence["items"][0]
            .as_object_mut()
            .expect("item object")
            // `remove` prend possession de l'ancienne valeur, ignoree ici, et rend le champ vraiment absent.
            .remove("confidence");
        let fallback = canonicalize_ingestion_output(
            &snapshot.local_day,
            std::slice::from_ref(&snapshot),
            &without_confidence.to_string(),
        )
        .expect("missing confidence fallback");
        // L'assertion distingue le fallback d'absence (0,5) de la coercition falsy (0).
        assert_eq!(fallback.items[0].confidence, 0.5);

        // Une provenance chaine blanche est rejetee, sans fallback legacy.
        let mut blank_source: Value =
            serde_json::from_str(VALID_INGESTION).expect("valid blank source fixture");
        blank_source["items"][0]["sources"] = Value::Array(vec![Value::String("   ".to_owned())]);
        let error = canonicalize_ingestion_output(
            &snapshot.local_day,
            std::slice::from_ref(&snapshot),
            &blank_source.to_string(),
        )
        .expect_err("blank source must fail closed");
        assert_eq!(error, CalendarOrchestrationError::UnknownItemProvenance);
    }

    // Scenario: une provenance depasse la taille compacte; risque: memoire/file native non bornee.
    #[test]
    fn oversized_raw_source_is_rejected() {
        // `repeat` construit un contenu deterministe assez grand pour depasser la borne apres emballage JSON.
        let snapshot = snapshot(VALID_SOURCE);
        let oversized_raw = format!("{{\"payload\":\"{}\"}}", "x".repeat(MAX_RAW_SOURCE_BYTES));
        // `replacen` ne touche que le premier objet brut de la fixture et retourne une nouvelle `String`.
        let output = VALID_INGESTION.replacen("{}", &oversized_raw, 1);
        let error =
            canonicalize_ingestion_output(&snapshot.local_day.clone(), &[snapshot], &output)
                .expect_err("oversized raw source");

        // La variante et son etiquette statique prouvent que le rejet vient de la borne de provenance.
        assert_eq!(
            error,
            CalendarOrchestrationError::ContractLimitExceeded {
                kind: "source_raw_bytes",
            }
        );
    }

    // Scenario: l'IA renvoie un evenement du lendemain; risque: polluer le jour cible canonique.
    #[test]
    fn item_outside_target_day_is_rejected() {
        // La nouvelle `String` ne modifie que l'horodatage; tous les autres invariants restent nominaux.
        let snapshot = snapshot(VALID_SOURCE);
        let wrong_day = VALID_INGESTION.replacen("2026-06-15T12:30:00Z", "2026-06-16T12:30:00Z", 1);
        // Le jour est clone avant que le snapshot soit deplace dans le tableau temporaire du dernier appel.
        let error =
            canonicalize_ingestion_output(&snapshot.local_day.clone(), &[snapshot], &wrong_day)
                .expect_err("item from another day");

        // L'index one-based dans l'erreur garantit que le diagnostic cible precisement le premier item.
        assert_eq!(
            error,
            CalendarOrchestrationError::ItemOutsideTargetDay { item_index: 1 }
        );
    }

    // Scenario: minuit UTC differe du jour produit Paris; risque: rejeter/accepter le mauvais run.
    #[test]
    fn event_time_is_checked_in_europe_paris_product_day() {
        // Le meme snapshot est emprunte dans les deux sous-scenarios pour isoler uniquement la conversion de zone.
        let snapshot = snapshot(VALID_SOURCE);
        // A 22 h 30 UTC en ete, Paris est deja le 15 juin: l'item appartient donc au jour cible.
        let paris_target =
            VALID_INGESTION.replacen("2026-06-15T12:30:00Z", "2026-06-14T22:30:00Z", 1);
        // L'absence d'erreur est l'assertion: la date UTC precedente doit devenir le jour cible a Paris.
        canonicalize_ingestion_output(
            &snapshot.local_day,
            std::slice::from_ref(&snapshot),
            &paris_target,
        )
        .expect("UTC previous date is target day in Paris");

        // A 22 h 30 UTC le 15, Paris est le 16: le meme localDay doit refuser cet item.
        let paris_next =
            VALID_INGESTION.replacen("2026-06-15T12:30:00Z", "2026-06-15T22:30:00Z", 1);
        // Le second montage inverse la frontiere locale et conserve l'erreur possedee pour comparaison.
        let error = canonicalize_ingestion_output(
            &snapshot.local_day,
            std::slice::from_ref(&snapshot),
            &paris_next,
        )
        .expect_err("UTC target date is next day in Paris");
        // La meme variante hors jour prouve que la decision utilise le fuseau produit, pas la date texte UTC.
        assert_eq!(
            error,
            CalendarOrchestrationError::ItemOutsideTargetDay { item_index: 1 }
        );
    }

    // Scenario: le JSON valide est entoure de prose/fence; risque: regression du parseur tolerant.
    #[test]
    fn fenced_and_embedded_json_are_supported_by_contract() {
        // `format!` possede la prose et le bloc Markdown tandis que la fixture constante reste empruntee.
        let snapshot = snapshot(VALID_SOURCE);
        let fenced = format!("analyse\n```json\n{}\n```\nfin", VALID_INGESTION.trim());
        // Le snapshot est deplace dans le slice car ce scenario ne le reutilise plus ensuite.
        let draft =
            canonicalize_ingestion_output(&snapshot.local_day.clone(), &[snapshot], &fenced)
                .expect("fenced model output");

        // Le compteur confirme que le parseur a extrait le bloc utile, pas un objet accidentel de la prose.
        assert_eq!(draft.items.len(), 1);
    }

    // Scenario: une fence metadata precede un payload nu; risque: Rust depasse la priorite canonique.
    #[test]
    fn non_calendar_fence_precedes_later_balanced_calendar_by_contract() {
        let snapshot = snapshot(VALID_SOURCE);
        // Le payload valide reste volontairement hors fence: le premier JSON metadata parseable est prioritaire.
        let output = format!(
            "```json\n{{\"analysis\":true}}\n```\n{}",
            VALID_INGESTION.trim()
        );

        let error = canonicalize_ingestion_output(
            &snapshot.local_day,
            std::slice::from_ref(&snapshot),
            &output,
        )
        .expect_err("the first non-calendar fence must keep canonical priority");

        assert_eq!(error, CalendarOrchestrationError::InvalidIngestionShape);
    }

    // Scenario: une fence calendrier suit une fence metadata; risque: rendre le fallback trop tot.
    #[test]
    fn later_calendar_fence_out_ranks_first_non_calendar_fence() {
        let snapshot = snapshot(VALID_SOURCE);
        // Toutes les fences sont inspectees avant de rendre le premier JSON non-calendrier memorise.
        let output = format!(
            "```json\n{{\"analysis\":true}}\n```\n```json\n{}\n```",
            VALID_INGESTION.trim()
        );

        let draft = canonicalize_ingestion_output(
            &snapshot.local_day,
            std::slice::from_ref(&snapshot),
            &output,
        )
        .expect("the later calendar fence must win");

        assert_eq!(draft.items.len(), 1);
    }

    // Scenario: deux objets nus se suivent; risque: rendre le premier avant la recherche calendrier.
    #[test]
    fn balanced_calendar_out_ranks_earlier_non_calendar_object() {
        let snapshot = snapshot(VALID_SOURCE);
        // Sans fence, le premier objet est conserve mais la recherche continue jusqu'au payload calendrier valide.
        let output = format!("{{\"analysis\":true}}\n{}", VALID_INGESTION.trim());

        let draft = canonicalize_ingestion_output(
            &snapshot.local_day,
            std::slice::from_ref(&snapshot),
            &output,
        )
        .expect("the later balanced calendar payload must win");

        assert_eq!(draft.items.len(), 1);
    }

    // Scenario: une enveloppe externe reste ouverte; risque: perdre la resynchronisation interne.
    #[test]
    fn incomplete_outer_object_resynchronizes_on_inner_calendar_payload() {
        let snapshot = snapshot(VALID_SOURCE);
        // La premiere accolade n'a pas de fermeture, mais l'objet calendrier imbrique reste complet.
        let output = format!("{{\"metadata\":{}", VALID_INGESTION.trim());

        let draft = canonicalize_ingestion_output(
            &snapshot.local_day,
            std::slice::from_ref(&snapshot),
            &output,
        )
        .expect("the inner calendar object must be retried by the canonical parser");

        assert_eq!(draft.items.len(), 1);
    }

    // Scenario: la prose ouvre une quote sans la fermer; risque: contaminer l'etat du payload suivant.
    #[test]
    fn unmatched_prose_quote_does_not_mask_balanced_calendar_payload() {
        let snapshot = snapshot(VALID_SOURCE);
        // L'etat de chaine est initialise au debut de chaque accolade candidate.
        let output = format!("\"analyse inachevee\n{}", VALID_INGESTION.trim());

        let draft = canonicalize_ingestion_output(
            &snapshot.local_day,
            std::slice::from_ref(&snapshot),
            &output,
        )
        .expect("a prose quote must not mask the later calendar object");

        assert_eq!(draft.items.len(), 1);
    }

    // Scenario: une reponse contient une longue suite d'objets; risque: allocation de candidats ouverte.
    #[test]
    fn balanced_json_candidate_collection_is_bounded() {
        let output = "{}".repeat(MAX_BALANCED_JSON_CANDIDATES + 1);

        assert_eq!(
            balanced_json_objects(&output).len(),
            MAX_BALANCED_JSON_CANDIDATES
        );
    }

    // Scenario: une accolade item manque apres `sources`; risque: perdre la reparation connue.
    #[test]
    fn malformed_item_boundary_is_repaired_by_contract() {
        // Le minifieur cree une representation deterministe avant de retirer exactement une accolade.
        let snapshot = snapshot(VALID_SOURCE);
        let compact = minify_fixture(DUPLICATE_INGESTION);
        let malformed = compact.replacen("}]},{\"eventTime\"", "}],{\"eventTime\"", 1);
        // Ce garde de montage empeche un test vert si la chaine sentinelle change dans la fixture.
        assert_ne!(malformed, compact, "fixture must remove one item boundary");

        let draft = canonicalize_ingestion_output(
            &snapshot.local_day,
            std::slice::from_ref(&snapshot),
            &malformed,
        )
        .expect("repairable model output");

        // Apres reparation, les deux items identiques doivent encore traverser la deduplication normale.
        assert_eq!(
            draft.items.len(),
            1,
            "repaired duplicates remain deduplicated"
        );

        // La reconnaissance accepte les espaces avant/apres `,`, apres `{` et avant `:`.
        // Ce second `replacen` consomme un emprunt de `compact` mais produit un montage espace distinct possede.
        let spaced = compact.replacen("}]},{\"eventTime\"", "}]   ,  {  \"eventTime\"  ", 1);
        assert_ne!(
            spaced, compact,
            "fixture must add whitespace and remove boundary"
        );
        // Le snapshot reste reempruntable grace a `from_ref`; aucun clone du texte source prive n'est fait.
        let spaced_draft = canonicalize_ingestion_output(
            &snapshot.local_day,
            std::slice::from_ref(&snapshot),
            &spaced,
        )
        .expect("repairable spaced model output");
        // Le resultat identique pince la tolerance exacte des espaces de la reparation corrigee.
        assert_eq!(spaced_draft.items.len(), 1);
    }

    // Scenario: chaque collection accepte sa borne et refuse borne+1; risque: file/memoire ouverte.
    #[test]
    fn collection_and_model_limits_are_enforced_at_the_boundary() {
        // Un seul snapshot nominal est reemprunte pour toutes les bornes afin de ne varier que les tailles.
        let snapshot = snapshot(VALID_SOURCE);

        // Le parseur accepte un payload valide complete par des espaces jusqu'a la borne exacte.
        let trimmed = VALID_INGESTION.trim();
        let at_model_limit = format!(
            "{trimmed}{}",
            " ".repeat(MAX_MODEL_OUTPUT_BYTES - trimmed.len())
        );
        // Cette precondition de montage garantit que le succes suivant exerce bien la valeur limite exacte.
        assert_eq!(at_model_limit.len(), MAX_MODEL_OUTPUT_BYTES);
        canonicalize_ingestion_output(
            &snapshot.local_day,
            std::slice::from_ref(&snapshot),
            &at_model_limit,
        )
        .expect("model output at byte limit");
        // Une espace ASCII ajoute exactement un octet et cree le cas limite + 1 sans invalider le JSON trimme.
        let over_model_limit = format!("{at_model_limit} ");
        assert_eq!(
            canonicalize_ingestion_output(
                &snapshot.local_day,
                std::slice::from_ref(&snapshot),
                &over_model_limit,
            )
            .expect_err("model output over byte limit"),
            CalendarOrchestrationError::ContractLimitExceeded {
                kind: "model_output_bytes",
            }
        );

        // La borne items est testee avant deduplication, comme protection de taille d'entree.
        // Le DOM parse est immuable; seul l'item cible est clone pour construire des vecteurs repetes possedes.
        let parsed: Value = serde_json::from_str(VALID_INGESTION).expect("valid item fixture");
        let item = parsed["items"][0].clone();
        // `vec![item.clone(); N]` requiert `Clone` et cree exactement N valeurs independantes dans le payload.
        let at_item_limit = serde_json::json!({
            "items": vec![item.clone(); MAX_CALENDAR_ITEMS],
        })
        .to_string();
        let bounded_items = canonicalize_ingestion_output(
            &snapshot.local_day,
            std::slice::from_ref(&snapshot),
            &at_item_limit,
        )
        .expect("item count at limit");
        // La longueur canonique vaut un car la borne d'entree precede puis autorise la deduplication exacte.
        assert_eq!(
            bounded_items.items.len(),
            1,
            "exact duplicates are still folded"
        );
        // Le montage limite + 1 reutilise un clone pour garder `item` disponible et expliciter la taille refusee.
        let over_item_limit = serde_json::json!({
            "items": vec![item.clone(); MAX_CALENDAR_ITEMS + 1],
        })
        .to_string();
        assert_eq!(
            canonicalize_ingestion_output(
                &snapshot.local_day,
                std::slice::from_ref(&snapshot),
                &over_item_limit,
            )
            .expect_err("item count over limit"),
            CalendarOrchestrationError::ContractLimitExceeded {
                kind: "calendar_items",
            }
        );

        // La provenance accepte seize entrees, puis bloque la dix-septieme avant construction.
        // Ce DOM devient mutable car le meme champ `sources` est teste successivement a N puis N + 1.
        let mut source_payload: Value =
            serde_json::from_str(VALID_INGESTION).expect("valid source count fixture");
        // L'affectation deplace un tableau possede de N chaines dans le sous-arbre cible.
        source_payload["items"][0]["sources"] =
            Value::Array(vec![Value::String("bea_json".to_owned()); MAX_ITEM_SOURCES]);
        let bounded_sources = canonicalize_ingestion_output(
            &snapshot.local_day,
            std::slice::from_ref(&snapshot),
            &source_payload.to_string(),
        )
        .expect("source count at limit");
        // Le compteur du contrat retourne prouve qu'aucune provenance n'est perdue a la borne autorisee.
        assert_eq!(bounded_sources.items[0].sources.len(), MAX_ITEM_SOURCES);
        // La mutation suivante remplace et libere le tableau precedent par une nouvelle valeur limite + 1.
        source_payload["items"][0]["sources"] =
            Value::Array(vec![
                Value::String("bea_json".to_owned());
                MAX_ITEM_SOURCES + 1
            ]);
        assert_eq!(
            canonicalize_ingestion_output(
                &snapshot.local_day,
                std::slice::from_ref(&snapshot),
                &source_payload.to_string(),
            )
            .expect_err("source count over limit"),
            CalendarOrchestrationError::ContractLimitExceeded {
                kind: "item_sources",
            }
        );
    }

    // Scenario: compte, IDs et texte high divergent; risque: generer un resume/audio mensonger.
    #[test]
    fn summary_high_evidence_must_match_ids_and_text() {
        // Le vecteur possede un item `high` minimal, suffisant pour deriver toutes les preuves attendues.
        let input = vec![summary_input(
            42,
            Some("2026-06-15T12:30:00Z"),
            "Decision de taux de la BCE",
            CalendarImportance::High,
        )];
        // Le summary est mutable uniquement dans le test afin d'injecter une incoherence a la fois.
        let mut summary = canonicalize_summary_output("2026-06-15", &input, VALID_SUMMARY)
            .expect("valid summary");
        // La serialisation emprunte le summary valide avant toute mutation destructive de ses preuves.
        let actual = serde_json::to_string_pretty(&summary).expect("summary golden serialization");
        // Le golden fige le contrat complet; la recherche negative exerce en plus le `Debug` redige.
        assert_eq!(actual.trim(), VALID_SUMMARY_GOLDEN.trim());
        assert!(!format!("{summary:?}").contains("decision de taux"));

        // Chaque bloc corrompt un seul invariant, verifie son erreur exacte, puis restaure l'etat nominal.
        summary.input_item_count = 0;
        assert_eq!(
            validate_summary_consistency(&input, &summary).expect_err("item count mismatch"),
            CalendarOrchestrationError::SummaryItemCountMismatch {
                expected: 1,
                received: 0,
            }
        );
        summary.input_item_count = 1;

        // Le compte high est teste apres restauration du compte global pour isoler sa propre variante.
        summary.input_high_item_count = 0;
        assert_eq!(
            validate_summary_consistency(&input, &summary).expect_err("count mismatch"),
            CalendarOrchestrationError::SummaryHighCountMismatch {
                expected: 1,
                received: 0,
            }
        );
        summary.input_high_item_count = 1;
        // Le nouveau vecteur remplace les IDs possedes sans modifier les entrees de reference empruntees.
        summary.input_high_item_ids = vec![99];
        assert_eq!(
            validate_summary_consistency(&input, &summary).expect_err("id mismatch"),
            CalendarOrchestrationError::SummaryHighIdsMismatch
        );
        summary.input_high_item_ids = vec![42];
        // Une nouvelle `String` contradictoire est deplacee dans le summary apres restauration des IDs valides.
        summary.summary_text =
            "Aucun evenement a fort impact n'est prevu; le calendrier economique reste calme."
                .to_owned();
        assert_eq!(
            validate_summary_consistency(&input, &summary).expect_err("text mismatch"),
            CalendarOrchestrationError::SummaryClaimsNoHigh
        );

        // La fixture externe prouve que la gate reste necessaire meme apres une deserialisation Rust reussie.
        let inconsistent: CalendarSummaryDraft =
            serde_json::from_str(INCONSISTENT_HIGH_SUMMARY).expect("inconsistent high fixture");
        assert_eq!(
            validate_summary_consistency(&input, &inconsistent).expect_err("fixture high mismatch"),
            CalendarOrchestrationError::SummaryHighCountMismatch {
                expected: 1,
                received: 0,
            }
        );
    }

    // Scenario: eventTime manque et createdAt inverse l'ordre des IDs; risque: preuves high differentes du contrat.
    #[test]
    fn summary_canonicalizer_uses_created_at_fallback_before_id() {
        // Le plus petit ID est volontairement cree apres l'autre item.
        let mut late = summary_input(10, None, "Publication tardive", CalendarImportance::High);
        late.created_at = "2026-06-15T10:00:00Z".to_owned();
        // L'item ID 30 doit passer devant grace au fallback `createdAt` du contrat.
        let mut early = summary_input(30, None, "Publication matinale", CalendarImportance::High);
        early.created_at = "2026-06-15T07:00:00Z".to_owned();

        // Une sortie summary simple suffit: l'assertion cible uniquement l'ordre des preuves d'entree.
        let draft = canonicalize_summary_output(
            "2026-06-15",
            &[late, early],
            r#"{"title":"Calendrier","summaryText":"Deux publications importantes sont attendues.","narrationText":"Deux publications importantes sont attendues."}"#,
        )
        .expect("le summary doit etre canonicalise");
        assert_eq!(draft.input_high_item_ids, vec![30, 10]);
    }

    // Scenario: `raw` contient les memes cles dans un autre ordre; risque: hash non deterministe.
    #[test]
    fn canonical_hash_is_stable_when_raw_object_keys_are_reordered() {
        // Deux `String` possedees ne different que par l'ordre lexical des memes cles/valeurs `raw`.
        let snapshot = snapshot(VALID_SOURCE);
        let left_output =
            VALID_INGESTION.replacen("\"raw\": {}", "\"raw\": {\"zeta\": 1, \"alpha\": true}", 1);
        let right_output =
            VALID_INGESTION.replacen("\"raw\": {}", "\"raw\": {\"alpha\": true, \"zeta\": 1}", 1);
        // Les deux canonicalisations reempruntent le meme snapshot et produisent des drafts autonomes.
        let left = canonicalize_ingestion_output(
            &snapshot.local_day,
            std::slice::from_ref(&snapshot),
            &left_output,
        )
        .expect("left raw order");
        let right = canonicalize_ingestion_output(
            &snapshot.local_day,
            std::slice::from_ref(&snapshot),
            &right_output,
        )
        .expect("right raw order");

        // L'egalite des hashes demontre l'effet du `BTreeMap`, independamment de l'ordre JSON d'arrivee.
        assert_eq!(
            left.canonical_hash().expect("left hash"),
            right.canonical_hash().expect("right hash")
        );
    }
}
