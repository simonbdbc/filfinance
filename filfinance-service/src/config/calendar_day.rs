//! Configuration du calendrier economique execute nativement par Rust.

use std::net::IpAddr;

use super::{
    CalendarDayConfig, CalendarNativeConfig, RedactedSecret,
    environment::{env_bool, env_csv, env_optional, env_or, env_u32, env_u64, env_usize},
};
use crate::errors::AppError;

impl CalendarDayConfig {
    /// Construit la configuration calendrier du jour depuis l'environnement.
    pub fn from_env() -> Result<Self, AppError> {
        let config = Self {
            enabled: env_bool("FILFINANCE_CALENDAR_DAY_ENABLED", false)?,
            schedule_time: env_or("FILFINANCE_CALENDAR_DAY_SCHEDULE_TIME", "01:30"),
            retry_end_time: env_or("FILFINANCE_CALENDAR_DAY_RETRY_END_TIME", "03:00"),
            timezone: env_or("FILFINANCE_CALENDAR_DAY_TIMEZONE", "Europe/Paris"),
            max_retries: env_u32("FILFINANCE_CALENDAR_DAY_MAX_RETRIES", 12)?,
            retry_base_secs: env_u64("FILFINANCE_CALENDAR_DAY_RETRY_BASE_SECS", 1800)?,
            week_generation_window_start: env_or(
                "FILFINANCE_CALENDAR_WEEK_GENERATION_WINDOW_START",
                "02:00",
            ),
            week_generation_window_end: env_or(
                "FILFINANCE_CALENDAR_WEEK_GENERATION_WINDOW_END",
                "04:00",
            ),
            week_verification_window_start: env_or(
                "FILFINANCE_CALENDAR_WEEK_VERIFICATION_WINDOW_START",
                "02:00",
            ),
            week_verification_window_end: env_or(
                "FILFINANCE_CALENDAR_WEEK_VERIFICATION_WINDOW_END",
                "04:00",
            ),
            native: CalendarNativeConfig {
                endpoint: env_or(
                    "OPENROUTER_API_URL",
                    "https://openrouter.ai/api/v1/chat/completions",
                ),
                model: env_or("OPENROUTER_MODEL", "dots-studio/dots-3-note-preview:free"),
                api_key: env_optional("OPENROUTER_API_KEY").map(RedactedSecret::new),
                request_timeout_ms: env_u64(
                    "FILFINANCE_CALENDAR_NATIVE_REQUEST_TIMEOUT_MS",
                    60_000,
                )?,
                retry_attempts: env_usize("FILFINANCE_CALENDAR_NATIVE_RETRY_ATTEMPTS", 2)?,
                retry_backoff_ms: env_u64("FILFINANCE_CALENDAR_NATIVE_RETRY_BACKOFF_MS", 500)?,
                max_concurrency: env_usize("FILFINANCE_CALENDAR_NATIVE_MAX_CONCURRENCY", 1)?,
                queue_capacity: env_usize("FILFINANCE_CALENDAR_NATIVE_QUEUE_CAPACITY", 1)?,
                max_response_bytes: env_usize(
                    "FILFINANCE_CALENDAR_NATIVE_MAX_RESPONSE_BYTES",
                    1_000_000,
                )?,
            },
            alert_offsets_secs: calendar_alert_offsets_from_env()?,
        };
        validate_calendar_day_config(&config)?;
        Ok(config)
    }

    /// Indique si les workers calendrier doivent tourner.
    pub fn active(&self) -> bool {
        self.enabled
    }

    #[cfg(test)]
    pub fn disabled_for_tests() -> Self {
        Self {
            enabled: false,
            schedule_time: "01:30".to_owned(),
            retry_end_time: "03:00".to_owned(),
            timezone: "Europe/Paris".to_owned(),
            max_retries: 12,
            retry_base_secs: 1800,
            week_generation_window_start: "02:00".to_owned(),
            week_generation_window_end: "04:00".to_owned(),
            week_verification_window_start: "02:00".to_owned(),
            week_verification_window_end: "04:00".to_owned(),
            native: CalendarNativeConfig {
                endpoint: "http://127.0.0.1:1/openrouter".to_owned(),
                model: "test-calendar-native-model".to_owned(),
                api_key: None,
                request_timeout_ms: 1_000,
                retry_attempts: 0,
                retry_backoff_ms: 1,
                max_concurrency: 1,
                queue_capacity: 1,
                max_response_bytes: 64 * 1024,
            },
            alert_offsets_secs: vec![300, 60],
        }
    }
}

