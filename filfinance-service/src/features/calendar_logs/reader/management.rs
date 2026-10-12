//! Bounded inventories and management of explicitly finalized RLOGGER files.
use super::protocol::{Error, Result};
use ordered_local_logger::storage::{self, FileState, LogName};
use serde::Serialize;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
use std::{
    collections::HashMap,
    fs::{self, File, Metadata},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, Weak,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
pub fn now() -> i64 {
    chrono::Utc::now().timestamp()
}
pub fn allocated(m: &Metadata) -> Option<String> {
    #[cfg(unix)]
    {
        Some((u128::from(m.blocks()) * 512).to_string())
    }
    #[cfg(not(unix))]
    {
        let _ = m;
        None
    }
}
pub fn file_state(root: &Path, path: &Path, now: i64) -> (String, bool, String) {
    if !storage::recognized(root) || !storage::hourly_layout(path) {
        return (
            "legacy".into(),
            false,
            "Ancien format ou racine générique : consultation seule.".into(),
        );
    }
    let Some(name) = path
        .file_name()
        .and_then(|n| n.to_str())
        .and_then(LogName::parse)
    else {
        return ("legacy".into(), false, "Format horaire non reconnu.".into());
    };
    if storage::open_beneath(
        root,
        &Path::new(storage::TECH).join(format!("{}.lock", name.run_id)),
        false,
    )
    .is_err()
    {
        return (
            "unknown".into(),
            false,
            "Métadonnées du run indisponibles.".into(),
        );
    }
    let state = match name.state {
        FileState::Active => "active",
        FileState::Closed => "closed",
        FileState::Recovered => "recovered",
    };
    let reason = if name.state == FileState::Active {
        "Fichier actif : écriture non finalisée."
    } else if !name.ended(now) {
        "L’intervalle horaire n’est pas terminé."
    } else {
        ""
    };
    (state.into(), reason.is_empty(), reason.into())
}
pub fn validate(root: &Path, relative: &Path) -> Result<Metadata> {
    storage::checked_directory(root)?;
    let mut p = root.to_path_buf();
    for c in relative.components() {
        if !matches!(c, std::path::Component::Normal(_)) {
            return Err(Error::new("FORBIDDEN", "Chemin refusé.", 403));
        }
        p.push(c);
        let m = fs::symlink_metadata(&p)?;
        if m.file_type().is_symlink() {
            return Err(Error::new("FORBIDDEN", "Lien symbolique refusé.", 403));
        }
    }
    if fs::canonicalize(&p)? != p {
        return Err(Error::new("FORBIDDEN", "Chemin modifié.", 403));
    }
    Ok(fs::symlink_metadata(p)?)
}
pub fn action_file(root: &Path, relative: &Path, expected: &str, delete: bool) -> Result<File> {
    let stat = validate(root, relative)?;
    let (_, allowed, reason) = file_state(root, relative, now());
    if !allowed {
        return Err(Error::new("FILE_PROTECTED", reason, 409));
    }
    let file = storage::open_beneath(root, relative, false)?;
    if delete {
        file.try_lock()
            .map_err(|_| Error::new("FILE_BUSY", "Fichier occupé par un téléchargement.", 409))?;
    } else {
        file.try_lock_shared()
            .map_err(|_| Error::new("FILE_BUSY", "Fichier occupé.", 409))?;
    }
    let opened = file.metadata()?;
    if storage::token(&stat) != expected
        || storage::token(&opened) != expected
        || !storage::same_file(&opened, &validate(root, relative)?)
    {
        return Err(Error::new(
            "FILE_CHANGED",
            "Le fichier a changé : actualisez avant de réessayer.",
            409,
        ));
    }
    Ok(file)
}
#[derive(Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Size {
    pub content: String,
    pub allocated: Option<String>,
    pub partial: bool,
}
#[derive(Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Inventory {
    pub items: HashMap<String, Size>,
    pub sampled_at: String,
    pub partial: bool,
    pub busy: bool,
    pub notices: Vec<String>,
}
#[derive(Default)]
pub struct Hub(Mutex<HashMap<PathBuf, Weak<Job>>>);
impl Hub {
    pub fn get(&self, path: PathBuf) -> Arc<Job> {
        let mut jobs = self.0.lock().unwrap();
        jobs.retain(|_, j| j.strong_count() > 0);
        if let Some(job) = jobs.get(&path).and_then(Weak::upgrade)
            && job.valid()
        {
            return job;
        }
        let job = Arc::new(Job {
            identity: fs::metadata(&path).ok(),
            path,
            snapshot: Mutex::new(Inventory::default()),
            running: AtomicBool::new(false),
            dirty: AtomicBool::new(false),
            last: Mutex::new(None),
        });
        jobs.insert(job.path.clone(), Arc::downgrade(&job));
        job
    }
}
pub struct Job {
    path: PathBuf,
    identity: Option<Metadata>,
    snapshot: Mutex<Inventory>,
    running: AtomicBool,
    dirty: AtomicBool,
    last: Mutex<Option<Instant>>,
}
impl Job {
    fn valid(&self) -> bool {
        self.identity
            .as_ref()
            .zip(fs::symlink_metadata(&self.path).ok().as_ref())
            .is_some_and(|(a, b)| storage::same_file(a, b) && !b.file_type().is_symlink())
    }
    pub fn snapshot(&self) -> Inventory {
        let mut v = self.snapshot.lock().unwrap().clone();
        v.busy = self.running.load(Ordering::Relaxed);
        v
    }
    pub fn kick(self: &Arc<Self>, force: bool) {
        if force {
            self.dirty.store(true, Ordering::Release);
        }
        if !force
            && self
                .last
                .lock()
                .unwrap()
                .is_some_and(|t| t.elapsed() < Duration::from_secs(60))
        {
            return;
        }
        if self.running.swap(true, Ordering::AcqRel) {
            return;
        }
        let job = self.clone();
        tokio::task::spawn_blocking(move || {
            job.dirty.store(false, Ordering::Release);
            let valid = job
                .identity
                .as_ref()
                .zip(fs::symlink_metadata(&job.path).ok().as_ref())
                .is_some_and(|(a, b)| storage::same_file(a, b) && !b.file_type().is_symlink());
            let mut scan = Scan {
                job: &job,
                seen: 0,
                result: Inventory::default(),
                now: now(),
                today: chrono::Local::now().date_naive(),
                managed: storage::recognized(&job.path),
            };
            if valid {
                scan.directory(Path::new(""), 0);
            } else {
                scan.notice("Racine remplacée : inventaire interrompu.");
            }
            scan.result.sampled_at = chrono::Utc::now().to_rfc3339();
            *job.snapshot.lock().unwrap() = scan.result;
            *job.last.lock().unwrap() = Some(Instant::now());
            job.running.store(false, Ordering::Release);
            if job.dirty.swap(false, Ordering::AcqRel) && Arc::strong_count(&job) > 1 {
                job.kick(true);
            }
        });
    }
}
#[derive(Default)]
struct Sum {
    bytes: u128,
    blocks: HashMap<(u64, u64), u128>,
    partial: bool,
    unavailable: bool,
}
impl Sum {
    fn size(&self) -> Size {
        Size {
            content: self.bytes.to_string(),
            allocated: (!self.unavailable).then(|| self.blocks.values().sum::<u128>().to_string()),
            partial: self.partial,
        }
    }
    fn add(&mut self, other: Self) {
        self.bytes += other.bytes;
        self.blocks.extend(other.blocks);
        self.partial |= other.partial;
        self.unavailable |= other.unavailable;
    }
}
struct Scan<'a> {
    job: &'a Arc<Job>,
    seen: usize,
    result: Inventory,
    now: i64,
    today: chrono::NaiveDate,
    managed: bool,
}
impl Scan<'_> {
    fn notice(&mut self, s: &str) {
        self.result.partial = true;
        if self.result.notices.len() < 16 {
            self.result.notices.push(s.into());
        }
    }
    fn save(&mut self, path: &Path, size: Size) {
        if self.result.items.len() < 10000 || path.as_os_str().is_empty() {
            self.result
                .items
                .insert(path.to_string_lossy().into_owned(), size);
        } else {
            self.notice("Cache de tailles limité à 10 000 entrées.");
        }
    }
    fn directory(&mut self, relative: &Path, depth: usize) -> Sum {
        let mut sum = Sum::default();
        if depth > 64 || self.seen >= 200000 || Arc::strong_count(self.job) <= 1 {
            sum.partial = true;
            self.notice("Inventaire limité ou annulé.");
            return sum;
        }
        let root = &self.job.path;
        let before = match validate(root, relative) {
            Ok(m) if m.is_dir() => m,
            _ => {
                sum.partial = true;
                self.notice("Dossier inaccessible.");
                return sum;
            }
        };
        let mut entries = match fs::read_dir(root.join(relative)) {
            Ok(e) => e,
            Err(_) => {
                sum.partial = true;
                self.notice("Dossier inaccessible.");
                return sum;
            }
        };
        while self.seen < 200000 {
            let Some(entry) = entries.next() else {
                break;
            };
            self.seen += 1;
            if Arc::strong_count(self.job) <= 1 {
                sum.partial = true;
                self.notice("Inventaire limité ou annulé.");
                break;
            }
            if self.seen.is_multiple_of(128) {
                std::thread::sleep(Duration::from_millis(1));
            }
            let Ok(entry) = entry else {
                sum.partial = true;
                continue;
            };
            let Ok(name) = entry.file_name().into_string() else {
                sum.partial = true;
                continue;
            };
            if self.managed && relative.as_os_str().is_empty() && name == storage::TECH {
                continue;
            }
            let mut child = relative.join(&name);
            if child.as_os_str().len() > 4096 {
                sum.partial = true;
                continue;
            }
            let Ok(meta) = validate(root, &child) else {
                sum.partial = true;
                continue;
            };
            if meta.is_dir() {
                let sub = self.directory(&child, depth + 1);
                sum.add(sub);
                if self.managed
                    && self.old_day(&child)
                    && let Err(e) = (|| -> Result<()> {
                        let _g = storage::gate(root)?;
                        if !self.job.valid() {
                            return Err(Error::new("ROOT_CHANGED", "Racine remplacée.", 409));
                        }
                        validate(root, &child)?;
                        match storage::remove_beneath(root, &child, true, None) {
                            Ok(()) => {}
                            Err(e)
                                if matches!(
                                    e.kind(),
                                    std::io::ErrorKind::DirectoryNotEmpty
                                        | std::io::ErrorKind::NotFound
                                ) => {}
                            Err(e) => return Err(e.into()),
                        }
                        Ok(())
                    })()
                {
                    self.notice(&format!("Nettoyage différé : {}", e.message));
                }
            } else if meta.is_file()
                && (name.to_ascii_lowercase().ends_with(".log")
                    || name.to_ascii_lowercase().ends_with(".txt"))
            {
                if self.managed
                    && depth == 2
                    && storage::hourly_layout(&child)
                    && let Some(log) = LogName::parse(&name)
                    && log.state == FileState::Active
                    && log.ended(self.now)
                {
                    match self.recover(&child, &log) {
                        Ok(Some(new)) => child = new,
                        Ok(None) => {}
                        Err(e) => self.notice(&format!("Récupération différée : {}", e.message)),
                    }
                }
                let m = match validate(root, &child) {
                    Ok(m) => m,
                    Err(_) => {
                        sum.partial = true;
                        continue;
                    }
                };
                let mut file = Sum {
                    bytes: u128::from(m.len()),
                    partial: storage::token(&meta) != storage::token(&m),
                    ..Default::default()
                };
                #[cfg(unix)]
                file.blocks
                    .insert((m.dev(), m.ino()), u128::from(m.blocks()) * 512);
                #[cfg(not(unix))]
                {
                    file.unavailable = true;
                }
                self.save(&child, file.size());
                sum.add(file);
            }
        }
        if self.seen >= 200000 {
            sum.partial = true;
            self.notice("Limite de 200 000 entrées parcourues atteinte.");
        }
        if !validate(root, relative).is_ok_and(|m| storage::token(&m) == storage::token(&before)) {
            sum.partial = true;
        }
        self.result.partial |= sum.partial;
        self.save(relative, sum.size());
        sum
    }
    fn old_day(&self, path: &Path) -> bool {
        path.components()
            .next()
            .and_then(|p| p.as_os_str().to_str())
            .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
            .is_some_and(|d| d < self.today)
    }
    fn recover(&self, path: &Path, log: &LogName) -> Result<Option<PathBuf>> {
        let root = &self.job.path;
        let _g = storage::gate(root)?;
        if !self.job.valid() {
            return Err(Error::new("ROOT_CHANGED", "Racine remplacée.", 409));
        }
        validate(root, path)?;
        let lease = storage::open_beneath(
            root,
            &Path::new(storage::TECH).join(format!("{}.lock", log.run_id)),
            false,
        )?;
        match lease.try_lock() {
            Ok(()) => {}
            Err(std::fs::TryLockError::WouldBlock) => return Ok(None),
            Err(std::fs::TryLockError::Error(e)) => return Err(e.into()),
        }
        let name = path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .replace(".active.log", ".recovered.log");
        let target = path.with_file_name(name);
        storage::rename_beneath(root, path, &target)?;
        Ok(Some(target))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn traversal_stops_at_the_shared_budget_before_fetching_an_extra_entry() {
        let temp = tempfile::tempdir().unwrap();
        for name in ["a.log", "b.log", "c.log"] {
            fs::write(temp.path().join(name), b"x").unwrap();
        }
        let root = fs::canonicalize(temp.path()).unwrap();
        let job = Hub::default().get(root);
        let _session = job.clone();
        let mut scan = Scan {
            job: &job,
            seen: 199999,
            result: Inventory::default(),
            now: now(),
            today: chrono::Local::now().date_naive(),
            managed: false,
        };
        let sum = scan.directory(Path::new(""), 0);
        assert_eq!(scan.seen, 200000);
        assert_eq!(sum.bytes, 1);
        assert!(sum.partial);
        assert!(scan.result.partial);
    }
}
