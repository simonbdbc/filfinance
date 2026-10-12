//! Diagnostics fermes : aucun texte fourni par une source ou un modele.
use super::openrouter::{
    CalendarFinishReason, OpenRouterCalendarError, OpenRouterCalendarFailureKind,
    OpenRouterResponseFailureKind,
};
use crate::features::calendar_day::contracts::CalendarOrchestrationError;
use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CalendarDiagnosticReason {
    RequiredSourceRefused,
    RequiredSourceUnavailable,
    RequiredSourceInvalid,
    RequiredSourceMissingDay,
    InvalidDay,
    InvalidJson,
    InvalidShape,
    EmptyOutput,
    EmptyItems,
    LimitExceeded,
    MissingField,
    NonFrenchField,
    InvalidTime,
    OutsideDay,
    SourceContract,
    Provenance,
    Serialization,
    IncompleteCompletion,
    OpenRouterHttpStatus,
    OpenRouterRequestFailed,
    OpenRouterInvalidJson,
    OpenRouterInvalidEnvelope,
    OpenRouterEmbeddedError,
    OpenRouterInvalidModel,
    OpenRouterInvalidContent,
    OpenRouterBusy,
    OpenRouterTimeout,
    OpenRouterTransport,
    OpenRouterRateLimited,
    OpenRouterServerError,
    OpenRouterEmptyCompletion,
    OpenRouterResponseTooLarge,
    OpenRouterInvalidPrompt,
    OpenRouterPromptTooLarge,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn contract_diagnostics_keep_only_closed_causes_and_coordinates() {
        use CalendarDiagnosticReason as R;
        use CalendarOrchestrationError as E;
        for (error, reason) in [
            (E::InvalidModelJson, R::InvalidJson),
            (E::InvalidIngestionShape, R::InvalidShape),
            (E::EmptyIngestion, R::EmptyItems),
            (E::EmptyModelOutput, R::EmptyOutput),
            (
                E::MissingVisibleField {
                    item_index: 2,
                    field: "ttsText",
                },
                R::MissingField,
            ),
            (
                E::NonFrenchVisibleField {
                    item_index: 3,
                    field: "translatedTitle",
                },
                R::NonFrenchField,
            ),
            (E::InvalidEventTime { item_index: 4 }, R::InvalidTime),
            (E::ItemOutsideTargetDay { item_index: 5 }, R::OutsideDay),
            (
                E::ContractLimitExceeded {
                    kind: "calendar_items",
                },
                R::LimitExceeded,
            ),
        ] {
            let diagnostic = CalendarIngestionDiagnostic::contract(&error);
            assert_eq!(diagnostic.reason, reason);
            let value = serde_json::to_value(&diagnostic).unwrap();
            assert_eq!(value.as_object().unwrap().len(), 5);
            assert!(value.get("content").is_none());
        }
    }

    #[test]
    fn openrouter_diagnostics_keep_only_a_closed_class_and_http_status() {
        let status = CalendarIngestionDiagnostic::openrouter(OpenRouterCalendarError::HttpStatus {
            status_code: 400,
        });
        assert_eq!(
            status.reason,
            CalendarDiagnosticReason::OpenRouterHttpStatus
        );
        assert_eq!(status.http_status, Some(400));
        let status_json = serde_json::to_value(&status).unwrap();
        assert_eq!(status_json["httpStatus"], 400);
        assert!(status_json.get("content").is_none());
        assert!(status_json.get("prompt").is_none());

        let truncated = CalendarIngestionDiagnostic::openrouter(
            OpenRouterCalendarError::IncompleteCompletion {
                reason: CalendarFinishReason::Length,
            },
        );
        assert_eq!(
            truncated.reason,
            CalendarDiagnosticReason::IncompleteCompletion
        );
        assert_eq!(truncated.finish_reason, Some(CalendarFinishReason::Length));
        assert_eq!(truncated.http_status, None);

        let invalid =
            CalendarIngestionDiagnostic::openrouter(OpenRouterCalendarError::InvalidResponse {
                kind: OpenRouterResponseFailureKind::Json,
            });
        assert_eq!(
            invalid.reason,
            CalendarDiagnosticReason::OpenRouterInvalidJson
        );
        assert_eq!(invalid.http_status, None);
    }

    #[test]
    fn exhausted_retries_preserve_closed_last_failure_and_optional_status() {
        use CalendarDiagnosticReason as R;
        use OpenRouterCalendarFailureKind as F;
        for (last_failure, status_code, reason) in [
            (F::Timeout, None, R::OpenRouterTimeout),
            (F::Transport, None, R::OpenRouterTransport),
            (F::RateLimited, Some(429), R::OpenRouterRateLimited),
            (F::Server, Some(503), R::OpenRouterServerError),
        ] {
            let diagnostic = CalendarIngestionDiagnostic::openrouter(
                OpenRouterCalendarError::RetriesExhausted {
                    attempts: 3,
                    last_failure,
                    status_code,
                },
            );
            assert_eq!(diagnostic.reason, reason);
            assert_eq!(diagnostic.http_status, status_code);
            assert_eq!(
                serde_json::to_value(diagnostic)
                    .unwrap()
                    .as_object()
                    .unwrap()
                    .len(),
                5
            );
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CalendarIngestionDiagnostic {
    pub reason: CalendarDiagnosticReason,
    pub item_index: Option<usize>,
    pub field: Option<&'static str>,
    pub finish_reason: Option<CalendarFinishReason>,
    pub http_status: Option<u16>,
}

impl CalendarIngestionDiagnostic {
    pub(crate) fn new(reason: CalendarDiagnosticReason) -> Self {
        Self {
            reason,
            item_index: None,
            field: None,
            finish_reason: None,
            http_status: None,
        }
    }

    /// Transforme l'erreur IA en cause ferme sans conserver de corps, URL, prompt ou detail fournisseur.
    pub(crate) fn openrouter(error: OpenRouterCalendarError) -> Self {
        use CalendarDiagnosticReason as R;
        let mut result = match error {
            OpenRouterCalendarError::HttpStatus { status_code } => {
                let mut result = Self::new(R::OpenRouterHttpStatus);
                result.http_status = Some(status_code);
                result
            }
            OpenRouterCalendarError::IncompleteCompletion { reason } => {
                let mut result = Self::new(R::IncompleteCompletion);
                result.finish_reason = Some(reason);
                result
            }
            OpenRouterCalendarError::InvalidResponse { kind } => Self::new(match kind {
                OpenRouterResponseFailureKind::Request => R::OpenRouterRequestFailed,
                OpenRouterResponseFailureKind::Json => R::OpenRouterInvalidJson,
                OpenRouterResponseFailureKind::Envelope => R::OpenRouterInvalidEnvelope,
                OpenRouterResponseFailureKind::EmbeddedError => R::OpenRouterEmbeddedError,
                OpenRouterResponseFailureKind::Model => R::OpenRouterInvalidModel,
                OpenRouterResponseFailureKind::Content => R::OpenRouterInvalidContent,
            }),
            OpenRouterCalendarError::EmptyCompletion => Self::new(R::OpenRouterEmptyCompletion),
            OpenRouterCalendarError::ResponseTooLarge => Self::new(R::OpenRouterResponseTooLarge),
            OpenRouterCalendarError::InvalidPrompt => Self::new(R::OpenRouterInvalidPrompt),
            OpenRouterCalendarError::PromptTooLarge => Self::new(R::OpenRouterPromptTooLarge),
            OpenRouterCalendarError::Busy => Self::new(R::OpenRouterBusy),
            OpenRouterCalendarError::RetriesExhausted {
                last_failure,
                status_code,
                ..
            } => {
                let mut result = Self::new(match last_failure {
                    OpenRouterCalendarFailureKind::Timeout => R::OpenRouterTimeout,
                    OpenRouterCalendarFailureKind::Transport => R::OpenRouterTransport,
                    OpenRouterCalendarFailureKind::RateLimited => R::OpenRouterRateLimited,
                    OpenRouterCalendarFailureKind::Server => R::OpenRouterServerError,
                });
                result.http_status = status_code;
                result
            }
        };
        // Les coordinations de champs restent absentes pour une erreur de transport ou fournisseur.
        result.item_index = None;
        result.field = None;
        result
    }

    pub(crate) fn contract(error: &CalendarOrchestrationError) -> Self {
        use CalendarDiagnosticReason as R;
        use CalendarOrchestrationError as E;
        let reason = match error {
            E::InvalidLocalDay | E::StageDayMismatch => R::InvalidDay,
            E::InvalidModelJson => R::InvalidJson,
            E::InvalidIngestionShape => R::InvalidShape,
            E::EmptyModelOutput => R::EmptyOutput,
            E::EmptyIngestion => R::EmptyItems,
            E::ContractLimitExceeded { .. } => R::LimitExceeded,
            E::MissingVisibleField { .. } => R::MissingField,
            E::NonFrenchVisibleField { .. } => R::NonFrenchField,
            E::InvalidEventTime { .. } => R::InvalidTime,
            E::ItemOutsideTargetDay { .. } => R::OutsideDay,
            E::CanonicalSerialization => R::Serialization,
            E::MissingItemProvenance
            | E::UnknownItemProvenance
            | E::UncoveredItemProvenance
            | E::ModelItemProvenanceUrl
            | E::ConflictingItemProvenance
            | E::UnsafeItemProvenanceMetadata => R::Provenance,
            _ => R::SourceContract,
        };
        let mut result = Self::new(reason);
        match error {
            E::MissingVisibleField { item_index, field }
            | E::NonFrenchVisibleField { item_index, field } => {
                result.item_index = Some(*item_index);
                result.field = Some(*field);
            }
            E::InvalidEventTime { item_index } | E::ItemOutsideTargetDay { item_index } => {
                result.item_index = Some(*item_index);
                result.field = Some("eventTime");
            }
            E::ContractLimitExceeded { kind } => result.field = Some(*kind),
            _ => {}
        }
        result
    }
}