fn calendar_alert_offsets_from_env() -> Result<Vec<u64>, AppError> {
    let raw = env_csv("FILFINANCE_CALENDAR_ALERT_OFFSETS_SECS");
    if raw.is_empty() {
        return Ok(vec![300, 60]);
    }

    let mut offsets = Vec::new();
    for value in raw {
        let parsed = value.parse::<u64>().map_err(|error| {
            AppError::Config(format!(
                "invalid FILFINANCE_CALENDAR_ALERT_OFFSETS_SECS entry {value}: {error}"
            ))
        })?;
        if parsed == 0 {
            return Err(AppError::Config(
                "FILFINANCE_CALENDAR_ALERT_OFFSETS_SECS entries must be greater than 0".to_owned(),
            ));
        }
        if !offsets.contains(&parsed) {
            offsets.push(parsed);
        }
    }
    offsets.sort_by(|left, right| right.cmp(left));
    Ok(offsets)
}

fn validate_calendar_day_config(config: &CalendarDayConfig) -> Result<(), AppError> {
    validate_schedule_time(&config.schedule_time)?;
    validate_schedule_time_with_name(
        "FILFINANCE_CALENDAR_DAY_RETRY_END_TIME",
        &config.retry_end_time,
    )?;
    validate_schedule_time_with_name(
        "FILFINANCE_CALENDAR_WEEK_GENERATION_WINDOW_START",
        &config.week_generation_window_start,
    )?;
    validate_schedule_time_with_name(
        "FILFINANCE_CALENDAR_WEEK_GENERATION_WINDOW_END",
        &config.week_generation_window_end,
    )?;
    validate_schedule_time_with_name(
        "FILFINANCE_CALENDAR_WEEK_VERIFICATION_WINDOW_START",
        &config.week_verification_window_start,
    )?;
    validate_schedule_time_with_name(
        "FILFINANCE_CALENDAR_WEEK_VERIFICATION_WINDOW_END",
        &config.week_verification_window_end,
    )?;
    validate_window_order(
        "FILFINANCE_CALENDAR_WEEK_GENERATION_WINDOW_START",
        &config.week_generation_window_start,
        "FILFINANCE_CALENDAR_WEEK_GENERATION_WINDOW_END",
        &config.week_generation_window_end,
    )?;
    validate_window_order(
        "FILFINANCE_CALENDAR_WEEK_VERIFICATION_WINDOW_START",
        &config.week_verification_window_start,
        "FILFINANCE_CALENDAR_WEEK_VERIFICATION_WINDOW_END",
        &config.week_verification_window_end,
    )?;
    if config.timezone != "Europe/Paris" {
        return Err(AppError::Config(
            "FILFINANCE_CALENDAR_DAY_TIMEZONE must be Europe/Paris".to_owned(),
        ));
    }
    if config.retry_base_secs == 0 {
        return Err(AppError::Config(
            "FILFINANCE_CALENDAR_DAY_RETRY_BASE_SECS must be greater than 0".to_owned(),
        ));
    }
    if config.alert_offsets_secs.is_empty() {
        return Err(AppError::Config(
            "FILFINANCE_CALENDAR_ALERT_OFFSETS_SECS must contain at least one offset".to_owned(),
        ));
    }
    if config.alert_offsets_secs.len() > 4
        || config
            .alert_offsets_secs
            .iter()
            .any(|offset| *offset == 0 || *offset > 86_400)
    {
        return Err(AppError::Config(
            "FILFINANCE_CALENDAR_ALERT_OFFSETS_SECS must contain at most four offsets between 1 and 86400"
                .to_owned(),
        ));
    }

    validate_calendar_native_config(&config.native)?;

    if !config.enabled {
        return Ok(());
    }

    let Some(api_key) = config.native.api_key.as_ref() else {
        return Err(AppError::Config(
            "OPENROUTER_API_KEY is required when FILFINANCE_CALENDAR_DAY_ENABLED=true".to_owned(),
        ));
    };
    if api_key.expose().len() > 4_096 || api_key.expose().chars().any(char::is_control) {
        return Err(AppError::Config(
            "OPENROUTER_API_KEY is invalid for calendar native runtime".to_owned(),
        ));
    }

    Ok(())
}

