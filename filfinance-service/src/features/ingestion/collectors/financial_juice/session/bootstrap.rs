//! Construction du formulaire de login WebForms FinancialJuice.

use super::super::{encode_form_body, model::FinancialJuiceHomePageState};

/// Encode le POST attendu par le formulaire ASP.NET WebForms de FinancialJuice.
pub(super) fn login_form_body(
    initial_home: &FinancialJuiceHomePageState,
    email: &str,
    password: &str,
) -> String {
    // Les noms de champs viennent du site FinancialJuice et restent volontairement inchangés.
    encode_form_body(&[
        (
            "__EVENTTARGET",
            "ctl00$SignInSignUp$loginForm1$btnLogin".to_owned(),
        ),
        ("__EVENTARGUMENT", "".to_owned()),
        ("__VIEWSTATE", initial_home.view_state.clone()),
        (
            "__VIEWSTATEGENERATOR",
            initial_home.view_state_generator.clone(),
        ),
        ("ctl00$SignInSignUp$loginForm1$inputEmail", email.to_owned()),
        (
            "ctl00$SignInSignUp$loginForm1$inputPassword",
            password.to_owned(),
        ),
        ("ctl00$SignInSignUp$loginForm1$cbRemmebrMe", "on".to_owned()),
        ("ctl00$SignInSignUp$loginForm1$btnLogin", "Login".to_owned()),
    ])
}
