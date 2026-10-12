//! Domaine du glossaire metier de traduction et de vocalisation.

// `Deserialize` lit les payloads admin; `Serialize` renvoie les contrats JSON au client.
use serde::{Deserialize, Serialize};

use crate::{errors::AppError, shared::text::normalize_whitespace};

/// Mode de correspondance supporte par le MVP glossaire.
pub(crate) const MATCH_MODE_EXACT: &str = "exact";
/// Mode de correspondance par limites de mots supporte par le MVP glossaire.
pub(crate) const MATCH_MODE_WORD_BOUNDARY: &str = "word_boundary";
/// Mode de correspondance par motif operateur, avec captures `<number>` et `<string>`.
pub(crate) const MATCH_MODE_PATTERN: &str = "pattern";
/// Portee historique: l'entree agit sur toute la chaine EN -> FR -> vocal.
pub(crate) const GLOSSARY_SCOPE_FULL_PIPELINE: &str = "full_pipeline";
/// Portee vocale: l'entree ne touche que le texte envoye a Piper.
pub(crate) const GLOSSARY_SCOPE_SPOKEN_ONLY: &str = "spoken_only";

/// Entree persistante du glossaire metier.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub(crate) struct GlossaryEntry {
    /// Identifiant technique SQLite.
    pub id: i64,
    /// Terme source anglais recherche avant traduction.
    pub source_term: String,
    /// Terme francais impose dans le texte affiche.
    pub translated_term: String,
    /// Forme francaise longue envoyee a Piper uniquement.
    pub spoken_term: Option<String>,
    /// Langue source supportee par le MVP.
    pub source_language: String,
    /// Langue cible supportee par le MVP.
    pub target_language: String,
    /// Mode de matching applique au terme source.
    pub match_mode: String,
    /// Indique si la casse doit correspondre exactement.
    pub case_sensitive: bool,
    /// Priorite metier, triee de la plus haute a la plus basse.
    pub priority: i64,
    /// Indique si l'entree est active dans le pipeline.
    pub enabled: bool,
    /// Portee fonctionnelle de l'entree.
    pub scope: String,
    /// Note operateur optionnelle.
    pub notes: Option<String>,
    /// Date de creation RFC3339.
    pub created_at: String,
    /// Date de derniere mise a jour RFC3339.
    pub updated_at: String,
}

/// Query string de `GET /api/glossary`.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct GlossaryListQuery {
    /// Filtre optionnel sur les entrees actives.
    pub enabled: Option<bool>,
    /// Recherche texte optionnelle sur source, traduction, forme vocale et notes.
    pub q: Option<String>,
    /// Limite optionnelle bornee cote serveur.
    pub limit: Option<i64>,
}

/// Parametres normalises de lecture glossaire.
#[derive(Debug, Clone)]
pub(crate) struct GlossaryListParams {
    /// Filtre optionnel sur les entrees actives.
    pub enabled: Option<bool>,
    /// Recherche texte deja trimmee.
    pub query: Option<String>,
    /// Limite bornee entre 1 et 500.
    pub limit: i64,
}

impl GlossaryListParams {
    /// Normalise la query string avant de construire du SQL dynamique.
    pub(crate) fn from_query(query: GlossaryListQuery) -> Self {
        // La recherche vide est traitee comme absente pour garder le SQL simple.
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

    /// Parametres utilises par le pipeline pour charger uniquement les entrees actives.
    pub(crate) fn active_for_pipeline() -> Self {
        Self {
            enabled: Some(true),
            query: None,
            limit: 1_000,
        }
    }
}

/// Payload de creation d'une entree glossaire.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct CreateGlossaryEntryRequest {
    /// Terme source anglais.
    pub source_term: String,
    /// Terme francais affiche.
    pub translated_term: String,
    /// Forme vocale optionnelle.
    pub spoken_term: Option<String>,
    /// Langue source.
    pub source_language: String,
    /// Langue cible.
    pub target_language: String,
    /// Mode de correspondance.
    pub match_mode: String,
    /// Correspondance sensible a la casse.
    pub case_sensitive: bool,
    /// Priorite d'application.
    pub priority: i64,
    /// Activation initiale.
    pub enabled: bool,
    /// Portee d'application.
    pub scope: Option<String>,
    /// Note optionnelle.
    pub notes: Option<String>,
}

