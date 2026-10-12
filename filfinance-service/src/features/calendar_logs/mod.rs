//! Detailed calendar diagnostics. Business code emits; the logger owns ordering and files.
pub(crate) mod reader;
mod redact;
pub(crate) use redact::environment_secrets;

use ordered_local_logger::{
    Config, EmitResult, Fields, InstanceConfig, Level, Logger, Runtime, Source,
};
use serde_json::Value;
use std::{
    future::Future,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::Instant,
};

tokio::task_local! { static ACTIVE: Context; }
static EVENT_ID: AtomicU64 = AtomicU64::new(1);
const PRODUCTION_MIN_FREE_BYTES: u64 = 10 * 1024 * 1024 * 1024;
const LOCAL_MIN_FREE_BYTES: u64 = 100 * 1024 * 1024;
const MAX_BRIEF_EVENT_BYTES: usize = 2048;

struct DetailGate {
    path: PathBuf,
    min_free_bytes: u64,
    degraded: AtomicBool,
}

impl DetailGate {
    fn new(path: PathBuf, min_free_bytes: u64) -> Self {
        Self {
            path,
            min_free_bytes,
            degraded: AtomicBool::new(false),
        }
    }

    fn available(&self) -> std::io::Result<u64> {
        available_bytes(&self.path)
    }
}

