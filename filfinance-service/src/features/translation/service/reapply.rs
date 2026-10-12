//! Retraitement controle des traductions existantes apres mise a jour du glossaire.

use time::{Date, OffsetDateTime};

use crate::{
    app_state::AppState,
    errors::AppError,
    features::{
        glossary::{
            domain::{GlossaryEntry, GlossaryListParams},
            pipeline::{
                apply_display_glossary, apply_source_glossary, glossary_entry_protects_source,
                source_glossary_display_targets,
            },
        },
        news::domain::PersistedNewsEvent,
        translation::{domain::NewsTranslationDraft, provider::TranslationProvider},
    },
    shared::time::europe_paris_day_for_utc,
};

use super::processing::translate_and_store_item_with_glossary;

/// Rapport retourne par la commande de retraitement glossaire.
#[derive(Debug, serde::Serialize)]
pub(crate) struct ReapplyTranslationsReport {
    pub local_day: String,
    pub local_days: Vec<String>,
    pub targeted_total: usize,
    pub reprocessed_total: usize,
    pub display_repaired_total: usize,
    pub errors: Vec<String>,
}

/// Retraduit les news FinancialJuice de la table retenue J/J-1 avec le glossaire actif.
pub(crate) async fn reapply_current_day_translations(
    state: &AppState,
) -> Result<ReapplyTranslationsReport, AppError> {
    let now_utc = OffsetDateTime::now_utc();
    let local_days = target_local_days(now_utc);
    let local_day_strings = local_day_strings(&local_days);
    let glossary_entries = load_active_glossary_entries(state).await?;
    let source_protection_entries = source_protection_glossary_entries(&glossary_entries);
    let items = state
        .news
        .list_news_matching_active_glossary(Some("financial_juice"), 2_000)
        .await?;
    let mut items_to_retranslate = Vec::new();
    let mut provider = TranslationProvider::from_config(&state.config.translation);
    let mut reprocessed_total = 0usize;
    let mut display_repaired_total = 0usize;
    let mut errors = Vec::new();

    for item in items {
        let source_hits = news_event_glossary_hits(&item, &source_protection_entries);

        match repair_existing_glossary_text(
            state,
            &item,
            &glossary_entries,
            &source_protection_entries,
        )
        .await
        {
            Ok(repair) => {
                if repair.repaired {
                    display_repaired_total += 1;
                }
                if source_hits > 0 && !repair.satisfies_source_targets {
                    items_to_retranslate.push(item);
                }
            }
            Err(error) => errors.push(format!(
                "news_event_id {} affichage non repare: {error}",
                item.id
            )),
        }
    }

    for item in &items_to_retranslate {
        if translate_and_store_item_with_glossary(
            state,
            &mut provider,
            item,
            false,
            &glossary_entries,
        )
        .await
        {
            reprocessed_total += 1;
        } else {
            errors.push(format!("news_event_id {} non retraite", item.id));
        }
    }

    Ok(ReapplyTranslationsReport {
        local_day: local_day_strings.first().cloned().unwrap_or_default(),
        local_days: local_day_strings,
        targeted_total: reprocessed_total + display_repaired_total,
        reprocessed_total,
        display_repaired_total,
        errors,
    })
}

async fn load_active_glossary_entries(state: &AppState) -> Result<Vec<GlossaryEntry>, AppError> {
    let entries = state
        .news
        .list_glossary_entries(&GlossaryListParams::active_for_pipeline())
        .await?;
    state
        .translation_runtime
        .record_glossary_loaded(entries.len() as u64);
    state.translation_runtime.clear_glossary_error();
    Ok(entries)
}

fn news_event_glossary_hits(item: &PersistedNewsEvent, entries: &[GlossaryEntry]) -> u64 {
    apply_source_glossary(&item.title, entries).hits
        + item
            .body
            .as_deref()
            .map(|body| apply_source_glossary(body, entries).hits)
            .unwrap_or(0)
}

fn source_protection_glossary_entries(entries: &[GlossaryEntry]) -> Vec<GlossaryEntry> {
    entries
        .iter()
        .filter(|entry| glossary_entry_protects_source(entry))
        .cloned()
        .collect()
}

#[derive(Debug, Clone, Copy)]
struct ExistingTranslationRepair {
    repaired: bool,
    satisfies_source_targets: bool,
}