/// Payload de mise a jour partielle d'une entree glossaire.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct UpdateGlossaryEntryRequest {
    /// Terme source anglais optionnel.
    pub source_term: Option<String>,
    /// Terme francais affiche optionnel.
    pub translated_term: Option<String>,
    /// Forme vocale optionnelle; `null` efface la valeur.
    pub spoken_term: Option<Option<String>>,
    /// Langue source optionnelle.
    pub source_language: Option<String>,
    /// Langue cible optionnelle.
    pub target_language: Option<String>,
    /// Mode de correspondance optionnel.
    pub match_mode: Option<String>,
    /// Sensibilite a la casse optionnelle.
    pub case_sensitive: Option<bool>,
    /// Priorite optionnelle.
    pub priority: Option<i64>,
    /// Activation optionnelle.
    pub enabled: Option<bool>,
    /// Portee optionnelle.
    pub scope: Option<String>,
    /// Note optionnelle; `null` efface la valeur.
    pub notes: Option<Option<String>>,
}

/// Payload normalise pret a inserer.
#[derive(Debug, Clone)]
pub(crate) struct NormalizedCreateGlossaryEntry {
    /// Terme source non vide.
    pub source_term: String,
    /// Terme affiche non vide.
    pub translated_term: String,
    /// Forme vocale trimmee.
    pub spoken_term: Option<String>,
    /// Langue source valide.
    pub source_language: String,
    /// Langue cible valide.
    pub target_language: String,
    /// Mode de matching valide.
    pub match_mode: String,
    /// Sensibilite a la casse.
    pub case_sensitive: bool,
    /// Priorite bornee.
    pub priority: i64,
    /// Activation initiale.
    pub enabled: bool,
    /// Portee normalisee.
    pub scope: String,
    /// Note trimmee.
    pub notes: Option<String>,
}

/// Payload de mise a jour partielle normalise.
#[derive(Debug, Clone)]
pub(crate) struct NormalizedUpdateGlossaryEntry {
    /// Terme source normalise.
    pub source_term: Option<String>,
    /// Terme affiche normalise.
    pub translated_term: Option<String>,
    /// Forme vocale tri-etat.
    pub spoken_term: Option<Option<String>>,
    /// Langue source normalisee.
    pub source_language: Option<String>,
    /// Langue cible normalisee.
    pub target_language: Option<String>,
    /// Mode de matching normalise.
    pub match_mode: Option<String>,
    /// Sensibilite a la casse.
    pub case_sensitive: Option<bool>,
    /// Priorite bornee.
    pub priority: Option<i64>,
    /// Etat actif.
    pub enabled: Option<bool>,
    /// Portee normalisee.
    pub scope: Option<String>,
    /// Note tri-etat.
    pub notes: Option<Option<String>>,
}

/// Requete d'aperçu textuel du pipeline dictionnaire/TTS.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct GlossaryPreviewRequest {
    /// Texte saisi par l'operateur.
    pub input_text: String,
    /// Etape de depart: `source_en` ou `display_fr`.
    pub input_stage: String,
    /// Entree de formulaire optionnelle a tester sans persistence.
    pub draft_entry: Option<CreateGlossaryEntryRequest>,
}

/// Reponse d'aperçu textuel du pipeline dictionnaire/TTS.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct GlossaryPreviewResponse {
    /// Texte affiche apres traduction ou entree FR directe.
    pub display_text: String,
    /// Texte apres application des formes vocales admin.
    pub spoken_text: String,
    /// Texte final envoye a Piper apres normalisation systeme.
    pub normalized_tts_text: String,
    /// Nombre de remplacements glossaire observes.
    pub glossary_hits: u64,
    /// Messages non bloquants pour l'operateur.
    pub warnings: Vec<String>,
}

