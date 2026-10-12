//! Helpers de preparation des runs de test E2E injectes dans le pipeline canonique.

// `SystemTime` et `UNIX_EPOCH` suffisent ici pour fabriquer un identifiant lisible sans dependance
// externe comme `uuid`.
use std::time::{SystemTime, UNIX_EPOCH};

// On importe le payload d'accuse live, le modele canonique de news, le prefixe servant a le
// reconnaitre plus tard, et l'helper de timestamp partage.
use crate::features::{
    live::dto::TestRunAcceptedResponse,
    news::domain::{NewsEvent, TEST_RUN_REF_PREFIX},
};
use crate::shared::time::now_rfc3339;

/// Source stable utilisee pour distinguer un evenement synthetique declenche depuis le client.
const TEST_RUN_SOURCE: &str = "test_run";
/// Headline anglaise envoyee telle quelle dans le pipeline pour valider Telegram et WebSocket.
const TEST_RUN_HEADLINE: &str = "Test: Fed futures edge higher while Treasury yields hold steady";
/// Corps optionnel utile pour les debugs HTTP/DB sans alourdir le flux live.
const TEST_RUN_BODY: &str =
    "Synthetic end-to-end validation event triggered manually from the web client.";

/// Prepare un run de test avec son identifiant et son evenement d'ingestion canonique.
pub fn prepare_test_run() -> (TestRunAcceptedResponse, NewsEvent) {
    // L'identifiant est genere d'abord pour etre reutilise dans l'URL, la reference brute et la reponse.
    let run_id = generate_test_run_id();
    // On capture tout de suite l'instant d'acceptation afin que la reponse et l'evenement soient coherents.
    let accepted_at = now_rfc3339();
    // `NewsEvent::new_headline` force le run de test a passer par le meme modele que les vraies news.
    let mut event = NewsEvent::new_headline(
        TEST_RUN_SOURCE,
        TEST_RUN_HEADLINE,
        format!("https://example.test/test-runs/{run_id}"),
        Some(accepted_at.clone()),
        Some(TEST_RUN_BODY.to_owned()),
    );

    // `raw_payload_ref` sert ensuite a rattacher tout le pipeline a ce run de test.
    event.raw_payload_ref = Some(format!("{TEST_RUN_REF_PREFIX}{run_id}"));
    // La signature est recalculee apres mutation pour garder la dedup en phase avec la reference brute.
    event.recompute_signature();

    // Le tuple de retour livre a la fois l'accuse immediat pour le client et l'evenement a ingerer.
    (
        TestRunAcceptedResponse {
            run_id,
            accepted_at,
        },
        event,
    )
}

/// Genere un identifiant de run lisible sans dependance additionnelle.
fn generate_test_run_id() -> String {
    // `SystemTime::now()` lit l'heure systeme courante.
    let elapsed = SystemTime::now()
        // `duration_since` peut theoriquement echouer si l'horloge systeme recule avant 1970.
        .duration_since(UNIX_EPOCH)
        // `unwrap_or_default()` evite de paniquer et remplace ce cas rarissime par `0`.
        .unwrap_or_default();

    // `{:x}` formate le nombre en hexadecimal pour un identifiant compact mais lisible.
    format!("run-{:x}", elapsed.as_nanos())
}
