//! Champs WebForms obligatoires de la page `/home` FinancialJuice.

use crate::errors::AppError;

use super::super::extraction::extract_html_input_value;

/// Extrait un input WebForms requis en conservant les erreurs historiques du parser.
pub(super) fn extract_required_home_input(
    source_name: &str,
    body: &str,
    input_id: &str,
) -> Result<String, AppError> {
    extract_html_input_value(body, input_id).ok_or_else(|| {
        AppError::parse(
            source_name.to_owned(),
            format!("missing {input_id} in FinancialJuice /home"),
        )
    })
}