/// Regle systeme TTS exposee en lecture seule dans l'admin.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct GlossarySystemRule {
    /// Identifiant stable cote client.
    pub id: &'static str,
    /// Famille fonctionnelle.
    pub family: &'static str,
    /// Libelle court.
    pub label: &'static str,
    /// Motif d'entree attendu.
    pub input_pattern: &'static str,
    /// Sortie vocale produite.
    pub output_pattern: &'static str,
    /// Description operateur.
    pub description: &'static str,
    /// Exemples representatifs.
    pub examples: &'static [&'static str],
}

impl CreateGlossaryEntryRequest {
    /// Valide et normalise un payload de creation.
    pub(crate) fn normalize(self) -> Result<NormalizedCreateGlossaryEntry, AppError> {
        let source_term = normalize_required_text("source_term", self.source_term)?;
        let translated_term = normalize_required_text("translated_term", self.translated_term)?;
        let spoken_term = normalize_optional_text(self.spoken_term);
        let match_mode = normalize_match_mode(self.match_mode)?;
        validate_glossary_pattern_capture_terms(
            &match_mode,
            &source_term,
            &translated_term,
            spoken_term.as_deref(),
        )?;

        Ok(NormalizedCreateGlossaryEntry {
            source_term,
            translated_term,
            spoken_term,
            source_language: normalize_language("source_language", self.source_language, "en")?,
            target_language: normalize_language("target_language", self.target_language, "fr")?,
            match_mode,
            case_sensitive: self.case_sensitive,
            priority: normalize_priority(self.priority)?,
            enabled: self.enabled,
            scope: normalize_scope(self.scope)?,
            notes: normalize_optional_text(self.notes),
        })
    }
}

impl UpdateGlossaryEntryRequest {
    /// Valide et normalise un payload de mise a jour.
    pub(crate) fn normalize(self) -> Result<NormalizedUpdateGlossaryEntry, AppError> {
        Ok(NormalizedUpdateGlossaryEntry {
            source_term: normalize_optional_required_text("source_term", self.source_term)?,
            translated_term: normalize_optional_required_text(
                "translated_term",
                self.translated_term,
            )?,
            spoken_term: normalize_optional_patch_text(self.spoken_term),
            source_language: normalize_optional_language(
                "source_language",
                self.source_language,
                "en",
            )?,
            target_language: normalize_optional_language(
                "target_language",
                self.target_language,
                "fr",
            )?,
            match_mode: self.match_mode.map(normalize_match_mode).transpose()?,
            case_sensitive: self.case_sensitive,
            priority: self.priority.map(normalize_priority).transpose()?,
            enabled: self.enabled,
            scope: self
                .scope
                .map(|value| normalize_scope(Some(value)))
                .transpose()?,
            notes: normalize_optional_patch_text(self.notes),
        })
    }
}

impl GlossaryEntry {
    /// Construit une entree ephemere pour l'aperçu sans toucher au stockage.
    pub(crate) fn preview_from_normalized(entry: NormalizedCreateGlossaryEntry) -> Self {
        Self {
            id: 0,
            source_term: entry.source_term,
            translated_term: entry.translated_term,
            spoken_term: entry.spoken_term,
            source_language: entry.source_language,
            target_language: entry.target_language,
            match_mode: entry.match_mode,
            case_sensitive: entry.case_sensitive,
            priority: entry.priority,
            enabled: entry.enabled,
            scope: entry.scope,
            notes: entry.notes,
            created_at: "preview".to_owned(),
            updated_at: "preview".to_owned(),
        }
    }

    /// Indique si l'entree doit agir avant et apres traduction.
    pub(crate) fn applies_to_display_pipeline(&self) -> bool {
        self.scope == GLOSSARY_SCOPE_FULL_PIPELINE
    }
}

/// Normalise un texte obligatoire en refusant la chaine vide.
fn normalize_required_text(field: &str, value: String) -> Result<String, AppError> {
    let normalized = normalize_whitespace(&value);
    if normalized.is_empty() {
        return Err(AppError::Config(format!("{field} cannot be empty")));
    }
    Ok(normalized)
}

/// Normalise un champ optionnel qui reste obligatoire s'il est present.
fn normalize_optional_required_text(
    field: &str,
    value: Option<String>,
) -> Result<Option<String>, AppError> {
    value
        .map(|candidate| normalize_required_text(field, candidate))
        .transpose()
}

/// Normalise un texte optionnel en transformant la chaine vide en absence.
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

