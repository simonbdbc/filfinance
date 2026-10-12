//! Parsing de l'etat `/home` FinancialJuice.

mod webforms;

use crate::errors::AppError;

use super::{
    bootstrap::FinancialJuiceBootstrap,
    extraction::{extract_js_bool, extract_js_number_list, extract_js_string},
};
use webforms::extract_required_home_input;

/// Etat d'une page `/home`, avant ou apres login.
#[derive(Debug, Clone)]
pub(in crate::features::ingestion) struct FinancialJuiceHomePageState {
    /// `__VIEWSTATE` requis par le POST WebForms.
    pub(in crate::features::ingestion) view_state: String,
    /// `__VIEWSTATEGENERATOR` requis par le POST WebForms.
    pub(in crate::features::ingestion) view_state_generator: String,
    /// Indique si la page reflete un utilisateur reellement connecte.
    pub(in crate::features::ingestion) user_logged_in: bool,
    /// Base `live.financialjuice.com` si la page l'expose.
    pub(in crate::features::ingestion) main_url: Option<String>,
    /// Jeton `info` utilise par les endpoints internes.
    pub(in crate::features::ingestion) info: Option<String>,
    /// `feedtoken` utilise par SignalR.
    pub(in crate::features::ingestion) feedtoken: Option<String>,
    /// IDs de feeds autorises visibles dans le bootstrap JS.
    pub(in crate::features::ingestion) feed_ids: Vec<i64>,
}

impl FinancialJuiceHomePageState {
    /// Convertit un etat `/home` en bootstrap authentifie exploitable.
    pub(in crate::features::ingestion::collectors::financial_juice) fn into_authenticated_bootstrap(
        self,
        source_name: &str,
    ) -> Result<FinancialJuiceBootstrap, AppError> {
        if !self.user_logged_in {
            return Err(AppError::fetch(
                source_name.to_owned(),
                "financial juice session is not authenticated".to_owned(),
            ));
        }

        let main_url = self.main_url.ok_or_else(|| {
            AppError::parse(
                source_name.to_owned(),
                "missing MainURLData in authenticated /home page",
            )
        })?;
        let info = self.info.ok_or_else(|| {
            AppError::parse(
                source_name.to_owned(),
                "missing info token in authenticated /home page",
            )
        })?;
        let feedtoken = self.feedtoken.ok_or_else(|| {
            AppError::parse(
                source_name.to_owned(),
                "missing feedtoken in authenticated /home page",
            )
        })?;

        Ok(FinancialJuiceBootstrap {
            main_url: main_url.trim_end_matches('/').to_owned(),
            info,
            feedtoken,
            feed_ids: self.feed_ids,
        })
    }
}

/// Parse la page `/home` pour en extraire les variables JS et les champs WebForms utiles.
pub(in crate::features::ingestion) fn parse_home_page_state(
    source_name: &str,
    body: &str,
) -> Result<FinancialJuiceHomePageState, AppError> {
    let view_state = extract_required_home_input(source_name, body, "__VIEWSTATE")?;
    let view_state_generator =
        extract_required_home_input(source_name, body, "__VIEWSTATEGENERATOR")?;

    Ok(FinancialJuiceHomePageState {
        view_state,
        view_state_generator,
        user_logged_in: extract_js_bool(body, "UserLoggedIn").unwrap_or(false)
            && extract_js_bool(body, "LoggedUser").unwrap_or(false),
        main_url: extract_js_string(body, "MainURLData"),
        info: extract_js_string(body, "info"),
        feedtoken: extract_js_string(body, "feedtoken"),
        feed_ids: extract_js_number_list(body, "FeedIds"),
    })
}
