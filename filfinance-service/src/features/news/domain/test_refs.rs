//! References brutes reservees aux runs de test E2E.

/// Prefixe stable utilise pour rattacher un evenement synthetique a un run de test E2E.
pub const TEST_RUN_REF_PREFIX: &str = "test_run:";

/// Extrait l'identifiant de run de test depuis la reference brute persistee quand elle suit le prefixe attendu.
pub fn extract_test_run_id(value: Option<&str>) -> Option<&str> {
    // `strip_prefix` retourne `None` si le prefixe ne correspond pas, ce qui rend l'extraction sure.
    value
        .and_then(|value| value.strip_prefix(TEST_RUN_REF_PREFIX))
        .map(|run_id| run_id.split(';').next().unwrap_or(run_id))
}
