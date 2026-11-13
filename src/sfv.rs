//! Verification of the files listed in an SFV file (CRC32).

use crate::scan::parse_sfv;
use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

const BUFFER_SIZE: usize = 1 << 20;

/// A problem found on one file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Problem {
    /// The file does not exist.
    Missing(String),
    /// The file content does not match the expected CRC32.
    Mismatch(String),
    /// The file exists but could not be read.
    Unreadable(String),
}

impl Problem {
    /// File name concerned.
    pub fn file(&self) -> &str {
        match self {
            Self::Missing(n) | Self::Mismatch(n) | Self::Unreadable(n) => n,
        }
    }
}

/// Result of a verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Every listed file matches.
    Ok,
    /// At least one file is missing or damaged.
    Bad(Vec<Problem>),
    /// The user cancelled the run.
    Cancelled,
}

/// Errors raised while verifying.
#[derive(Debug, thiserror::Error)]
pub enum SfvError {
    /// The SFV file itself could not be read.
    #[error("cannot read SFV file {path}: {source}")]
    Read {
        /// SFV file.
        path: PathBuf,
        /// Underlying error.
        #[source]
        source: io::Error,
    },
}

/// Verifies every file listed in `sfv`, relative to the SFV folder.
pub fn verify(sfv: &Path, cancel: &AtomicBool, mut progress: impl FnMut(f32)) -> Result<Outcome, SfvError> {
    let dir = sfv.parent().unwrap_or_else(|| Path::new("."));
    let entries = parse_sfv(sfv).map_err(|source| SfvError::Read {
        path: sfv.to_path_buf(),
        source,
    })?;
    let total: u64 = entries
        .iter()
        .filter_map(|(n, _)| dir.join(n).metadata().ok().map(|m| m.len()))
        .sum::<u64>()
        .max(1);
    let mut done = 0u64;
    let mut problems = Vec::new();
    let mut buf = vec![0u8; BUFFER_SIZE];
    for (name, expected) in entries {
        let mut file = match File::open(dir.join(&name)) {
            Ok(f) => f,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                problems.push(Problem::Missing(name));
                continue;
            }
            Err(_) => {
                problems.push(Problem::Unreadable(name));
                continue;
            }
        };
        let mut hasher = crc32fast::Hasher::new();
        let mut readable = true;
        loop {
            if cancel.load(Ordering::Relaxed) {
                return Ok(Outcome::Cancelled);
            }
            match file.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    hasher.update(buf.get(..n).unwrap_or_default());
                    done += n as u64;
                    #[allow(clippy::cast_precision_loss)] // progress ratio only
                    progress(done as f32 / total as f32);
                }
                Err(_) => {
                    readable = false;
                    break;
                }
            }
        }
        if !readable {
            problems.push(Problem::Unreadable(name));
        } else if hasher.finalize() != expected {
            problems.push(Problem::Mismatch(name));
        }
    }
    Ok(if problems.is_empty() {
        Outcome::Ok
    } else {
        Outcome::Bad(problems)
    })
}
