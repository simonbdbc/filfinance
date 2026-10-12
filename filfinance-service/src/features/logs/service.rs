//! Lecture bornee des fichiers de logs applicatifs.

// La lecture disque est synchrone mais executee via `spawn_blocking` cote HTTP.
use std::{
    collections::VecDeque,
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, Seek, SeekFrom},
    path::{Path, PathBuf},
};

// `Date` valide strictement les jours consultables.
use time::{Date, OffsetDateTime};

use crate::{
    config::LoggingConfig,
    errors::AppError,
    features::logs::domain::{
        AdminLogDay, AdminLogDaysResponse, AdminLogLine, AdminLogPage, AdminLogTailAppend,
        AdminLogTailSnapshot,
    },
};

/// Parametres normalises de lecture d'une page de logs.
#[derive(Debug, Clone)]
pub struct AdminLogReadParams {
    /// Jour UTC demande.
    pub day: String,
    /// Nombre de lignes maximum demande avant bornage config.
    pub limit: Option<usize>,
    /// Curseur zero-based de la prochaine ligne fichier a inspecter.
    pub cursor: Option<usize>,
    /// Filtre niveau optionnel.
    pub level: Option<String>,
    /// Recherche texte optionnelle, insensible a la casse.
    pub query: Option<String>,
}

/// Parametres normalises pour le tail WebSocket d'un fichier de logs.
#[derive(Debug, Clone)]
pub struct AdminLogTailParams {
    /// Jour UTC demande; `None` cible le jour courant.
    pub day: Option<String>,
    /// Nombre de lignes initiales a rejouer.
    pub limit: Option<usize>,
    /// Filtre niveau optionnel.
    pub level: Option<String>,
    /// Recherche texte optionnelle, insensible a la casse.
    pub query: Option<String>,
}

/// Etat disque conserve entre deux cycles de tail.
#[derive(Debug, Clone)]
pub struct AdminLogTailCursor {
    /// Jour UTC effectivement observe.
    pub day: String,
    /// Fichier resolu cote serveur, jamais fourni directement par le client.
    pub path: PathBuf,
    /// Taille fichier connue apres la derniere lecture.
    pub file_size_bytes: u64,
    /// Prochain index ligne a affecter aux nouvelles lignes.
    pub next_cursor: usize,
    /// Filtre niveau normalise.
    pub level: Option<String>,
    /// Recherche texte normalisee.
    pub query: Option<String>,
}

/// Liste les jours J/J-1 qui possedent un fichier de logs.
pub async fn list_log_days(config: &LoggingConfig) -> Result<AdminLogDaysResponse, AppError> {
    let config = config.clone();
    tokio::task::spawn_blocking(move || list_log_days_blocking(&config))
        .await
        .map_err(|error| AppError::Storage(format!("log days task failed: {error}")))?
}

/// Lit une page de logs sans jamais accepter de chemin arbitraire.
pub async fn read_log_page(
    config: &LoggingConfig,
    params: AdminLogReadParams,
) -> Result<AdminLogPage, AppError> {
    let config = config.clone();
    tokio::task::spawn_blocking(move || read_log_page_blocking(&config, params))
        .await
        .map_err(|error| AppError::Storage(format!("log page task failed: {error}")))?
}

/// Lit le snapshot initial du tail WebSocket sans bloquer le runtime async.
pub async fn read_tail_snapshot(
    config: &LoggingConfig,
    params: AdminLogTailParams,
) -> Result<(AdminLogTailSnapshot, AdminLogTailCursor), AppError> {
    let config = config.clone();
    tokio::task::spawn_blocking(move || read_tail_snapshot_blocking(&config, params))
        .await
        .map_err(|error| AppError::Storage(format!("log tail snapshot task failed: {error}")))?
}

/// Lit les lignes ajoutees depuis le dernier offset connu.
pub async fn read_tail_append(
    cursor: AdminLogTailCursor,
) -> Result<(AdminLogTailAppend, AdminLogTailCursor), AppError> {
    tokio::task::spawn_blocking(move || read_tail_append_blocking(cursor))
        .await
        .map_err(|error| AppError::Storage(format!("log tail append task failed: {error}")))?
}

pub(crate) fn list_log_days_blocking(
    config: &LoggingConfig,
) -> Result<AdminLogDaysResponse, AppError> {
    let today = OffsetDateTime::now_utc().date();
    let mut days = Vec::new();

    for day in retained_days(today, config.retention_days) {
        let path = log_path(&config.directory, day);
        if let Ok(metadata) = fs::symlink_metadata(&path)
            && metadata.file_type().is_file()
        {
            let size_bytes = metadata.len();
            days.push(AdminLogDay {
                day: format_day(day),
                size_bytes,
            });
        }
    }

    Ok(AdminLogDaysResponse { days })
}