async fn repair_existing_glossary_text(
    state: &AppState,
    item: &PersistedNewsEvent,
    all_entries: &[GlossaryEntry],
    source_entries: &[GlossaryEntry],
) -> Result<ExistingTranslationRepair, AppError> {
    let Some(translation) = state
        .news
        .get_translation_by_news_event_id_and_language(
            item.id,
            &state.config.translation.target_language,
        )
        .await?
    else {
        return Ok(ExistingTranslationRepair {
            repaired: false,
            satisfies_source_targets: false,
        });
    };

    let targets = news_event_display_targets(item, source_entries);
    let translated_title =
        repair_translation_text(&translation.translated_title, all_entries, &targets);
    let translated_body = translation
        .translated_body
        .as_deref()
        .map(|body| repair_translation_text(body, all_entries, &targets));
    let narration_text =
        repair_translation_text(&translation.narration_text, all_entries, &targets);

    let repaired = translated_title != translation.translated_title
        || translated_body != translation.translated_body
        || narration_text != translation.narration_text;
    let satisfies_source_targets = translation_satisfies_targets(
        &translated_title,
        translated_body.as_deref(),
        &narration_text,
        &targets,
    );

    if !repaired {
        return Ok(ExistingTranslationRepair {
            repaired: false,
            satisfies_source_targets,
        });
    }

    state
        .news
        .upsert_translation(
            item,
            &NewsTranslationDraft {
                language: translation.language,
                status: translation.status,
                translated_title,
                translated_body,
                narration_text,
                translation_mode: translation.translation_mode,
                error_message: translation.error_message,
            },
        )
        .await?;

    Ok(ExistingTranslationRepair {
        repaired: true,
        satisfies_source_targets,
    })
}

fn target_local_days(now_utc: OffsetDateTime) -> Vec<Date> {
    let current_day = europe_paris_day_for_utc(now_utc).unwrap_or_else(|| now_utc.date());
    let mut days = vec![current_day];
    if let Some(previous_day) = current_day.previous_day() {
        days.push(previous_day);
    }
    days
}

fn local_day_strings(local_days: &[Date]) -> Vec<String> {
    local_days.iter().map(ToString::to_string).collect()
}

fn news_event_display_targets(item: &PersistedNewsEvent, entries: &[GlossaryEntry]) -> Vec<String> {
    let mut targets = source_glossary_display_targets(&item.title, entries);
    if let Some(body) = item.body.as_deref() {
        targets.extend(source_glossary_display_targets(body, entries));
    }
    deduplicate_targets(targets)
}

fn repair_translation_text(text: &str, entries: &[GlossaryEntry], targets: &[String]) -> String {
    let repaired = apply_display_glossary(text, entries);
    repair_source_target_artifacts(&repaired, targets)
}

fn repair_source_target_artifacts(text: &str, targets: &[String]) -> String {
    targets.iter().fold(text.to_owned(), |acc, target| {
        repair_bce_possessive_artifact(&acc, target)
    })
}

fn repair_bce_possessive_artifact(text: &str, target: &str) -> String {
    let Some(capture) = target.strip_suffix(" de la BCE") else {
        return text.to_owned();
    };
    let capture = capture.trim();
    if capture.is_empty() {
        return text.to_owned();
    }

    let mut repaired = text.to_owned();
    for artifact in [
        format!("{capture}: de la BCE"),
        format!("{capture} : de la BCE"),
        format!("BCE's {capture}"),
        format!("ECB's {capture}"),
    ] {
        repaired = repaired.replace(&artifact, target);
    }
    repaired
}

fn translation_satisfies_targets(
    translated_title: &str,
    translated_body: Option<&str>,
    narration_text: &str,
    targets: &[String],
) -> bool {
    if targets.is_empty() {
        return true;
    }

    let mut combined = String::with_capacity(translated_title.len() + narration_text.len() + 2);
    combined.push_str(translated_title);
    combined.push('\n');
    if let Some(body) = translated_body {
        combined.push_str(body);
        combined.push('\n');
    }
    combined.push_str(narration_text);

    targets.iter().all(|target| combined.contains(target))
}

fn deduplicate_targets(targets: Vec<String>) -> Vec<String> {
    let mut deduped = Vec::new();
    for target in targets {
        let trimmed = target.trim();
        if trimmed.is_empty() || deduped.iter().any(|existing| existing == trimmed) {
            continue;
        }
        deduped.push(trimmed.to_owned());
    }
    deduped
}
