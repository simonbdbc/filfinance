//! Versioned hourly storage contract shared with the local file manager.
#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
};
pub const MARKER: &str = "RLOGGER storage 2\n";
pub const TECH: &str = ".rlogger";

pub fn hourly_layout(path: &Path) -> bool {
    let parts = path.components().collect::<Vec<_>>();
    if parts.len() != 3
        || parts
            .iter()
            .any(|p| !matches!(p, std::path::Component::Normal(_)))
    {
        return false;
    }
    let Some(day) = parts[0].as_os_str().to_str() else {
        return false;
    };
    chrono::NaiveDate::parse_from_str(day, "%Y-%m-%d")
        .is_ok_and(|d| d.format("%Y-%m-%d").to_string() == day)
        && parts[1]
            .as_os_str()
            .to_str()
            .is_some_and(crate::model::valid_name)
}

#[cfg(unix)]
fn parent_fd(root: &Path, relative: &Path, create: bool) -> io::Result<(File, std::ffi::CString)> {
    use std::{
        ffi::CString,
        os::{
            fd::{AsRawFd, FromRawFd},
            unix::ffi::OsStrExt,
        },
    };
    let mut dir = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW)
        .open(root)?;
    let parts = relative.components().collect::<Vec<_>>();
    if parts.is_empty()
        || parts
            .iter()
            .any(|p| !matches!(p, std::path::Component::Normal(_)))
    {
        return Err(io::Error::other("unsafe relative path"));
    }
    for part in &parts[..parts.len() - 1] {
        let name = CString::new(part.as_os_str().as_bytes())?;
        if create && unsafe { libc::mkdirat(dir.as_raw_fd(), name.as_ptr(), 0o755) } != 0 {
            let e = io::Error::last_os_error();
            if e.kind() != io::ErrorKind::AlreadyExists {
                return Err(e);
            }
        }
        let fd = unsafe {
            libc::openat(
                dir.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        dir = unsafe { File::from_raw_fd(fd) };
    }
    Ok((
        dir,
        CString::new(parts.last().unwrap().as_os_str().as_bytes())?,
    ))
}
pub fn open_beneath(root: &Path, relative: &Path, create: bool) -> io::Result<File> {
    #[cfg(unix)]
    {
        use std::os::fd::{AsRawFd, FromRawFd};
        let (dir, name) = parent_fd(root, relative, create)?;
        let flags = libc::O_NOFOLLOW
            | libc::O_NONBLOCK
            | libc::O_CLOEXEC
            | if create {
                libc::O_RDWR | libc::O_CREAT | libc::O_EXCL
            } else {
                libc::O_RDONLY
            };
        let fd = unsafe { libc::openat(dir.as_raw_fd(), name.as_ptr(), flags, 0o600) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        let file = unsafe { File::from_raw_fd(fd) };
        if !file.metadata()?.is_file() {
            return Err(io::Error::other("not a regular file"));
        }
        Ok(file)
    }
    #[cfg(not(unix))]
    {
        let _ = (root, relative, create);
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "confined operations unsupported",
        ))
    }
}
pub fn remove_beneath(
    root: &Path,
    relative: &Path,
    directory: bool,
    expected: Option<&str>,
) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::fd::{AsRawFd, FromRawFd};
        let (dir, name) = parent_fd(root, relative, false)?;
        let fd = unsafe {
            libc::openat(
                dir.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY
                    | libc::O_NOFOLLOW
                    | libc::O_NONBLOCK
                    | libc::O_CLOEXEC
                    | if directory { libc::O_DIRECTORY } else { 0 },
            )
        };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        let f = unsafe { File::from_raw_fd(fd) };
        let metadata = f.metadata()?;
        if expected.is_some_and(|t| token(&metadata) != t) {
            return Err(io::Error::other("file changed"));
        }
        if unsafe {
            libc::unlinkat(
                dir.as_raw_fd(),
                name.as_ptr(),
                if directory { libc::AT_REMOVEDIR } else { 0 },
            )
        } == 0
        {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }
    #[cfg(not(unix))]
    {
        let _ = (root, relative, directory, expected);
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "confined operations unsupported",
        ))
    }
}
pub fn rename_beneath(root: &Path, relative: &Path, target: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::fd::AsRawFd;
        if relative.parent() != target.parent() {
            return Err(io::Error::other("publication must stay in its directory"));
        }
        // Publication is reserved for regular files, never a symlink source.
        let _source = open_beneath(root, relative, false)?;
        let (dir, old) = parent_fd(root, relative, false)?;
        let (_, new) = parent_fd(root, target, false)?;
        #[cfg(target_os = "macos")]
        let rc = unsafe {
            libc::renameatx_np(
                dir.as_raw_fd(),
                old.as_ptr(),
                dir.as_raw_fd(),
                new.as_ptr(),
                libc::RENAME_EXCL,
            )
        };
        #[cfg(target_os = "linux")]
        let rc = unsafe {
            libc::renameat2(
                dir.as_raw_fd(),
                old.as_ptr(),
                dir.as_raw_fd(),
                new.as_ptr(),
                libc::RENAME_NOREPLACE,
            )
        };
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "atomic rename unsupported",
        ));
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        if rc == 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }
    #[cfg(not(unix))]
    {
        let _ = (root, relative, target);
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "confined operations unsupported",
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileState {
    Active,
    Closed,
    Recovered,
}
#[derive(Debug, Clone)]
pub struct LogName {
    pub run_id: String,
    pub hour_start: i64,
    pub state: FileState,
}
impl LogName {
    pub fn parse(name: &str) -> Option<Self> {
        let (base, state) = if let Some(s) = name.strip_suffix(".active.log") {
            (s, FileState::Active)
        } else if let Some(s) = name.strip_suffix(".recovered.log") {
            (s, FileState::Recovered)
        } else {
            (name.strip_suffix(".log")?, FileState::Closed)
        };
        let (base, segment) = base.rsplit_once("-s")?;
        if segment.parse::<u64>().ok()? == 0 {
            return None;
        }
        let (base, hour) = base.rsplit_once("-h")?;
        let hour_start = hour.parse::<i64>().ok()?;
        hour_start.checked_add(3600)?;
        let mut parts = base.rsplitn(5, '-');
        let counter = parts.next()?;
        let pid = parts.next()?;
        let stamp = parts.next()?;
        let hh = parts.next()?;
        let destination = parts.next()?;
        if ![counter, pid, stamp]
            .iter()
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
            || hh.len() != 2
            || hh.parse::<u8>().ok()? > 23
            || !crate::model::valid_name(destination)
        {
            return None;
        }
        Some(Self {
            run_id: format!("{stamp}-{pid}-{counter}"),
            hour_start,
            state,
        })
    }
    pub fn ended(&self, now: i64) -> bool {
        self.hour_start
            .checked_add(3600)
            .is_some_and(|end| end <= now)
    }
}

/// Open regular files without following a final symlink. Caller validates parents.
pub fn open_file(path: &Path, write: bool, create: bool) -> io::Result<File> {
    let mut o = OpenOptions::new();
    o.read(true).write(write).create_new(create);
    #[cfg(unix)]
    o.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    let f = o.open(path)?;
    if !f.metadata()?.is_file() {
        return Err(io::Error::other("not a regular file"));
    }
    Ok(f)
}
pub fn checked_directory(path: &Path) -> io::Result<()> {
    let m = fs::symlink_metadata(path)?;
    if !m.is_dir() || m.file_type().is_symlink() {
        return Err(io::Error::other("unsafe directory"));
    }
    Ok(())
}
pub fn recognized(root: &Path) -> bool {
    (|| -> io::Result<bool> {
        checked_directory(root)?;
        checked_directory(&root.join(TECH))?;
        let mut s = String::new();
        open_beneath(root, &Path::new(TECH).join("format"), false)?
            .take(128)
            .read_to_string(&mut s)?;
        Ok(s == MARKER)
    })()
    .unwrap_or(false)
}
pub fn initialize(root: &Path) -> io::Result<()> {
    fs::create_dir_all(root)?;
    checked_directory(root)?;
    match fs::create_dir(root.join(TECH)) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
        Err(e) => return Err(e),
    }
    checked_directory(&root.join(TECH))?;
    // The gate is permanent: unlinking a lock would split cooperating processes.
    let gate_path = Path::new(TECH).join("gate");
    let gate = match open_beneath(root, &gate_path, true) {
        Ok(f) => f,
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
            open_beneath(root, &gate_path, false)?
        }
        Err(e) => return Err(e),
    };
    gate.lock()?;
    let marker = Path::new(TECH).join("format");
    match open_beneath(root, &marker, true) {
        Ok(mut f) => f.write_all(MARKER.as_bytes())?,
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
        Err(e) => return Err(e),
    }
    if !recognized(root) {
        return Err(io::Error::other("unsupported RLOGGER storage"));
    }
    Ok(())
}
pub fn gate(root: &Path) -> io::Result<File> {
    if !recognized(root) {
        return Err(io::Error::other("unrecognized RLOGGER root"));
    }
    let file = open_beneath(root, &Path::new(TECH).join("gate"), false)?;
    file.lock()?;
    Ok(file)
}
pub fn lease_path(root: &Path, run: &str) -> PathBuf {
    root.join(TECH).join(format!("{run}.lock"))
}
pub fn token(meta: &fs::Metadata) -> String {
    #[cfg(unix)]
    {
        format!(
            "{}-{}-{}-{}-{}-{}-{}",
            meta.dev(),
            meta.ino(),
            meta.len(),
            meta.mtime(),
            meta.mtime_nsec(),
            meta.ctime(),
            meta.ctime_nsec()
        )
    }
    #[cfg(not(unix))]
    {
        format!("{}-{:?}", meta.len(), meta.modified().ok())
    }
}
pub fn same_file(a: &fs::Metadata, b: &fs::Metadata) -> bool {
    #[cfg(unix)]
    {
        a.dev() == b.dev() && a.ino() == b.ino()
    }
    #[cfg(not(unix))]
    {
        a.created().ok() == b.created().ok()
    }
}