fn validate_calendar_native_config(config: &CalendarNativeConfig) -> Result<(), AppError> {
    let endpoint = url::Url::parse(&config.endpoint)
        .map_err(|_| AppError::Config("OPENROUTER_API_URL is invalid".to_owned()))?;
    let Some(host) = endpoint.host_str() else {
        return Err(AppError::Config(
            "OPENROUTER_API_URL must contain a host".to_owned(),
        ));
    };
    let loopback = host.eq_ignore_ascii_case("localhost")
        || host
            .parse::<IpAddr>()
            .is_ok_and(|address| address.is_loopback());
    if endpoint.scheme() != "https" && !(endpoint.scheme() == "http" && loopback) {
        return Err(AppError::Config(
            "OPENROUTER_API_URL must use HTTPS except for loopback tests".to_owned(),
        ));
    }
    if !endpoint.username().is_empty()
        || endpoint.password().is_some()
        || endpoint.query().is_some()
        || endpoint.fragment().is_some()
    {
        return Err(AppError::Config(
            "OPENROUTER_API_URL must not contain credentials, query or fragment".to_owned(),
        ));
    }
    let model = config.model.trim();
    if model.is_empty()
        || model.len() > 160
        || !model.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'/' | b':')
        })
    {
        return Err(AppError::Config(
            "OPENROUTER_MODEL must be a valid provider/model identifier".to_owned(),
        ));
    }
    if config.request_timeout_ms == 0 || config.request_timeout_ms > 60_000 {
        return Err(AppError::Config(
            "FILFINANCE_CALENDAR_NATIVE_REQUEST_TIMEOUT_MS must be between 1 and 60000".to_owned(),
        ));
    }
    if config.retry_attempts > 3 {
        return Err(AppError::Config(
            "FILFINANCE_CALENDAR_NATIVE_RETRY_ATTEMPTS must be at most 3".to_owned(),
        ));
    }
    if (config.retry_attempts > 0 && config.retry_backoff_ms == 0)
        || config.retry_backoff_ms > 5_000
    {
        return Err(AppError::Config(
            "FILFINANCE_CALENDAR_NATIVE_RETRY_BACKOFF_MS must be between 1 and 5000 when retries are enabled"
                .to_owned(),
        ));
    }
    if config.max_concurrency != 1 {
        return Err(AppError::Config(
            "FILFINANCE_CALENDAR_NATIVE_MAX_CONCURRENCY must be exactly 1".to_owned(),
        ));
    }
    if config.queue_capacity == 0 || config.queue_capacity > 16 {
        return Err(AppError::Config(
            "FILFINANCE_CALENDAR_NATIVE_QUEUE_CAPACITY must be between 1 and 16".to_owned(),
        ));
    }
    if config.max_response_bytes == 0 || config.max_response_bytes > 1_000_000 {
        return Err(AppError::Config(
            "FILFINANCE_CALENDAR_NATIVE_MAX_RESPONSE_BYTES must be between 1 and 1000000"
                .to_owned(),
        ));
    }
    Ok(())
}

fn validate_schedule_time(value: &str) -> Result<(), AppError> {
    validate_schedule_time_with_name("FILFINANCE_CALENDAR_DAY_SCHEDULE_TIME", value)
}

fn validate_schedule_time_with_name(name: &str, value: &str) -> Result<(), AppError> {
    let Some((hour, minute)) = value.split_once(':') else {
        return Err(AppError::Config(format!("{name} must use HH:MM")));
    };
    let hour = hour
        .parse::<u8>()
        .map_err(|error| AppError::Config(format!("invalid {name} hour: {error}")))?;
    let minute = minute
        .parse::<u8>()
        .map_err(|error| AppError::Config(format!("invalid {name} minute: {error}")))?;
    if hour > 23 || minute > 59 {
        return Err(AppError::Config(format!(
            "{name} must be a valid HH:MM time"
        )));
    }
    Ok(())
}

