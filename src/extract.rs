//! RAR extraction with the official `UnRAR` library (through the `unrar` crate).
//!
//! `UnRAR` does not report byte-level progress through the crate API, so a helper
//! thread watches the size of the file currently being written.
//!
//! Safety rules applied to every entry:
//! - absolute paths and `..` components are refused (no writes outside `dest`);
//! - existing files are never overwritten unless asked: a free name is used instead;
//! - the free space of the destination drive is checked before writing anything;
//! - when an extraction fails or is cancelled, the files it created are removed,
//!   so a retry starts clean (files that existed before are never removed).

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
/// Free space kept in reserve on the destination drive, on top of the archive size.
pub const SPACE_MARGIN: u64 = 64 * 1024 * 1024;

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
    /// The destination drive does not have enough free space.
    #[error("not enough free space in {dest}: {needed} bytes needed, {available} available")]
    NotEnoughSpace {
        /// Destination folder.
        dest: PathBuf,
        /// Bytes needed (archive content plus [`SPACE_MARGIN`]).
        needed: u64,
        /// Bytes available.
        available: u64,
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

/// Nearest existing folder at or above `path` (the destination may not exist yet).
fn existing_ancestor(path: &Path) -> Option<&Path> {
    path.ancestors().find(|p| p.is_dir())
}

/// Checks that the drive of `dest` can hold `size` bytes plus [`SPACE_MARGIN`].
/// When the free space cannot be read, extraction is allowed: a full disk is still
/// reported by `UnRAR` as a write error.
fn check_space(dest: &Path, size: u64) -> Result<(), ExtractError> {
    let Some(available) = existing_ancestor(dest).and_then(|dir| fs4::available_space(dir).ok()) else {
        return Ok(());
    };
    let needed = size.saturating_add(SPACE_MARGIN);
    if available < needed {
        return Err(ExtractError::NotEnoughSpace {
            dest: dest.to_path_buf(),
            needed,
            available,
        });
    }
    Ok(())
}

/// A file written by an attempt.
struct Created {
    path: PathBuf,
    /// The file was already there (overwrite enabled): never removed.
    existed: bool,
}

/// Removes the files an attempt created (not those it overwrote), then the
/// folders left empty under `dest`.
fn remove_created(created: &[Created], dest: &Path) {
    for file in created.iter().rev().filter(|c| !c.existed) {
        let _ = fs::remove_file(&file.path);
        let mut dir = file.path.parent();
        while let Some(d) = dir.filter(|d| d.starts_with(dest) && *d != dest) {
            if fs::remove_dir(d).is_err() {
                break; // not empty
            }
            dir = d.parent();
        }
    }
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
        check_space(dest, total)?;
        let (result, created) = extract_once(first, dest, password, total, options, cancel, &progress);
        match result {
            Ok(Attempt::Done) => {
                return Ok(Outcome::Extracted(created.into_iter().map(|c| c.path).collect()));
            }
            // Remove what the wrong password produced, then try the next one.
            Ok(Attempt::WrongPassword) => remove_created(&created, dest),
            Ok(Attempt::Cancelled) => {
                remove_created(&created, dest);
                return Ok(Outcome::Cancelled);
            }
            Err(e) => {
                remove_created(&created, dest);
                return Err(e);
            }
        }
    }
    Ok(Outcome::NeedPassword)
}

enum Attempt {
    Done,
    WrongPassword,
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
) -> (Result<Attempt, ExtractError>, Vec<Created>) {
    let mut created = Vec::new();
    if let Err(source) = fs::create_dir_all(dest) {
        let error = ExtractError::CreateDir {
            path: dest.to_path_buf(),
            source,
        };
        return (Err(error), created);
    }
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
        let result = run_entries(first, dest, password, options, cancel, &shared, &mut created);
        shared.finished.store(true, Ordering::Relaxed);
        (result, created)
    })
}

fn run_entries(
    first: &Path,
    dest: &Path,
    password: Option<&str>,
    options: Options,
    cancel: &AtomicBool,
    shared: &Shared,
    created: &mut Vec<Created>,
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
        Err(e) if is_password_error(&e) => return Ok(Attempt::WrongPassword),
        Err(e) => return Err(err(None, e.code)),
    };
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Ok(Attempt::Cancelled);
        }
        let header = match open.read_header() {
            Ok(Some(h)) => h,
            Ok(None) => break,
            Err(e) if is_password_error(&e) => return Ok(Attempt::WrongPassword),
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
        created.push(Created {
            existed: target.exists(),
            path: target.clone(),
        });
        open = match header.extract_to(&target) {
            Ok(a) => a,
            // RAR4 reports a wrong password as a CRC error on encrypted entries.
            Err(e)
                if is_password_error(&e) || (encrypted && password.is_some() && e.code == Code::BadData) =>
            {
                return Ok(Attempt::WrongPassword);
            }
            Err(e) => return Err(err(Some(&rel), e.code)),
        };
        *lock(&shared.current) = None;
        shared.done_bytes.fetch_add(size, Ordering::Relaxed);
    }
    Ok(Attempt::Done)
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
    fn space_check_uses_the_nearest_existing_folder() {
        let dir = tempfile::tempdir().unwrap();
        let deep = dir.path().join("not").join("yet").join("created");
        assert_eq!(existing_ancestor(&deep), Some(dir.path()));
        assert!(check_space(&deep, 1).is_ok());
        match check_space(&deep, u64::MAX - 1) {
            Err(ExtractError::NotEnoughSpace {
                needed, available, ..
            }) => {
                assert_eq!(needed, u64::MAX, "saturates instead of overflowing");
                assert!(available < needed);
            }
            other => panic!("expected NotEnoughSpace, got {other:?}"),
        }
    }

    #[test]
    fn only_new_files_and_their_empty_folders_are_removed() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("out");
        fs::create_dir_all(dest.join("sub")).unwrap();
        fs::write(dest.join("sub").join("new.txt"), b"new").unwrap();
        fs::write(dest.join("kept.txt"), b"was there").unwrap();
        let created = [
            Created {
                path: dest.join("kept.txt"),
                existed: true,
            },
            Created {
                path: dest.join("sub").join("new.txt"),
                existed: false,
            },
        ];
        remove_created(&created, &dest);
        assert!(dest.join("kept.txt").is_file());
        assert!(!dest.join("sub").exists());
        assert!(dest.is_dir(), "the destination itself stays");
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