#[cfg(unix)]
fn available_bytes(path: &Path) -> std::io::Result<u64> {
    use std::{ffi::CString, os::unix::ffi::OsStrExt};
    let path = CString::new(path.as_os_str().as_bytes())?;
    let mut info = std::mem::MaybeUninit::<libc::statvfs>::uninit();
    if unsafe { libc::statvfs(path.as_ptr(), info.as_mut_ptr()) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    let info = unsafe { info.assume_init() };
    Ok((info.f_bavail as u64).saturating_mul(info.f_frsize as u64))
}

#[cfg(not(unix))]
fn available_bytes(_path: &Path) -> std::io::Result<u64> {
    Ok(u64::MAX)
}

#[derive(Clone, Default)]
struct Context {
    logger: Option<Logger>,
    fields: Fields,
    secrets: Arc<Vec<String>>,
    refused: Arc<AtomicU64>,
    detail_gate: Option<Arc<DetailGate>>,
}

#[derive(Clone)]
pub(crate) struct CalendarLogs {
    context: Context,
    pub reader: Arc<reader::Reader>,
}

pub(crate) struct CalendarLogRuntime {
    runtime: Option<Runtime>,
}

impl CalendarLogs {
    pub async fn start(
        directory: PathBuf,
        secrets: Vec<String>,
        production: bool,
    ) -> (Self, CalendarLogRuntime) {
        let path = directory.clone();
        let result = tokio::task::spawn_blocking(move || {
            std::fs::create_dir_all(&path)?;
            let path = std::fs::canonicalize(&path)?;
            let mut config = Config::new(path.clone());
            config.max_event_bytes = 16 * 1024;
            let runtime = Runtime::new(config).map_err(std::io::Error::other)?;
            let mut instance = InstanceConfig::new("calendar");
            instance.level = Level::Debug;
            instance.destinations = vec!["workflow".into()];
            let logger = runtime.instance(instance).map_err(std::io::Error::other)?;
            Ok::<_, std::io::Error>((runtime, logger))
        })
        .await;
        let (runtime, logger, log_root, rlogger_run_id) = match result {
            Ok(Ok((runtime, logger))) => {
                let log_root = runtime.log_root().to_path_buf();
                let rlogger_run_id = runtime.run_id().to_owned();
                (Some(runtime), Some(logger), log_root, Some(rlogger_run_id))
            }
            _ => {
                tracing::error!(
                    "calendar logger unavailable; workflow continues without detailed file diagnostics"
                );
                (None, None, directory.join("rlogger"), None)
            }
        };
        let detail_gate = logger.as_ref().map(|_| {
            Arc::new(DetailGate::new(
                log_root.clone(),
                if production {
                    PRODUCTION_MIN_FREE_BYTES
                } else {
                    LOCAL_MIN_FREE_BYTES
                },
            ))
        });
        let mut context = Context {
            logger,
            secrets: Arc::new(secrets),
            detail_gate,
            ..Context::default()
        };
        if let Some(run_id) = rlogger_run_id {
            context
                .fields
                .insert("rlogger_run_id".into(), run_id.into());
        }
        let logs = Self {
            context,
            reader: Arc::new(reader::Reader::new(log_root)),
        };
        (logs, CalendarLogRuntime { runtime })
    }

    #[cfg(test)]
    pub fn disabled() -> Self {
        Self {
            context: Context::default(),
            reader: Arc::new(reader::Reader::new(PathBuf::from(
                "data/calendar-logs/rlogger",
            ))),
        }
    }

    #[cfg(test)]
    pub fn test_reader(directory: PathBuf) -> Self {
        Self {
            context: Context::default(),
            reader: Arc::new(reader::Reader::new(directory)),
        }
    }

    /// Explicit root scope; child scopes inherit it, including across awaits.
    pub async fn scope<T>(&self, fields: Value, future: impl Future<Output = T>) -> T {
        let mut context = self.context.clone();
        context.extend(fields);
        ACTIVE.scope(context, future).await
    }

    pub fn event(&self, label: &str, value: &Value) {
        self.context
            .write_brief(Level::Info, label, &value.to_string());
    }
}

impl CalendarLogRuntime {
    pub async fn shutdown(mut self) {
        if let Some(runtime) = self.runtime.take() {
            let result = tokio::task::spawn_blocking(move || runtime.shutdown()).await;
            match result {
                Ok(Ok(report)) => tracing::info!(?report, "calendar logger drained"),
                Ok(Err(error)) => tracing::error!(
                    ?error,
                    "calendar logger shutdown failed; log delivery incomplete"
                ),
                Err(error) => tracing::error!(%error, "calendar logger shutdown task failed"),
            }
        }
    }
}

impl Context {
    #[track_caller]
    fn write_brief(&self, level: Level, label: &str, text: &str) {
        let text = redact::text(text, &self.secrets);
        if text.len() <= MAX_BRIEF_EVENT_BYTES {
            self.write(level, label, &text);
            return;
        }
        let mut end = MAX_BRIEF_EVENT_BYTES;
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        self.write(
            level,
            label,
            &format!(
                "{} [brief_event_truncated original_bytes={}]",
                &text[..end],
                text.len()
            ),
        );
    }

    fn extend(&mut self, value: Value) {
        if let Value::Object(fields) = value {
            for (key, value) in fields {
                // Context values stay small. Full inputs belong in the message fragments.
                let text = match value {
                    Value::String(s) => s,
                    v => v.to_string(),
                };
                let text = redact::text(&text, &self.secrets);
                let bounded: String = text.chars().take(160).collect();
                self.fields.insert(key, bounded.into());
            }
        }
    }

    #[track_caller]
    fn write(&self, level: Level, label: &str, text: &str) {
        let Some(logger) = &self.logger else {
            return;
        };
        let text = redact::text(text, &self.secrets);
        let location = std::panic::Location::caller();
        let source = Source {
            module: "calendar_workflow",
            file: location.file(),
            line: location.line(),
        };
        let id = EVENT_ID.fetch_add(1, Ordering::Relaxed);
        let mut fields = self.fields.clone();
        fields.insert("event_id".into(), id.into());
        fields.insert("action".into(), label.to_owned().into());
        fields.insert("payload_bytes".into(), (text.len() as u64).into());
        // Chunk BEFORE RLOG escaping; the logger accounts for escaping separately.
        // Small chunks also leave room for correlation fields within the admission budget.
        let mut start = 0;
        let mut part = 0u64;
        loop {
            let mut end = (start + 2048).min(text.len());
            while !text.is_char_boundary(end) {
                end -= 1;
            }
            part += 1;
            fields.insert("part".into(), part.into());
            fields.insert("last_part".into(), (end == text.len()).into());
            let result = logger.emit(level, source, None, &fields, &text[start..end]);
            if let EmitResult::Refused(reason) = result {
                let count = self.refused.fetch_add(1, Ordering::Relaxed) + 1;
                if count == 1 || count.is_power_of_two() {
                    tracing::error!(target: "calendar_logger_internal", ?reason, refused_events = count, "calendar logger refused diagnostic fragments; logs incomplete");
                }
            }
            if end == text.len() {
                break;
            }
            start = end;
        }
    }

    #[track_caller]
    fn write_payload(&self, label: &str, text: &str) {
        let Some(gate) = &self.detail_gate else {
            self.write(Level::Debug, label, text);
            return;
        };
        let available = gate.available().ok();
        let degraded = available.is_none_or(|bytes| bytes < gate.min_free_bytes);
        let previous = gate.degraded.swap(degraded, Ordering::Relaxed);
        if degraded {
            if !previous {
                self.write(Level::Warn, "diagnostics.degraded", &serde_json::json!({
                    "reason": if available.is_none() { "disk_space_unavailable" } else { "low_disk_space" },
                    "available_bytes": available,
                    "min_free_bytes": gate.min_free_bytes,
                }).to_string());
                tracing::error!(target: "calendar_logger_internal", ?available, min_free_bytes = gate.min_free_bytes, "calendar payload diagnostics suspended");
            }
            self.write(
                Level::Warn,
                "payload.suppressed",
                &serde_json::json!({
                    "payload_action": label,
                    "payload_bytes": text.len(),
                    "reason": "disk_reserve",
                })
                .to_string(),
            );
            return;
        }
        if previous {
            self.write(
                Level::Info,
                "diagnostics.restored",
                &serde_json::json!({
                    "available_bytes": available,
                })
                .to_string(),
            );
        }
        self.write(Level::Debug, label, text);
    }
}

/// Add correlation without attaching arbitrary payloads to persistent context.
pub(crate) async fn context<T>(fields: Value, future: impl Future<Output = T>) -> T {
    let mut current = ACTIVE.try_with(Clone::clone).unwrap_or_default();
    current.extend(fields);
    ACTIVE.scope(current, future).await
}

#[track_caller]
pub(crate) fn payload(label: &str, text: &str) {
    let _ = ACTIVE.try_with(|current| current.write_payload(label, text));
}

#[track_caller]
pub(crate) fn event(label: &str, value: Value) {
    let _ = ACTIVE.try_with(|current| current.write_brief(Level::Info, label, &value.to_string()));
}

/// Records pure parsing and validation boundaries when the caller is already
/// inside a calendar workflow scope. It does not alter their error contracts.
pub(crate) fn sync_operation<T>(label: &str, operation: impl FnOnce() -> T) -> T {
    if !is_active() {
        return operation();
    }
    let mut guard = Operation::start(label, serde_json::json!({}));
    let result = operation();
    guard.finish_observed(&"completed");
    result
}

/// Tool diagnostics are text. Audio bytes are logged only through their
/// caller's metadata, never as an RLOG payload.
pub(crate) fn bytes(label: &str, value: &[u8]) {
    event(
        label,
        serde_json::json!({
            "bytes": value.len(),
        }),
    );
    payload(&format!("{label}.text"), &String::from_utf8_lossy(value));
}

/// Piper emits base64 WAV alongside its text diagnostics. Omit the binary data
/// while retaining the response fields and the exact decoded-size estimate.
pub(crate) fn piper_response(value: &str) {
    let mut payload = serde_json::from_str::<serde_json::Value>(value)
        .unwrap_or_else(|_| serde_json::json!({ "raw": value }));
    if let Some(object) = payload.as_object_mut() {
        if let Some(audio) = object.remove("audio_base64") {
            object.insert(
                "audio_base64_bytes".into(),
                serde_json::json!(
                    audio
                        .as_str()
                        .map(|encoded| encoded.len().saturating_mul(3) / 4)
                ),
            );
        }
    }
    event("piper.response", serde_json::json!({"bytes": value.len()}));
    self::payload("piper.response_body", &payload.to_string());
}

/// Observe a Result without altering it; on cancellation Drop reports an unfinished operation.
pub(crate) fn is_active() -> bool {
    ACTIVE.try_with(|c| c.logger.is_some()).unwrap_or(false)
}

pub(crate) fn operation<'a, T: std::fmt::Debug + Send + 'a, E: std::fmt::Debug + Send + 'a>(
    label: &'a str,
    input: Value,
    future: impl Future<Output = Result<T, E>> + Send + 'a,
) -> std::pin::Pin<Box<dyn Future<Output = Result<T, E>> + Send + 'a>> {
    Box::pin(async move {
        if !is_active() {
            return future.await;
        }
        context(
            serde_json::json!({"step": label, "call_id": uuid::Uuid::new_v4().to_string()}),
            async {
                let mut guard = Operation::start(label, input);
                let result = future.await;
                guard.complete(&result);
                result
            },
        )
        .await
    })
}