/// Normalise un patch optionnel ou `null` conserve un sens explicite.
fn normalize_optional_patch_text(value: Option<Option<String>>) -> Option<Option<String>> {
    value.map(normalize_optional_text)
}

/// Valide une langue MVP stricte.
fn normalize_language(field: &str, value: String, expected: &str) -> Result<String, AppError> {
    let normalized = value.trim().to_ascii_lowercase();
    if normalized != expected {
        return Err(AppError::Config(format!("{field} must be {expected}")));
    }
    Ok(normalized)
}

/// Valide une langue optionnelle.
fn normalize_optional_language(
    field: &str,
    value: Option<String>,
    expected: &str,
) -> Result<Option<String>, AppError> {
    value
        .map(|candidate| normalize_language(field, candidate, expected))
        .transpose()
}

/// Valide le mode de matching supporte.
fn normalize_match_mode(value: String) -> Result<String, AppError> {
    let normalized = value.trim().to_ascii_lowercase();
    match normalized.as_str() {
        MATCH_MODE_EXACT | MATCH_MODE_WORD_BOUNDARY | MATCH_MODE_PATTERN => Ok(normalized),
        _ => Err(AppError::Config(
            "match_mode must be exact, word_boundary or pattern".to_owned(),
        )),
    }
}

/// Valide la portee fonctionnelle de l'entree.
fn normalize_scope(value: Option<String>) -> Result<String, AppError> {
    let normalized = value
        .unwrap_or_else(|| GLOSSARY_SCOPE_FULL_PIPELINE.to_owned())
        .trim()
        .to_ascii_lowercase();
    match normalized.as_str() {
        GLOSSARY_SCOPE_FULL_PIPELINE | GLOSSARY_SCOPE_SPOKEN_ONLY => Ok(normalized),
        _ => Err(AppError::Config(
            "scope must be full_pipeline or spoken_only".to_owned(),
        )),
    }
}

/// Regles de normalisation TTS systeme exposees comme reference non editable.
pub(crate) fn glossary_system_rules() -> Vec<GlossarySystemRule> {
    vec![
        GlossarySystemRule {
            id: "tts-time-hh-mm",
            family: "Horaires",
            label: "HH:MM",
            input_pattern: "14:30",
            output_pattern: "quatorze heures trente",
            description: "Convertit les horaires avec deux-points en lecture francaise.",
            examples: &[
                "09:30 -> neuf heures trente",
                "15:45 -> quinze heures quarante cinq",
            ],
        },
        GlossarySystemRule {
            id: "tts-percentages",
            family: "Marches",
            label: "Pourcentages",
            input_pattern: "+1.5%",
            output_pattern: "plus 1 virgule 5 pour cent",
            description: "Rend les variations signees plus naturelles pour Piper.",
            examples: &[
                "+3% -> plus 3 pour cent",
                "-0.2% -> moins 0 virgule 2 pour cent",
            ],
        },
        GlossarySystemRule {
            id: "tts-basis-points",
            family: "Taux",
            label: "Points de base",
            input_pattern: "+25 bps",
            output_pattern: "plus 25 points de base",
            description: "Traite les unites bp/bps sur deux tokens consecutifs.",
            examples: &[
                "25 bps -> 25 points de base",
                "+10 bp -> plus 10 points de base",
            ],
        },
        GlossarySystemRule {
            id: "tts-fx-pairs",
            family: "Devises",
            label: "Paires FX",
            input_pattern: "EUR/USD",
            output_pattern: "euro dollar",
            description: "Remplace les paires de devises par leurs libelles vocaux.",
            examples: &["EUR/USD -> euro dollar", "GBP/JPY -> livre sterling yen"],
        },
        GlossarySystemRule {
            id: "tts-us-treasury-tenor",
            family: "Obligataire",
            label: "Treasury US",
            input_pattern: "US 10Y",
            output_pattern: "Treasury americain a 10 ans",
            description: "Stabilise la lecture des maturites obligataires americaines.",
            examples: &["US 10Y -> Treasury americain a 10 ans"],
        },
        GlossarySystemRule {
            id: "tts-macro-acronyms",
            family: "Macro",
            label: "Acronymes macro",
            input_pattern: "NFP, CPI, PCE, PMI",
            output_pattern: "rapport emploi americain, C P I, P C E, P M I",
            description: "Evite les lectures mecaniques ou ambigues des sigles macro.",
            examples: &["NFP -> rapport emploi americain", "CPI -> C P I"],
        },
        GlossarySystemRule {
            id: "tts-tickers",
            family: "Actions",
            label: "Tickers",
            input_pattern: "NVDA",
            output_pattern: "N V D A",
            description: "Epelle les codes majuscules restants lettre par lettre.",
            examples: &["NVDA -> N V D A", "TSLA -> T S L A"],
        },
    ]
}