fn retained_days(today: Date, retention_days: usize) -> Vec<Date> {
    let mut days = Vec::with_capacity(retention_days);
    let mut current = today;

    for _ in 0..retention_days {
        days.push(current);
        current = current.previous_day().unwrap_or(current);
    }

    days
}

pub(crate) fn read_tail_snapshot_blocking(
    config: &LoggingConfig,
    params: AdminLogTailParams,
) -> Result<(AdminLogTailSnapshot, AdminLogTailCursor), AppError> {
    let day = match params.day.as_deref() {
        Some(day) if !day.trim().is_empty() => parse_allowed_day(day)?,
        _ => OffsetDateTime::now_utc().date(),
    };
    let day_text = format_day(day);
    let path = log_path(&config.directory, day);
    let limit = params
        .limit
        .unwrap_or(config.admin_max_lines)
        .clamp(1, config.admin_max_lines);
    let level_filter = params.level.as_deref().map(str::to_ascii_lowercase);
    let query_filter = params.query.as_deref().map(str::to_ascii_lowercase);
    let mut retained_items = VecDeque::with_capacity(limit);
    let mut next_cursor = 0;

    let file = open_log_file(&path)?;
    let mut file_size_bytes = 0;
    if let Some(file) = file {
        let mut reader = BufReader::new(file);
        for (index, line_result) in (&mut reader).lines().enumerate() {
            let text = line_result?;
            next_cursor = index + 1;
            if !line_matches_filters(&text, level_filter.as_deref(), query_filter.as_deref()) {
                continue;
            }
            if retained_items.len() == limit {
                retained_items.pop_front();
            }
            retained_items.push_back(AdminLogLine {
                cursor: index,
                level: detect_level(&text),
                text,
            });
        }
        // Advance by the bytes actually consumed from this descriptor, including
        // a concurrent append already returned in the snapshot.
        file_size_bytes = reader.stream_position()?;
    }

    let items = retained_items.into_iter().collect::<Vec<_>>();
    let snapshot = AdminLogTailSnapshot {
        day: day_text.clone(),
        items,
        next_cursor,
        file_size_bytes,
    };
    let cursor = AdminLogTailCursor {
        day: day_text,
        path,
        file_size_bytes,
        next_cursor,
        level: level_filter,
        query: query_filter,
    };

    Ok((snapshot, cursor))
}

pub(crate) fn read_tail_append_blocking(
    mut cursor: AdminLogTailCursor,
) -> Result<(AdminLogTailAppend, AdminLogTailCursor), AppError> {
    let file = open_log_file(&cursor.path)?;
    let current_size = file
        .as_ref()
        .map(|file| file.metadata().map(|m| m.len()))
        .transpose()?
        .unwrap_or(0);
    let mut items = Vec::new();

    // Une rotation ou truncation du fichier invalide l'offset; on repart au debut pour rester coherent.
    if current_size < cursor.file_size_bytes {
        cursor.file_size_bytes = 0;
        cursor.next_cursor = 0;
    }

    if current_size > cursor.file_size_bytes {
        let mut file = file.expect("a growing regular log is present");
        file.seek(SeekFrom::Start(cursor.file_size_bytes))?;
        let mut reader = BufReader::new(file);

        for line_result in (&mut reader).lines() {
            let text = line_result?;
            let line_cursor = cursor.next_cursor;
            cursor.next_cursor += 1;
            if !line_matches_filters(&text, cursor.level.as_deref(), cursor.query.as_deref()) {
                continue;
            }
            items.push(AdminLogLine {
                cursor: line_cursor,
                level: detect_level(&text),
                text,
            });
        }
        cursor.file_size_bytes = reader.stream_position()?;
    }

    let append = AdminLogTailAppend {
        day: cursor.day.clone(),
        items,
        next_cursor: cursor.next_cursor,
        file_size_bytes: cursor.file_size_bytes,
    };

    Ok((append, cursor))
}

