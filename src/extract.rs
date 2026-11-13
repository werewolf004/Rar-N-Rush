//! RAR extraction with the official `UnRAR` library (through the `unrar` crate).
//!
//! `UnRAR` does not report byte-level progress through the crate API, so a helper
//! thread watches the size of the file currently being written.
//!
//! Safety rules applied to every entry:
//! - absolute paths and `..` components are refused (no writes outside `dest`);
//! - existing files are never overwritten unless asked: a free name is used instead.

use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;
use unrar::Archive;
use unrar::error::{Code, UnrarError};

use crate::sync::lock;

const PROGRESS_INTERVAL: Duration = Duration::from_millis(250);
/// Upper bound when looking for a free `name (n).ext`.
const MAX_RENAME_ATTEMPTS: u32 = 10_000;

/// Options for [`extract`].
#[derive(Debug, Clone, Copy, Default)]
pub struct Options {
    /// Overwrite files that already exist in the destination.
    pub overwrite_existing: bool,
}

/// Successful end states of an extraction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Paths of the extracted files.
    Extracted(Vec<PathBuf>),
    /// The archive is protected and none of the given passwords works.
    NeedPassword,
    /// The user cancelled the run.
    Cancelled,
}

/// What went wrong inside the archive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveProblem {
    /// Data is corrupt (CRC error).
    Corrupt,
    /// Not a valid archive.
    Invalid,
    /// Unknown archive format or version.
    UnknownFormat,
    /// A volume is missing or cannot be opened.
    MissingVolume,
    /// A destination file cannot be created.
    CannotCreate,
    /// Writing failed (disk full?).
    WriteFailed,
    /// Out of memory.
    OutOfMemory,
    /// Other `UnRAR` error.
    Other,
}

impl From<Code> for ArchiveProblem {
    fn from(code: Code) -> Self {
        match code {
            Code::BadData => Self::Corrupt,
            Code::BadArchive => Self::Invalid,
            Code::UnknownFormat => Self::UnknownFormat,
            Code::EOpen => Self::MissingVolume,
            Code::ECreate => Self::CannotCreate,
            Code::EWrite => Self::WriteFailed,
            Code::NoMemory => Self::OutOfMemory,
            _ => Self::Other,
        }
    }
}

/// Errors raised during extraction.
#[derive(Debug, thiserror::Error)]
pub enum ExtractError {
    /// The destination folder cannot be created.
    #[error("cannot create folder {path}: {source}")]
    CreateDir {
        /// Folder concerned.
        path: PathBuf,
        /// Underlying error.
        #[source]
        source: io::Error,
    },
    /// The archive could not be read or an entry could not be written.
    #[error("{problem:?} in {archive}{}", entry.as_ref().map(|e| format!(" ({})", e.display())).unwrap_or_default())]
    Archive {
        /// First volume of the archive.
        archive: PathBuf,
        /// Entry being processed, if any.
        entry: Option<PathBuf>,
        /// What went wrong.
        problem: ArchiveProblem,
    },
}

struct Entry {
    size: u64,
    is_dir: bool,
    encrypted: bool,
}

fn is_password_error(e: &UnrarError) -> bool {
    matches!(e.code, Code::MissingPassword | Code::BadPassword)
}

/// `true` when the entry path stays inside the destination folder.
pub fn is_safe_relative(p: &Path) -> bool {
    !p.as_os_str().is_empty()
        && p.components()
            .all(|c| matches!(c, Component::Normal(_) | Component::CurDir))
}

/// First free path: `path`, then `stem (1).ext`, `stem (2).ext`, …
pub fn free_path(path: &Path) -> PathBuf {
    if !path.exists() {
        return path.to_path_buf();
    }
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let ext = path
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    let parent = path.parent().unwrap_or_else(|| Path::new(""));
    (1..MAX_RENAME_ATTEMPTS)
        .map(|n| parent.join(format!("{stem} ({n}){ext}")))
        .find(|p| !p.exists())
        .unwrap_or_else(|| path.to_path_buf())
}

