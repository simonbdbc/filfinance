//! URLs et identifiants externes FinancialJuice.

// `Url` absolutise les chemins FinancialJuice.
use reqwest::Url;

use super::text::non_empty;

/// Base canonique du site public, utile pour les absolutisations d'URL.
const FINANCIAL_JUICE_SITE_URL: &str = "https://www.financialjuice.com/";

/// Convertit un chemin/URL source en URL absolue FinancialJuice.
pub(in crate::features::ingestion::collectors::financial_juice) fn absolutize_url_from_site(
    raw_url: Option<&str>,
) -> Option<String> {
    let raw_url = non_empty(raw_url?)?;

    if let Ok(url) = Url::parse(&raw_url) {
        return Some(url.to_string());
    }

    Url::parse(FINANCIAL_JUICE_SITE_URL)
        .ok()?
        .join(raw_url.trim_start_matches('/'))
        .ok()
        .map(|url| url.to_string())
}