/// Valide qu'un motif glossaire peut repliquer ses captures dans les sorties FR et TTS.
pub(crate) fn validate_glossary_pattern_capture_terms(
    match_mode: &str,
    source_term: &str,
    translated_term: &str,
    spoken_term: Option<&str>,
) -> Result<(), AppError> {
    if match_mode != MATCH_MODE_PATTERN {
        return Ok(());
    }

    let source_captures = pattern_capture_sequence(source_term, false);
    if source_captures.is_empty() {
        return Err(AppError::Config(
            "pattern source_term must contain <number> or <string>".to_owned(),
        ));
    }

    validate_matching_capture_sequence(
        "translated_term",
        &source_captures,
        translated_term,
        false,
    )?;
    if let Some(spoken_term) = spoken_term {
        validate_matching_capture_sequence("spoken_term", &source_captures, spoken_term, true)?;
    }

    Ok(())
}

fn validate_matching_capture_sequence(
    field: &str,
    source_captures: &[GlossaryPatternCapture],
    value: &str,
    allow_number_modifiers: bool,
) -> Result<(), AppError> {
    let captures = pattern_capture_sequence(value, allow_number_modifiers);
    if captures == source_captures {
        return Ok(());
    }

    Err(AppError::Config(format!(
        "pattern {field} captures must match source_term captures"
    )))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GlossaryPatternCapture {
    Number,
    Text,
}

fn pattern_capture_sequence(
    value: &str,
    allow_number_modifiers: bool,
) -> Vec<GlossaryPatternCapture> {
    let mut captures = Vec::new();
    let mut cursor = 0usize;

    while let Some((relative_index, capture, token_len)) =
        find_next_pattern_capture_token(&value[cursor..], allow_number_modifiers)
    {
        captures.push(capture);
        cursor += relative_index + token_len;
    }

    captures
}

fn find_next_pattern_capture_token(
    value: &str,
    allow_number_modifiers: bool,
) -> Option<(usize, GlossaryPatternCapture, usize)> {
    let mut best = capture_token_candidate(value, "<number>", GlossaryPatternCapture::Number);
    if allow_number_modifiers {
        for candidate in [
            capture_token_candidate(value, "<number:hour>", GlossaryPatternCapture::Number),
            capture_token_candidate(value, "<number:minute>", GlossaryPatternCapture::Number),
        ] {
            if candidate.is_some_and(|candidate| {
                best.is_none_or(|current: (usize, GlossaryPatternCapture, usize)| {
                    candidate.0 < current.0
                })
            }) {
                best = candidate;
            }
        }
    }

    let string_match = capture_token_candidate(value, "<string>", GlossaryPatternCapture::Text);
    if string_match.is_some_and(|candidate| {
        best.is_none_or(|current: (usize, GlossaryPatternCapture, usize)| candidate.0 < current.0)
    }) {
        best = string_match;
    }

    best
}

fn capture_token_candidate(
    value: &str,
    placeholder: &str,
    capture: GlossaryPatternCapture,
) -> Option<(usize, GlossaryPatternCapture, usize)> {
    value
        .find(placeholder)
        .map(|index| (index, capture, placeholder.len()))
}

/// Borne la priorite pour eviter les valeurs absurdes depuis l'admin.
fn normalize_priority(value: i64) -> Result<i64, AppError> {
    if !(-10_000..=10_000).contains(&value) {
        return Err(AppError::Config(
            "priority must be between -10000 and 10000".to_owned(),
        ));
    }
    Ok(value)
}
