//! Extraction HTML et JavaScript minimale de la page `/home`.

/// Extrait la valeur d'un input HTML par son `id`.
pub(super) fn extract_html_input_value(body: &str, input_id: &str) -> Option<String> {
    let pattern = format!("id=\"{input_id}\" value=\"");
    extract_after(body, &pattern, "\"")
}

/// Extrait une variable JS simple de type chaine `'...'`.
pub(super) fn extract_js_string(body: &str, variable_name: &str) -> Option<String> {
    [
        format!("var {variable_name}='"),
        format!("var {variable_name} = '"),
        format!("{variable_name}='"),
        format!("{variable_name} = '"),
    ]
    .into_iter()
    .find_map(|pattern| extract_after(body, &pattern, "'"))
}

/// Extrait une variable JS booleenne `true/false`.
pub(super) fn extract_js_bool(body: &str, variable_name: &str) -> Option<bool> {
    [
        format!("var {variable_name}=true"),
        format!("var {variable_name}=false"),
        format!("var {variable_name} = true"),
        format!("var {variable_name} = false"),
        format!("{variable_name}=true"),
        format!("{variable_name}=false"),
    ]
    .into_iter()
    .find_map(|pattern| {
        if body.contains(&pattern) {
            Some(pattern.ends_with("true"))
        } else {
            None
        }
    })
}

/// Extrait une liste JS d'entiers `[...]`.
pub(in crate::features::ingestion) fn extract_js_number_list(
    body: &str,
    variable_name: &str,
) -> Vec<i64> {
    [
        format!("var {variable_name}= ["),
        format!("var {variable_name} = ["),
        format!("{variable_name}= ["),
        format!("{variable_name} = ["),
        format!("var {variable_name}=["),
        format!("{variable_name}=["),
    ]
    .into_iter()
    .find_map(|pattern| extract_after(body, &pattern, "]"))
    .map(|content| {
        content
            .split(',')
            .filter_map(|value| value.trim().parse::<i64>().ok())
            .collect()
    })
    .unwrap_or_default()
}

/// Petit helper string pour chercher une valeur entre deux delimiteurs.
fn extract_after(body: &str, start_pattern: &str, end_pattern: &str) -> Option<String> {
    let start_index = body.find(start_pattern)? + start_pattern.len();
    let remaining = &body[start_index..];
    let end_index = remaining.find(end_pattern)?;
    Some(remaining[..end_index].to_owned())
}