fn validate_window_order(
    start_name: &str,
    start_value: &str,
    end_name: &str,
    end_value: &str,
) -> Result<(), AppError> {
    let start = parse_schedule_minutes(start_name, start_value)?;
    let end = parse_schedule_minutes(end_name, end_value)?;
    if end <= start {
        return Err(AppError::Config(format!(
            "{end_name} must be after {start_name}"
        )));
    }
    Ok(())
}

fn parse_schedule_minutes(name: &str, value: &str) -> Result<u16, AppError> {
    let Some((hour, minute)) = value.split_once(':') else {
        return Err(AppError::Config(format!("{name} must use HH:MM")));
    };
    let hour = hour
        .parse::<u16>()
        .map_err(|error| AppError::Config(format!("invalid {name} hour: {error}")))?;
    let minute = minute
        .parse::<u16>()
        .map_err(|error| AppError::Config(format!("invalid {name} minute: {error}")))?;
    Ok(hour.saturating_mul(60).saturating_add(minute))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_native_calendar_is_valid_and_bounded() {
        let config = CalendarDayConfig::disabled_for_tests();

        assert!(!config.enabled);
        assert_eq!(config.native.max_concurrency, 1);
        assert_eq!(config.native.queue_capacity, 1);
        validate_calendar_day_config(&config).expect("disabled native config should be valid");
    }

    #[test]
    fn enabled_native_calendar_requires_openrouter_secret() {
        let mut config = CalendarDayConfig::disabled_for_tests();
        config.enabled = true;

        let error = validate_calendar_day_config(&config)
            .expect_err("enabled calendar without OpenRouter key must fail");
        assert!(error.to_string().contains("OPENROUTER_API_KEY"));
    }

    #[test]
    fn native_calendar_rejects_invalid_resource_bounds() {
        let mut config = CalendarDayConfig::disabled_for_tests().native;
        config.queue_capacity = 0;
        assert!(validate_calendar_native_config(&config).is_err());

        config.queue_capacity = 1;
        config.max_concurrency = 2;
        assert!(validate_calendar_native_config(&config).is_err());

        config.max_concurrency = 1;
        config.retry_attempts = 4;
        assert!(validate_calendar_native_config(&config).is_err());

        config.retry_attempts = 2;
        config.retry_backoff_ms = 5_001;
        assert!(validate_calendar_native_config(&config).is_err());

        config.retry_attempts = 0;
        config.retry_backoff_ms = 1;
        config.max_response_bytes = 1_000_001;
        assert!(validate_calendar_native_config(&config).is_err());
    }

    #[test]
    fn native_calendar_rejects_insecure_or_secret_bearing_endpoint() {
        let mut config = CalendarDayConfig::disabled_for_tests().native;
        config.endpoint = "http://openrouter.example/v1/chat/completions".to_owned();
        assert!(validate_calendar_native_config(&config).is_err());

        config.endpoint = "https://openrouter.ai/api/v1/chat/completions?token=secret".to_owned();
        assert!(validate_calendar_native_config(&config).is_err());

        config.endpoint = "http://127.0.0.1:8080/v1/chat/completions".to_owned();
        validate_calendar_native_config(&config)
            .expect("a bounded loopback endpoint should remain valid");
    }

    #[test]
    fn calendar_windows_and_alert_offsets_remain_bounded() {
        let mut config = CalendarDayConfig::disabled_for_tests();
        config.week_generation_window_end = "02:00".to_owned();
        assert!(validate_calendar_day_config(&config).is_err());

        config.week_generation_window_end = "04:00".to_owned();
        config.alert_offsets_secs = vec![300, 60, 30, 10, 5];
        assert!(validate_calendar_day_config(&config).is_err());
    }

    #[test]
    fn redacted_secret_debug_never_contains_its_value() {
        let secret = RedactedSecret::new("openrouter-test-secret".to_owned());
        let debug = format!("{secret:?}");

        assert!(!debug.contains("openrouter-test-secret"));
        assert!(debug.contains("configured"));
    }
}