fn list(first: &Path, password: Option<&str>) -> Result<Vec<Entry>, UnrarError> {
    let archive = match password {
        Some(pw) => Archive::with_password(first, pw),
        None => Archive::new(first),
    };
    archive
        .open_for_listing()?
        .map(|h| {
            h.map(|h| Entry {
                size: h.unpacked_size,
                is_dir: h.is_directory(),
                encrypted: h.is_encrypted(),
            })
        })
        .collect()
}

/// Extracts `first` (first volume) into `dest`, trying `passwords` when needed.
pub fn extract(
    first: &Path,
    dest: &Path,
    passwords: &[String],
    options: Options,
    cancel: &AtomicBool,
    progress: impl Fn(f32) + Sync,
) -> Result<Outcome, ExtractError> {
    let archive_err = |problem| ExtractError::Archive {
        archive: first.to_path_buf(),
        entry: None,
        problem,
    };

    // 1. List without password: is the archive encrypted, and how big is it?
    let (plain_entries, needs_password) = match list(first, None) {
        Ok(entries) => {
            let encrypted = entries.iter().any(|e| e.encrypted);
            (Some(entries), encrypted)
        }
        Err(e) if is_password_error(&e) => (None, true), // encrypted headers
        Err(e) => return Err(archive_err(e.code.into())),
    };

    let candidates: Vec<Option<&str>> = if needs_password {
        passwords
            .iter()
            .filter(|p| !p.is_empty())
            .map(|p| Some(p.as_str()))
            .collect()
    } else {
        vec![None]
    };

    for password in candidates {
        let entries = match (&plain_entries, password) {
            (Some(entries), None) => entries.iter().map(|e| (e.size, e.is_dir)).collect(),
            _ => match list(first, password) {
                Ok(entries) => entries.iter().map(|e| (e.size, e.is_dir)).collect::<Vec<_>>(),
                Err(e) if is_password_error(&e) || e.code == Code::BadData => continue,
                Err(e) => return Err(archive_err(e.code.into())),
            },
        };
        let total: u64 = entries.iter().filter(|(_, dir)| !dir).map(|(s, _)| s).sum();
        match extract_once(first, dest, password, total, options, cancel, &progress)? {
            Attempt::Done(files) => return Ok(Outcome::Extracted(files)),
            Attempt::Cancelled => return Ok(Outcome::Cancelled),
            Attempt::WrongPassword(created) => {
                // Remove what the wrong password produced, then try the next one.
                for f in created.iter().rev() {
                    let _ = fs::remove_file(f);
                }
            }
        }
    }
    Ok(Outcome::NeedPassword)
}

enum Attempt {
    Done(Vec<PathBuf>),
    WrongPassword(Vec<PathBuf>),
    Cancelled,
}

struct Shared {
    done_bytes: AtomicU64,
    current: Mutex<Option<PathBuf>>,
    finished: AtomicBool,
}

fn extract_once(
    first: &Path,
    dest: &Path,
    password: Option<&str>,
    total: u64,
    options: Options,
    cancel: &AtomicBool,
    progress: &(impl Fn(f32) + Sync),
) -> Result<Attempt, ExtractError> {
    fs::create_dir_all(dest).map_err(|source| ExtractError::CreateDir {
        path: dest.to_path_buf(),
        source,
    })?;
    let total = total.max(1);
    let shared = Shared {
        done_bytes: AtomicU64::new(0),
        current: Mutex::new(None),
        finished: AtomicBool::new(false),
    };

    std::thread::scope(|scope| {
        scope.spawn(|| {
            while !shared.finished.load(Ordering::Relaxed) {
                let current = lock(&shared.current).clone();
                let partial = current.and_then(|p| p.metadata().ok()).map_or(0, |m| m.len());
                let done = shared.done_bytes.load(Ordering::Relaxed) + partial;
                #[allow(clippy::cast_precision_loss)] // progress ratio only
                progress((done as f32 / total as f32).min(1.0));
                std::thread::sleep(PROGRESS_INTERVAL);
            }
        });
        let result = run_entries(first, dest, password, options, cancel, &shared);
        shared.finished.store(true, Ordering::Relaxed);
        result
    })
}

