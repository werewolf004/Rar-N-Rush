//! One instance at a time.
//!
//! The first instance holds an exclusive lock on `rar-n-rush.lock` in the
//! settings folder for as long as it runs; the operating system releases it
//! when the process ends, even after a crash. A second start finds the lock
//! taken, drops a `rar-n-rush.show` file next to it and exits; the first
//! instance sees that file and brings its window to the front.
//!
//! The lock is per settings folder: two copies of the program with separate
//! settings (for example a portable copy) may still run side by side, since
//! they would not share any state.

use std::fs::{self, File, OpenOptions, TryLockError};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

/// Lock file held by the running instance.
pub const LOCK_FILE: &str = "rar-n-rush.lock";
/// File created by a second start to ask the running instance to show itself.
pub const SHOW_FILE: &str = "rar-n-rush.show";
/// How often the running instance looks for [`SHOW_FILE`].
const WATCH_INTERVAL: Duration = Duration::from_millis(300);

/// Errors raised while taking the instance lock.
#[derive(Debug, thiserror::Error)]
pub enum InstanceError {
    /// The lock file could not be created or locked.
    #[error("cannot lock {path}: {source}")]
    Lock {
        /// Lock file.
        path: PathBuf,
        /// Underlying error.
        #[source]
        source: io::Error,
    },
}

/// Result of [`acquire`].
#[derive(Debug)]
pub enum Instance {
    /// This is the only instance; keep the guard alive while the program runs.
    First(Guard),
    /// Another instance already runs with the same settings folder.
    AlreadyRunning,
}

/// Holds the instance lock; dropping it releases the lock.
#[derive(Debug)]
pub struct Guard {
    _file: File,
    dir: PathBuf,
}

/// Takes the instance lock in `dir` (created if needed).
pub fn acquire(dir: &Path) -> Result<Instance, InstanceError> {
    let path = dir.join(LOCK_FILE);
    let lock_error = |source| InstanceError::Lock {
        path: path.clone(),
        source,
    };
    fs::create_dir_all(dir).map_err(lock_error)?;
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&path)
        .map_err(lock_error)?;
    match file.try_lock() {
        Ok(()) => {
            // A request left over from before this start means nothing now.
            let _ = fs::remove_file(dir.join(SHOW_FILE));
            Ok(Instance::First(Guard {
                _file: file,
                dir: dir.to_path_buf(),
            }))
        }
        Err(TryLockError::WouldBlock) => Ok(Instance::AlreadyRunning),
        Err(TryLockError::Error(source)) => Err(lock_error(source)),
    }
}

/// Asks the instance running with the settings folder `dir` to show its window.
pub fn request_show(dir: &Path) -> io::Result<()> {
    fs::write(dir.join(SHOW_FILE), b"")
}

impl Guard {
    /// Starts watching for show requests; `wake` is called for each one (to
    /// repaint the window, even while it is hidden in the notification area).
    /// The returned value keeps the lock: the program stays the single instance
    /// for as long as it is alive.
    pub fn watch(self, wake: impl Fn() + Send + 'static) -> ShowRequests {
        let show_file = self.dir.join(SHOW_FILE);
        let requests = ShowRequests {
            pending: Arc::new(AtomicBool::new(false)),
            stop: Arc::new(AtomicBool::new(false)),
            _guard: self,
        };
        let (pending, stop) = (Arc::clone(&requests.pending), Arc::clone(&requests.stop));
        thread::spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                if show_file.exists() {
                    let _ = fs::remove_file(&show_file);
                    pending.store(true, Ordering::Relaxed);
                    wake();
                }
                thread::sleep(WATCH_INTERVAL);
            }
        });
        requests
    }
}

/// Show requests from later starts; dropping it stops the watch and releases
/// the lock.
#[derive(Debug)]
pub struct ShowRequests {
    pending: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    _guard: Guard,
}

impl ShowRequests {
    /// `true` once per request received since the last call.
    pub fn take(&self) -> bool {
        self.pending.swap(false, Ordering::Relaxed)
    }
}

impl Drop for ShowRequests {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;
    use std::time::Instant;

    #[test]
    fn second_instance_is_detected_until_the_first_ends() {
        let dir = tempfile::tempdir().unwrap();
        let first = acquire(dir.path()).unwrap();
        assert!(matches!(first, Instance::First(_)));
        assert!(matches!(acquire(dir.path()).unwrap(), Instance::AlreadyRunning));
        drop(first);
        assert!(matches!(acquire(dir.path()).unwrap(), Instance::First(_)));
    }

    #[test]
    fn separate_settings_folders_do_not_block_each_other() {
        let (a, b) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        let _first = acquire(a.path()).unwrap();
        assert!(matches!(acquire(b.path()).unwrap(), Instance::First(_)));
    }

    #[test]
    fn show_request_reaches_the_running_instance() {
        let dir = tempfile::tempdir().unwrap();
        let Instance::First(guard) = acquire(dir.path()).unwrap() else {
            panic!("first instance expected");
        };
        let wakes = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&wakes);
        let requests = guard.watch(move || {
            counter.fetch_add(1, Ordering::Relaxed);
        });
        assert!(!requests.take());

        request_show(dir.path()).unwrap();
        let start = Instant::now();
        while !requests.take() {
            assert!(start.elapsed() < Duration::from_secs(5), "request not seen");
            thread::sleep(Duration::from_millis(20));
        }
        assert_eq!(wakes.load(Ordering::Relaxed), 1);
        assert!(!dir.path().join(SHOW_FILE).exists(), "request consumed");
        assert!(!requests.take(), "one request, one show");
    }

    #[test]
    fn stale_request_is_ignored_at_start() {
        let dir = tempfile::tempdir().unwrap();
        request_show(dir.path()).unwrap();
        let Instance::First(_guard) = acquire(dir.path()).unwrap() else {
            panic!("first instance expected");
        };
        assert!(!dir.path().join(SHOW_FILE).exists());
    }
}
