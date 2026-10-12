//! Validation de la frame initiale SignalR FinancialJuice.

use futures_util::{Stream, StreamExt};
use tokio_tungstenite::tungstenite::{Error as TungsteniteError, Message};

use crate::errors::AppError;

use super::super::super::signalr::FinancialJuiceSignalrEnvelope;

/// Lit et valide l'accuse d'initialisation attendu avant le `start` HTTP SignalR.
pub(super) async fn read_initial_signalr_frame<S>(websocket: &mut S) -> Result<(), AppError>
where
    S: Stream<Item = Result<Message, TungsteniteError>> + Unpin,
{
    let Some(initial_frame) = websocket.next().await else {
        return Err(AppError::downstream(
            "signalr websocket closed before initialization".to_owned(),
        ));
    };

    match initial_frame {
        Ok(Message::Text(text)) => {
            let envelope: FinancialJuiceSignalrEnvelope = serde_json::from_str(text.as_str())?;
            if envelope.error.is_some() {
                return Err(AppError::downstream(
                    envelope
                        .error
                        .unwrap_or_else(|| "signalr initialization failed".to_owned()),
                ));
            }
        }
        Ok(other) => {
            return Err(AppError::downstream(format!(
                "unexpected initial signalr frame: {other:?}"
            )));
        }
        Err(error) => {
            return Err(AppError::downstream(format!(
                "signalr initialization read failed: {error}"
            )));
        }
    }

    Ok(())
}
