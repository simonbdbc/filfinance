//! Initialisation de la telemetry process.

// Les logs fichiers utilisent uniquement la bibliotheque standard pour eviter une dependance runtime.
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

// `Date` donne une rotation journaliere stable en UTC.
use time::{Date, OffsetDateTime};
// `EnvFilter` lit le niveau de logs depuis l'environnement.
use tracing_subscriber::{
    EnvFilter, Layer, fmt::MakeWriter, layer::SubscriberExt, util::SubscriberInitExt,
};
// `fmt` installe le renderer texte des logs `tracing`.
use tracing_subscriber::fmt;

use crate::config::LoggingConfig;

/// Initialise le subscriber `tracing` avec un filtre configurable via l'environnement.
pub fn init_tracing(logging: &LoggingConfig) {
    // `try_from_default_env` lit par exemple `RUST_LOG`; le fallback garde un niveau exploitable en dev.
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    // `with_target(false)` retire le nom du module dans le rendu texte pour garder des logs plus lisibles.
    let stdout_layer = fmt::layer().with_target(false);

    if logging.file_enabled {
        let file_writer = DailyLogMakeWriter::new(PathBuf::from(&logging.directory));
        // Le layer fichier est volontairement sans ANSI pour que le viewer admin lise du texte propre.
        let file_layer = fmt::layer()
            .with_target(false)
            .with_ansi(false)
            .with_writer(file_writer);
        tracing_subscriber::registry()
            .with(stdout_layer.with_filter(filter.clone()))
            .with(file_layer.with_filter(filter))
            .init();
        return;
    }

    tracing_subscriber::registry()
        .with(stdout_layer.with_filter(filter))
        .init();
}

/// Writer `tracing` qui ouvre le fichier du jour et purge J-2+ lors du changement de date.
#[derive(Clone)]
struct DailyLogMakeWriter {
    directory: Arc<PathBuf>,
    last_pruned_day: Arc<Mutex<Option<Date>>>,
}

impl DailyLogMakeWriter {
    fn new(directory: PathBuf) -> Self {
        Self {
            directory: Arc::new(directory),
            last_pruned_day: Arc::new(Mutex::new(None)),
        }
    }

    fn open_today_file(&self) -> Option<File> {
        let today = today_utc();
        self.prune_once_per_day(today);
        if fs::create_dir_all(self.directory.as_ref()).is_err() {
            return None;
        }

        OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.directory.join(log_file_name(today)))
            .ok()
    }

    fn prune_once_per_day(&self, today: Date) {
        let mut last_pruned_day = self
            .last_pruned_day
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if last_pruned_day.as_ref() == Some(&today) {
            return;
        }

        prune_old_log_files(self.directory.as_ref(), today);
        *last_pruned_day = Some(today);
    }
}

impl<'a> MakeWriter<'a> for DailyLogMakeWriter {
    type Writer = DailyLogWriter;

    fn make_writer(&'a self) -> Self::Writer {
        DailyLogWriter {
            file: self.open_today_file(),
        }
    }
}

/// Writer d'un evenement de log; si le fichier est indisponible, l'evenement reste sur stdout.
struct DailyLogWriter {
    file: Option<File>,
}

impl Write for DailyLogWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self.file.as_mut() {
            Some(file) => file.write(buf),
            None => Ok(buf.len()),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self.file.as_mut() {
            Some(file) => file.flush(),
            None => Ok(()),
        }
    }
}

fn today_utc() -> Date {
    OffsetDateTime::now_utc().date()
}

fn prune_old_log_files(directory: &Path, today: Date) {
    let yesterday = today.previous_day().unwrap_or(today);
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let Some(day) = path
            .file_name()
            .and_then(|name| parse_log_day(name.to_string_lossy().as_ref()))
        else {
            continue;
        };
        if day != today && day != yesterday {
            let _ = fs::remove_file(path);
        }
    }
}

fn parse_log_day(file_name: &str) -> Option<Date> {
    let day = file_name.strip_suffix(".log")?;
    parse_yyyy_mm_dd(day)
}

fn parse_yyyy_mm_dd(value: &str) -> Option<Date> {
    let mut parts = value.split('-');
    let year = parts.next()?.parse::<i32>().ok()?;
    let month = parts.next()?.parse::<u8>().ok()?;
    let day = parts.next()?.parse::<u8>().ok()?;
    if parts.next().is_some() {
        return None;
    }

    Date::from_calendar_date(year, month.try_into().ok()?, day).ok()
}

fn log_file_name(day: Date) -> String {
    let month: u8 = day.month().into();
    format!("{:04}-{:02}-{:02}.log", day.year(), month, day.day())
}