fn run_entries(
    first: &Path,
    dest: &Path,
    password: Option<&str>,
    options: Options,
    cancel: &AtomicBool,
    shared: &Shared,
) -> Result<Attempt, ExtractError> {
    let err = |entry: Option<&Path>, code: Code| ExtractError::Archive {
        archive: first.to_path_buf(),
        entry: entry.map(Path::to_path_buf),
        problem: code.into(),
    };
    let archive = match password {
        Some(pw) => Archive::with_password(first, pw),
        None => Archive::new(first),
    };
    let mut open = match archive.open_for_processing() {
        Ok(a) => a,
        Err(e) if is_password_error(&e) => return Ok(Attempt::WrongPassword(Vec::new())),
        Err(e) => return Err(err(None, e.code)),
    };
    let mut created: Vec<PathBuf> = Vec::new();
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Ok(Attempt::Cancelled);
        }
        let header = match open.read_header() {
            Ok(Some(h)) => h,
            Ok(None) => break,
            Err(e) if is_password_error(&e) => return Ok(Attempt::WrongPassword(created)),
            Err(e) => return Err(err(None, e.code)),
        };
        let entry = header.entry();
        let rel = entry.filename.clone();
        let (size, is_dir, encrypted) = (entry.unpacked_size, entry.is_directory(), entry.is_encrypted());

        if !is_safe_relative(&rel) || is_dir {
            if is_dir && is_safe_relative(&rel) {
                let _ = fs::create_dir_all(dest.join(&rel));
            }
            open = header.skip().map_err(|e| err(Some(&rel), e.code))?;
            continue;
        }
        let mut target = dest.join(&rel);
        if !options.overwrite_existing {
            target = free_path(&target);
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|source| ExtractError::CreateDir {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        *lock(&shared.current) = Some(target.clone());
        created.push(target.clone());
        open = match header.extract_to(&target) {
            Ok(a) => a,
            // RAR4 reports a wrong password as a CRC error on encrypted entries.
            Err(e)
                if is_password_error(&e) || (encrypted && password.is_some() && e.code == Code::BadData) =>
            {
                return Ok(Attempt::WrongPassword(created));
            }
            Err(e) => return Err(err(Some(&rel), e.code)),
        };
        *lock(&shared.current) = None;
        shared.done_bytes.fetch_add(size, Ordering::Relaxed);
    }
    Ok(Attempt::Done(created))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_paths() {
        assert!(is_safe_relative(Path::new("a/b.txt")));
        assert!(is_safe_relative(Path::new("./a.txt")));
        assert!(!is_safe_relative(Path::new("../a.txt")));
        assert!(!is_safe_relative(Path::new("a/../../b")));
        assert!(!is_safe_relative(Path::new("C:\\Windows\\x.dll")));
        assert!(!is_safe_relative(Path::new("\\x")));
        assert!(!is_safe_relative(Path::new("")));
    }

    #[test]
    fn free_path_never_overwrites() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("a.txt");
        assert_eq!(free_path(&p), p);
        fs::write(&p, b"x").unwrap();
        assert_eq!(free_path(&p), dir.path().join("a (1).txt"));
        fs::write(dir.path().join("a (1).txt"), b"x").unwrap();
        assert_eq!(free_path(&p), dir.path().join("a (2).txt"));
        let no_ext = dir.path().join("README");
        fs::write(&no_ext, b"x").unwrap();
        assert_eq!(free_path(&no_ext), dir.path().join("README (1)"));
    }
}