pub(crate) fn observe<'a, T: std::fmt::Debug + Send + 'a>(
    label: &'a str,
    input: Value,
    future: impl Future<Output = T> + Send + 'a,
) -> std::pin::Pin<Box<dyn Future<Output = T> + Send + 'a>> {
    Box::pin(async move {
        if !is_active() {
            return future.await;
        }
        context(
            serde_json::json!({"step":label,"call_id":uuid::Uuid::new_v4().to_string()}),
            async {
                let mut guard = Operation::start(label, input);
                let result = future.await;
                guard.finish_observed(&result);
                result
            },
        )
        .await
    })
}

pub(crate) struct Operation {
    label: String,
    context: Context,
    started: Instant,
    complete: bool,
}
impl Operation {
    pub fn start(label: &str, input: Value) -> Self {
        let context = ACTIVE.try_with(Clone::clone).unwrap_or_default();
        context.write_brief(Level::Info, &format!("{label}.started"), &input.to_string());
        Self {
            label: label.into(),
            context,
            started: Instant::now(),
            complete: false,
        }
    }
    pub fn complete<T: std::fmt::Debug, E: std::fmt::Debug>(&mut self, result: &Result<T, E>) {
        self.complete = true;
        self.context.write_brief(
            if result.is_ok() {
                Level::Info
            } else {
                Level::Error
            },
            &format!(
                "{}.{}",
                self.label,
                if result.is_ok() {
                    "succeeded"
                } else {
                    "failed"
                }
            ),
            &format!(
                "duration_ms={} result={result:?}",
                self.started.elapsed().as_millis()
            ),
        );
    }

    /// An observed value is not a Result: a business error can be an enum variant.
    pub fn finish_observed<T: std::fmt::Debug>(&mut self, result: &T) {
        self.complete = true;
        self.context.write_brief(
            Level::Info,
            &format!("{}.finished", self.label),
            &format!(
                "duration_ms={} outcome={result:?}",
                self.started.elapsed().as_millis()
            ),
        );
    }
}
impl Drop for Operation {
    fn drop(&mut self) {
        if !self.complete {
            self.context.write_brief(
                Level::Warn,
                &format!("{}.interrupted", self.label),
                &format!(
                    "duration_ms={} operation dropped before completion",
                    self.started.elapsed().as_millis()
                ),
            );
        }
    }
}

/// Capture context explicitly when handing work to a detached task.
pub(crate) fn carry<T>(future: impl Future<Output = T>) -> impl Future<Output = T> {
    let current = ACTIVE.try_with(Clone::clone).unwrap_or_default();
    ACTIVE.scope(current, future)
}

#[cfg(test)]
mod tests;
