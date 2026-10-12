//! Injection d'un run de test depuis le WebSocket de debug.

use axum::extract::ws::{Message, WebSocket};
use futures_util::stream::SplitSink;

use crate::{
    app_state::AppState,
    features::live::{dto::WsTestTraceMessage, test_runs},
};

use super::super::{send_text, serialize_test_trace_message};

/// Injecte un run synthetique dans la queue canonique et trace son admission.
pub(super) async fn trigger_test_run(
    sender: &mut SplitSink<WebSocket, Message>,
    state: &AppState,
) -> Result<(), ()> {
    // `prepare_test_run()` fabrique a la fois l'accuse immediat et l'event brut a injecter.
    let (accepted, event) = test_runs::prepare_test_run();

    if let Some(payload) =
        serialize_test_trace_message(WsTestTraceMessage::test_run_accepted(accepted.clone()))
        && send_text(sender, payload).await.is_err()
    {
        return Err(());
    }

    state.emit_test_trace(
        &accepted.run_id,
        "test_run_requested",
        "info",
        "Signal de test recu depuis le client de debug.",
        None,
    );

    if let Err(error) = state.ingest_tx.send(event).await {
        // Le test doit passer par la meme queue canonique que la prod; on ne bypasse jamais le pipeline.
        state.emit_test_trace(
            &accepted.run_id,
            "test_run_enqueue_failed",
            "error",
            "L'injection du run de test dans le pipeline canonique a echoue.",
            Some(error.to_string()),
        );
    } else {
        state.emit_test_trace(
            &accepted.run_id,
            "test_run_enqueued",
            "ok",
            "Le run de test a ete injecte dans le pipeline canonique.",
            Some(format!("accepted_at={}", accepted.accepted_at)),
        );
    }

    Ok(())
}
