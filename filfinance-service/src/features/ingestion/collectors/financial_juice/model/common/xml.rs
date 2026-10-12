//! Parsing des enveloppes ASP.NET `.asmx` FinancialJuice.

// `quick_xml` lit les enveloppes XML ASP.NET `.asmx`.
use quick_xml::{Reader, events::Event};

use crate::errors::AppError;

use super::text::decode_xml_entities;

/// Relit le texte contenu dans `<string>...</string>` d'une réponse ASP.NET `.asmx`.
pub(in crate::features::ingestion) fn extract_asmx_string_payload(
    source_name: &str,
    body: &str,
) -> Result<String, AppError> {
    // Les réponses `.asmx` observées sont simples: une enveloppe `<string ...>JSON</string>`.
    // Une extraction manuelle est plus robuste ici qu'un parseur XML streaming qui peut découper
    // le texte en plusieurs events selon le contenu.
    if let Some(start_index) = body.find("<string") {
        let tag_end = body[start_index..]
            .find('>')
            .map(|offset| start_index + offset + 1);
        let end_index = body.rfind("</string>");

        if let (Some(tag_end), Some(end_index)) = (tag_end, end_index)
            && tag_end <= end_index
        {
            let payload = body[tag_end..end_index].trim();
            return Ok(decode_xml_entities(payload));
        }
    }

    // Le fallback XML reste utile si la structure change légèrement à l'avenir.
    let mut reader = Reader::from_str(body);
    reader.config_mut().trim_text(true);
    let mut buffer = Vec::new();
    let mut payload = String::new();

    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Text(text)) => {
                payload.push_str(&String::from_utf8_lossy(text.as_ref()));
            }
            Ok(Event::CData(text)) => {
                payload.push_str(&String::from_utf8_lossy(text.as_ref()));
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(error) => {
                return Err(AppError::parse(
                    source_name.to_owned(),
                    format!("failed to parse FinancialJuice XML wrapper: {error}"),
                ));
            }
        }

        buffer.clear();
    }

    if !payload.trim().is_empty() {
        return Ok(decode_xml_entities(payload.trim()));
    }

    Err(AppError::parse(
        source_name.to_owned(),
        "FinancialJuice XML wrapper did not contain a payload string",
    ))
}