pub(crate) fn read_log_page_blocking(
    config: &LoggingConfig,
    params: AdminLogReadParams,
) -> Result<AdminLogPage, AppError> {
    let day = parse_allowed_day(&params.day)?;
    let path = log_path(&config.directory, day);
    let limit = params
        .limit
        .unwrap_or(config.admin_max_lines)
        .clamp(1, config.admin_max_lines);
    let start_cursor = params.cursor.unwrap_or(0);
    let level_filter = params.level.as_deref().map(str::to_ascii_lowercase);
    let query_filter = params.query.as_deref().map(str::to_ascii_lowercase);
    let mut items = Vec::with_capacity(limit);
    let mut next_cursor = None;

    let Some(file) = open_log_file(&path)? else {
        return Ok(AdminLogPage {
            day: format_day(day),
            items,
            has_more: false,
            next_cursor: None,
        });
    };

    let reader = BufReader::new(file);

    for (index, line_result) in reader.lines().enumerate().skip(start_cursor) {
        let text = line_result?;
        let level = detect_level(&text);
        if !matches_level(level.as_deref(), level_filter.as_deref()) {
            continue;
        }
        if !matches_query(&text, query_filter.as_deref()) {
            continue;
        }

        if items.len() == limit {
            next_cursor = Some(index);
            break;
        }

        items.push(AdminLogLine {
            cursor: index,
            level,
            text,
        });
    }

    Ok(AdminLogPage {
        day: format_day(day),
        items,
        has_more: next_cursor.is_some(),
        next_cursor,
    })
}

// The configured log directory is an operator-controlled storage root. Refuse
// untrusted leaf links and special files before reading any content. Descriptor
// checks also reject a replacement during opening; O_NONBLOCK prevents FIFO hangs.
fn open_log_file(path: &Path) -> Result<Option<File>, AppError> {
    let before = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    if !before.file_type().is_file() {
        return Err(AppError::Storage(
            "log must be a regular file without symlinks".to_owned(),
        ));
    }
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = options.open(path)?;
    let opened = file.metadata()?;
    let after = fs::symlink_metadata(path)?;
    if !opened.is_file() || !after.file_type().is_file() {
        return Err(AppError::Storage("log changed during opening".to_owned()));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if before.dev() != opened.dev()
            || before.ino() != opened.ino()
            || after.dev() != opened.dev()
            || after.ino() != opened.ino()
        {
            return Err(AppError::Storage("log changed during opening".to_owned()));
        }
    }
    Ok(Some(file))
}

fn parse_allowed_day(value: &str) -> Result<Date, AppError> {
    let requested = parse_day(value).ok_or_else(|| {
        AppError::Config("log day must use YYYY-MM-DD and stay inside J/J-1".to_owned())
    })?;
    let today = OffsetDateTime::now_utc().date();
    let yesterday = today.previous_day().unwrap_or(today);

    if requested == today || requested == yesterday {
        return Ok(requested);
    }

    Err(AppError::Config(
        "log day is outside the retained J/J-1 window".to_owned(),
    ))
}

fn log_path(directory: &str, day: Date) -> PathBuf {
    Path::new(directory).join(format!("{}.log", format_day(day)))
}

fn parse_day(value: &str) -> Option<Date> {
    let mut parts = value.split('-');
    let year = parts.next()?.parse::<i32>().ok()?;
    let month = parts.next()?.parse::<u8>().ok()?;
    let day = parts.next()?.parse::<u8>().ok()?;
    if parts.next().is_some() {
        return None;
    }

    Date::from_calendar_date(year, month.try_into().ok()?, day).ok()
}

pub(crate) fn format_day(day: Date) -> String {
    let month: u8 = day.month().into();
    format!("{:04}-{:02}-{:02}", day.year(), month, day.day())
}

fn detect_level(line: &str) -> Option<String> {
    for level in ["ERROR", "WARN", "INFO", "DEBUG", "TRACE"] {
        if line.contains(level) {
            return Some(level.to_ascii_lowercase());
        }
    }

    None
}

fn line_matches_filters(line: &str, level: Option<&str>, query: Option<&str>) -> bool {
    let detected_level = detect_level(line);
    matches_level(detected_level.as_deref(), level) && matches_query(line, query)
}

fn matches_level(level: Option<&str>, expected: Option<&str>) -> bool {
    expected.is_none_or(|expected| level == Some(expected))
}

fn matches_query(line: &str, expected: Option<&str>) -> bool {
    expected.is_none_or(|expected| line.to_ascii_lowercase().contains(expected))
}

#[cfg(test)]
pub(super) mod test_api {
    pub(crate) use super::{
        AdminLogReadParams, AdminLogTailParams, format_day, list_log_days_blocking,
        read_log_page_blocking, read_tail_append_blocking, read_tail_snapshot_blocking,
    };
}
