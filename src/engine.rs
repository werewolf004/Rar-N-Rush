//! Background worker: scans the watched folder, waits until sets are complete,
//! then verifies, repairs and extracts them one at a time.
//!
//! The UI and the worker share an [`EngineState`] behind a mutex. The UI sends
//! [`Command`]s; the worker never blocks the UI thread.

use crate::extract::{self, ExtractError};
use crate::i18n::{Lang, Localize};
use crate::par2::{self, Par2Error};
use crate::scan::{self, FileSet, ScanError, SetKind, Signature};
use crate::settings::{Outcome, Settings, SettingsError, Store};
use crate::sfv::{self, SfvError};
use crate::sync::lock;
use crate::tidy::{self, NotMoved, TidyError};
use std::collections::{HashMap, HashSet};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

/// Maximum number of log lines kept in memory.
pub const LOG_CAPACITY: usize = 2000;
/// Maximum nesting depth for archives inside archives.
const MAX_NESTING: u32 = 3;
const COMMAND_POLL: Duration = Duration::from_millis(500);

/// Processing state of a set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Files are still arriving.
    Waiting,
    /// Complete and waiting for its turn.
    Queued,
    /// PAR2 or SFV verification running.
    Verifying,
    /// PAR2 repair running.
    Repairing,
    /// RAR extraction running.
    Extracting,
    /// A password is required.
    NeedPassword,
    /// Not enough PAR2 recovery blocks; waiting for more files.
    RepairImpossible,
    /// Finished successfully.
    Done,
    /// Failed; see the job detail.
    Failed,
    /// Skipped or cancelled by the user.
    Skipped,
}

impl Status {
    /// `true` while the worker is processing the set.
    pub fn is_active(self) -> bool {
        matches!(self, Self::Verifying | Self::Repairing | Self::Extracting)
    }
}

/// Why a set failed.
#[derive(Debug, Clone, thiserror::Error)]
pub enum JobError {
    /// No `par2.exe` could be found.
    #[error("par2.exe not found")]
    Par2NotFound,
    /// par2 could not run.
    #[error(transparent)]
    Par2(Arc<Par2Error>),
    /// The SFV file could not be read.
    #[error(transparent)]
    Sfv(Arc<SfvError>),
    /// Some files do not match the SFV.
    #[error("SFV check failed: {}", .0.iter().map(sfv::Problem::file).collect::<Vec<_>>().join(", "))]
    SfvMismatch(Vec<sfv::Problem>),
    /// The first RAR volume is missing.
    #[error("first RAR volume not found")]
    FirstVolumeMissing,
    /// Extraction failed.
    #[error(transparent)]
    Extract(Arc<ExtractError>),
}

/// Extra information shown next to a set.
#[derive(Debug, Clone, Default)]
pub enum Detail {
    /// Nothing to show.
    #[default]
    None,
    /// New files arrived; the set will be tried again.
    NewFiles,
    /// PAR2 verification running.
    VerifyingPar2,
    /// SFV verification running.
    VerifyingSfv,
    /// Extraction running into this folder.
    ExtractingTo(PathBuf),
    /// PAR2 repair impossible; par2's explanation.
    WaitingForPar2(String),
    /// Waiting for the user to type a password.
    EnterPassword,
    /// The typed password was wrong.
    WrongPassword,
    /// Extraction finished.
    Extracted {
        /// Number of files extracted.
        count: usize,
        /// Files already in the destination, kept as they were (overwrite off).
        kept: usize,
        /// Destination folder.
        dest: PathBuf,
    },
    /// Verified; nothing to extract.
    NothingToExtract,
    /// Skipped by the user.
    SkippedByUser,
    /// Cancelled by the user while running.
    Cancelled,
    /// Failure reason.
    Error(JobError),
}

/// Event written to the log.
#[derive(Debug, Clone)]
pub enum Notice {
    /// The engine started.
    Started,
    /// Monitoring was started.
    MonitoringStarted,
    /// Monitoring was paused.
    MonitoringPaused,
    /// The watched folder is missing; monitoring paused.
    WatchDirMissing(PathBuf),
    /// The watched folder could not be scanned.
    ScanFailed(Arc<ScanError>),
    /// Processing of a set started.
    Processing {
        /// Set name.
        name: String,
        /// Set kind.
        kind: SetKind,
    },
    /// A noteworthy line printed by par2.
    Par2Output(String),
    /// PAR2: every file is correct.
    AllCorrect {
        /// Set name.
        name: String,
    },
    /// PAR2: damaged files were repaired.
    Repaired {
        /// Set name.
        name: String,
    },
    /// PAR2: repair impossible for now.
    RepairImpossible {
        /// Set name.
        name: String,
    },
    /// SFV check passed.
    SfvOk {
        /// Set name.
        name: String,
    },
    /// A password is required.
    PasswordRequired {
        /// Set name.
        name: String,
    },
    /// An archive was found inside an extracted archive.
    NestedArchive {
        /// File name of the nested archive.
        file: String,
    },
    /// Source files were moved to the recycle bin.
    SourcesTrashed {
        /// Number of files.
        count: usize,
    },
    /// Source files could not be moved to the recycle bin.
    TrashFailed(String),
    /// Extra files of a set were moved to the destination.
    ExtraFilesMoved {
        /// Number of files.
        count: usize,
        /// Destination folder.
        dest: PathBuf,
    },
    /// Extra files were left in place.
    ExtraFilesKept {
        /// Set name.
        name: String,
        /// Why they were not moved.
        reason: NotMoved,
    },
    /// Empty folders were moved to the recycle bin.
    EmptyFoldersTrashed {
        /// Number of folders.
        count: usize,
    },
    /// Moving extra files or removing empty folders failed.
    TidyFailed(Arc<TidyError>),
    /// A set processed before is found again: it is processed again (files
    /// already extracted are kept or overwritten, as set in the settings).
    ProcessingAgain {
        /// Set name.
        name: String,
    },
    /// Files already in the destination were kept (overwriting is off).
    ExistingKept {
        /// Set name.
        name: String,
        /// Number of files kept.
        count: usize,
    },
    /// A set finished successfully.
    Finished {
        /// Set name.
        name: String,
    },
    /// A set failed.
    Failed {
        /// Set name.
        name: String,
        /// Reason.
        error: JobError,
    },
    /// Processing was cancelled.
    Cancelled {
        /// Set name.
        name: String,
    },
    /// Persistent state could not be written.
    StateError(Arc<SettingsError>),
}

impl Notice {
    /// `true` for notices shown as errors.
    pub fn is_error(&self) -> bool {
        matches!(
            self,
            Self::WatchDirMissing(_)
                | Self::ScanFailed(_)
                | Self::RepairImpossible { .. }
                | Self::PasswordRequired { .. }
                | Self::TrashFailed(_)
                | Self::TidyFailed(_)
                | Self::Failed { .. }
                | Self::StateError(_)
        )
    }
}

/// A set tracked by the engine.
#[derive(Debug, Clone)]
pub struct Job {
    /// Identifier used by [`Command`]s.
    pub id: u64,
    /// Files of the set.
    pub set: FileSet,
    /// Current state.
    pub status: Status,
    /// Progress of the current step, from 0 to 1.
    pub progress: f32,
    /// Extra information.
    pub detail: Detail,
    changed_at: SystemTime,
    failed_signature: Option<Signature>,
    password: Option<String>,
    /// Its files were not found by the last scan. If they come back after it
    /// was processed, they are a new copy, processed again.
    missing: bool,
}

/// One log line.
#[derive(Debug, Clone)]
pub struct LogLine {
    /// Local time, `HH:MM:SS`.
    pub time: String,
    /// Event.
    pub notice: Notice,
}

/// State shared between the worker and the UI.
#[derive(Debug)]
pub struct EngineState {
    /// Known sets, in discovery order.
    pub jobs: Vec<Job>,
    /// Recent log lines (at most [`LOG_CAPACITY`]).
    pub log: Vec<LogLine>,
    /// Monitoring is running.
    pub running: bool,
    /// Current settings.
    pub settings: Settings,
    /// Time of the last scan.
    pub last_scan: Option<Instant>,
}

/// Requests sent to the worker.
#[derive(Debug, Clone)]
pub enum Command {
    /// Scan now.
    ScanNow,
    /// Start or pause monitoring.
    SetRunning(bool),
    /// Skip a set (cancels it if running) and never process it again.
    Skip(u64),
    /// Process a set again.
    Retry(u64),
    /// Provide the password of a set.
    Password(u64, String),
    /// Remove a set from the list for this session.
    Forget(u64),
    /// Replace the settings.
    UpdateSettings(Settings),
}

/// Handle to the background worker. Dropping it stops the worker.
pub struct Engine {
    state: Arc<Mutex<EngineState>>,
    cancel: Arc<AtomicBool>,
    tx: Sender<Command>,
}

impl Engine {
    /// Starts the worker thread. `repaint` is called whenever the state changes.
    pub fn start(
        settings: Settings,
        store: Store,
        repaint: impl Fn() + Send + Sync + 'static,
    ) -> io::Result<Self> {
        let settings = settings.validated();
        let running = settings.autostart && settings.watch_path().is_some();
        let state = Arc::new(Mutex::new(EngineState {
            jobs: Vec::new(),
            log: Vec::new(),
            running,
            settings,
            last_scan: None,
        }));
        let cancel = Arc::new(AtomicBool::new(false));
        let (tx, rx) = std::sync::mpsc::channel();
        let mut worker = Worker {
            state: Arc::clone(&state),
            cancel: Arc::clone(&cancel),
            repaint: Box::new(repaint),
            processed: HashMap::new(),
            ignored: HashSet::new(),
            store,
            next_id: 1,
            scan_requested: true,
        };
        match worker.store.load_processed() {
            Ok(keys) => worker.processed = keys,
            Err(e) => worker.notice(Notice::StateError(Arc::new(e))),
        }
        std::thread::Builder::new()
            .name("engine".into())
            .spawn(move || worker.run(&rx))?;
        Ok(Self { state, cancel, tx })
    }

    /// Runs `f` with read access to the shared state.
    pub fn with_state<R>(&self, f: impl FnOnce(&EngineState) -> R) -> R {
        f(&lock(&self.state))
    }

    /// Sends a command to the worker.
    pub fn send(&self, command: Command) {
        if let Command::Skip(id) = &command {
            let active = self.with_state(|s| s.jobs.iter().any(|j| j.id == *id && j.status.is_active()));
            if active {
                self.cancel.store(true, Ordering::Relaxed);
            }
        }
        // The worker only exits once this sender is dropped, so this cannot fail
        // while `self` is alive.
        let _ = self.tx.send(command);
    }
}

struct Worker {
    state: Arc<Mutex<EngineState>>,
    cancel: Arc<AtomicBool>,
    repaint: Box<dyn Fn() + Send + Sync>,
    /// Sets handled in this or an earlier session, and how they ended.
    processed: HashMap<String, Outcome>,
    ignored: HashSet<String>,
    store: Store,
    next_id: u64,
    scan_requested: bool,
}

enum Stop {
    Cancelled,
    NeedPassword,
    Failed(JobError),
}

impl Worker {
    fn notice(&self, notice: Notice) {
        let now = crate::clock::now();
        // The file gets English text: it is meant to be attached to bug reports.
        // Notices never carry a password, so none can reach the file.
        let file_line = lock(&self.state)
            .settings
            .log_to_file
            .then(|| format!("{}  {}", now.log_line(), notice.localize(Lang::En)));
        let mut state = lock(&self.state);
        state.log.push(LogLine {
            time: now.time_of_day(),
            notice,
        });
        let excess = state.log.len().saturating_sub(LOG_CAPACITY);
        state.log.drain(..excess);
        drop(state);
        if let Some(line) = file_line {
            // A log that cannot be written must not stop the processing, nor log
            // about itself in a loop.
            let _ = self.store.append_log(&line);
        }
        (self.repaint)();
    }

    fn update(&self, id: u64, f: impl FnOnce(&mut Job)) {
        let mut state = lock(&self.state);
        if let Some(job) = state.jobs.iter_mut().find(|j| j.id == id) {
            f(job);
        }
        drop(state);
        (self.repaint)();
    }

    fn remember(&mut self, key: &str, outcome: Outcome) {
        self.processed.insert(key.to_owned(), outcome);
        if let Err(e) = self.store.append_processed(key, outcome) {
            self.notice(Notice::StateError(Arc::new(e)));
        }
    }

    fn run(mut self, rx: &Receiver<Command>) {
        self.notice(Notice::Started);
        loop {
            match rx.recv_timeout(COMMAND_POLL) {
                Ok(cmd) => self.handle(cmd),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return,
            }
            while let Ok(cmd) = rx.try_recv() {
                self.handle(cmd);
            }
            let (running, interval, last_scan) = {
                let s = lock(&self.state);
                (s.running, s.settings.scan_interval_secs, s.last_scan)
            };
            if !running {
                continue;
            }
            let due = last_scan.is_none_or(|t| t.elapsed() >= Duration::from_secs(interval));
            if due || self.scan_requested {
                self.scan_requested = false;
                self.scan();
            }
            let next = lock(&self.state)
                .jobs
                .iter()
                .find(|j| j.status == Status::Queued)
                .cloned();
            if let Some(job) = next {
                self.process(&job);
                self.scan_requested = true; // processing changed the folder
            }
        }
    }

    fn handle(&mut self, cmd: Command) {
        match cmd {
            Command::ScanNow => self.scan_requested = true,
            Command::SetRunning(running) => {
                lock(&self.state).running = running;
                self.notice(if running {
                    Notice::MonitoringStarted
                } else {
                    Notice::MonitoringPaused
                });
                self.scan_requested = running;
            }
            Command::Skip(id) => {
                let key = lock(&self.state)
                    .jobs
                    .iter()
                    .find(|j| j.id == id)
                    .map(|j| j.set.key.clone());
                // Persist before showing the new status (see `finish`).
                if let Some(k) = key {
                    self.remember(&k, Outcome::Skipped);
                }
                self.update(id, |j| {
                    if !j.status.is_active() {
                        j.status = Status::Skipped;
                        j.detail = Detail::SkippedByUser;
                    }
                });
            }
            Command::Retry(id) => self.update(id, |j| {
                j.status = Status::Queued;
                j.failed_signature = None;
                j.detail = Detail::None;
                j.progress = 0.0;
            }),
            Command::Password(id, password) => self.update(id, |j| {
                j.password = Some(password);
                j.status = Status::Queued;
                j.detail = Detail::None;
            }),
            Command::Forget(id) => {
                let mut state = lock(&self.state);
                if let Some(pos) = state
                    .jobs
                    .iter()
                    .position(|j| j.id == id && !j.status.is_active())
                {
                    let job = state.jobs.remove(pos);
                    self.ignored.insert(job.set.key);
                }
                drop(state);
                (self.repaint)();
            }
            Command::UpdateSettings(new) => {
                let mut state = lock(&self.state);
                let new = new.validated();
                let folder_changed =
                    state.settings.watch_dir != new.watch_dir || state.settings.recursive != new.recursive;
                state.settings = new;
                if folder_changed {
                    state.jobs.retain(|j| j.status.is_active());
                    self.scan_requested = true;
                }
                drop(state);
                (self.repaint)();
            }
        }
    }

    fn scan(&mut self) {
        let settings = {
            let mut s = lock(&self.state);
            s.last_scan = Some(Instant::now());
            s.settings.clone()
        };
        let Some(root) = settings.watch_path() else { return };
        if !root.is_dir() {
            lock(&self.state).running = false;
            self.notice(Notice::WatchDirMissing(root));
            return;
        }
        let found = match scan::scan(&root, settings.recursive, settings.extract_path().as_deref()) {
            Ok(found) => found,
            Err(e) => {
                self.notice(Notice::ScanFailed(Arc::new(e)));
                return;
            }
        };
        let now = SystemTime::now();
        let stable = Duration::from_secs(settings.stable_secs);

        let mut state = lock(&self.state);
        let mut seen = HashSet::new();
        // Written to the log once the state is unlocked.
        let mut notices = Vec::new();
        for set in found {
            seen.insert(set.key.clone());
            if let Some(job) = state.jobs.iter_mut().find(|j| j.set.key == set.key) {
                if job.status.is_active() {
                    continue;
                }
                let came_back = std::mem::take(&mut job.missing);
                if came_back && job.status == Status::Done {
                    // Its files went away after it was processed: this is a new copy.
                    job.status = Status::Waiting;
                    job.detail = Detail::NewFiles;
                    job.progress = 0.0;
                    job.changed_at = now;
                    notices.push(Notice::ProcessingAgain {
                        name: set.name.clone(),
                    });
                } else if job.set.signature != set.signature {
                    job.changed_at = now;
                    let retry = matches!(job.status, Status::Failed | Status::RepairImpossible)
                        && job.failed_signature.is_some_and(|s| s != set.signature);
                    if retry {
                        job.status = Status::Waiting;
                        job.detail = Detail::NewFiles;
                    }
                }
                job.set = set;
            } else if !self.ignored.contains(&set.key) {
                let Some((status, detail, notice)) = first_seen(self.processed.get(&set.key), &set.name)
                else {
                    continue;
                };
                notices.extend(notice);
                let id = self.next_id;
                self.next_id += 1;
                let changed_at = set.signature.newest.unwrap_or(now).min(now);
                state.jobs.push(Job {
                    id,
                    set,
                    status,
                    progress: 0.0,
                    detail,
                    changed_at,
                    failed_signature: None,
                    password: None,
                    missing: false,
                });
            }
        }
        // Drop waiting sets whose files disappeared; remember that the others
        // are gone, so that their files coming back count as a new copy.
        state
            .jobs
            .retain(|j| seen.contains(&j.set.key) || !matches!(j.status, Status::Waiting | Status::Queued));
        for job in state.jobs.iter_mut().filter(|j| !j.status.is_active()) {
            if !seen.contains(&job.set.key) {
                job.missing = true;
            }
        }
        // A set is ready once it has not changed for `stable` seconds.
        for job in state.jobs.iter_mut().filter(|j| j.status == Status::Waiting) {
            let quiet = now.duration_since(job.changed_at).unwrap_or_default();
            let newest = job
                .set
                .signature
                .newest
                .map_or(stable, |t| now.duration_since(t).unwrap_or_default());
            if quiet >= stable && newest >= stable {
                job.status = Status::Queued;
                job.detail = Detail::None;
            }
        }
        drop(state);
        for notice in notices {
            self.notice(notice);
        }
        (self.repaint)();
    }

    fn process(&mut self, job: &Job) {
        self.cancel.store(false, Ordering::Relaxed);
        let settings = lock(&self.state).settings.clone();
        let (id, set) = (job.id, &job.set);
        self.notice(Notice::Processing {
            name: set.name.clone(),
            kind: set.kind(),
        });

        match self.verify(id, set, &settings) {
            Ok(true) => {}
            Ok(false) => return, // status already set (repair impossible)
            Err(stop) => return self.stop(job, stop),
        }

        let firsts: Vec<PathBuf> = set
            .rar_groups
            .iter()
            .filter_map(|g| scan::rar_first_on_disk(&set.dir, g))
            .collect();
        if firsts.is_empty() {
            if set.rar_groups.is_empty() {
                return self.finish(job, Detail::NothingToExtract, &settings);
            }
            return self.stop(job, Stop::Failed(JobError::FirstVolumeMissing));
        }

        // Straight into the extraction folder (the watched folder by default).
        let mut dest = settings.extract_root().unwrap_or_else(|| set.dir.clone());
        if settings.create_subfolder {
            dest.push(sanitize_folder_name(&set.name));
        }
        let passwords: Vec<String> = job
            .password
            .iter()
            .chain(settings.passwords.iter())
            .filter(|p| !p.is_empty())
            .cloned()
            .collect();

        let (mut count, mut kept) = (0, 0);
        for first in &firsts {
            self.update(id, |j| {
                j.status = Status::Extracting;
                j.progress = 0.0;
                j.detail = Detail::ExtractingTo(dest.clone());
            });
            match self.extract_tree(id, first, &dest, &passwords, &settings, 0) {
                Ok((n, k)) => {
                    count += n;
                    kept += k;
                }
                Err(stop) => return self.stop(job, stop),
            }
        }
        if kept > 0 {
            self.notice(Notice::ExistingKept {
                name: set.name.clone(),
                count: kept,
            });
        }
        self.finish(job, Detail::Extracted { count, kept, dest }, &settings);
    }

    /// Runs PAR2 or SFV verification. `Ok(false)` means "stop here, status already set".
    fn verify(&self, id: u64, set: &FileSet, settings: &Settings) -> Result<bool, Stop> {
        if let Some(index) = &set.par2_index {
            let exe = settings
                .resolve_par2()
                .ok_or(Stop::Failed(JobError::Par2NotFound))?;
            self.update(id, |j| {
                j.status = Status::Verifying;
                j.progress = 0.0;
                j.detail = Detail::VerifyingPar2;
            });
            let outcome = par2::verify_and_repair(
                &exe,
                index,
                &self.cancel,
                |phase, p| {
                    self.update(id, |j| {
                        j.status = if phase == par2::Phase::Repair {
                            Status::Repairing
                        } else {
                            Status::Verifying
                        };
                        j.progress = p;
                    });
                },
                |line| self.notice(Notice::Par2Output(line)),
            )
            .map_err(|e| Stop::Failed(JobError::Par2(Arc::new(e))))?;
            match outcome {
                par2::Outcome::AllCorrect => self.notice(Notice::AllCorrect {
                    name: set.name.clone(),
                }),
                par2::Outcome::Repaired => self.notice(Notice::Repaired {
                    name: set.name.clone(),
                }),
                par2::Outcome::Cancelled => return Err(Stop::Cancelled),
                par2::Outcome::NotPossible { details } => {
                    self.notice(Notice::RepairImpossible {
                        name: set.name.clone(),
                    });
                    let signature = set.signature;
                    self.update(id, |j| {
                        j.status = Status::RepairImpossible;
                        j.detail = Detail::WaitingForPar2(details);
                        j.failed_signature = Some(signature);
                    });
                    return Ok(false);
                }
            }
        } else if let Some(sfv_path) = &set.sfv {
            self.update(id, |j| {
                j.status = Status::Verifying;
                j.progress = 0.0;
                j.detail = Detail::VerifyingSfv;
            });
            let outcome = sfv::verify(sfv_path, &self.cancel, |p| self.update(id, |j| j.progress = p))
                .map_err(|e| Stop::Failed(JobError::Sfv(Arc::new(e))))?;
            match outcome {
                sfv::Outcome::Ok => self.notice(Notice::SfvOk {
                    name: set.name.clone(),
                }),
                sfv::Outcome::Bad(problems) => return Err(Stop::Failed(JobError::SfvMismatch(problems))),
                sfv::Outcome::Cancelled => return Err(Stop::Cancelled),
            }
        }
        Ok(true)
    }

    /// Extracts an archive, then the RAR archives it contains (if enabled).
    /// Returns the number of files written and of existing files kept.
    fn extract_tree(
        &self,
        id: u64,
        first: &Path,
        dest: &Path,
        passwords: &[String],
        settings: &Settings,
        depth: u32,
    ) -> Result<(usize, usize), Stop> {
        let options = extract::Options {
            overwrite_existing: settings.overwrite_existing,
        };
        let outcome = extract::extract(first, dest, passwords, options, &self.cancel, |p| {
            self.update(id, |j| j.progress = p);
        })
        .map_err(|e| Stop::Failed(JobError::Extract(Arc::new(e))))?;
        let (files, mut kept) = match outcome {
            extract::Outcome::Extracted { files, kept } => (files, kept.len()),
            extract::Outcome::NeedPassword => return Err(Stop::NeedPassword),
            extract::Outcome::Cancelled => return Err(Stop::Cancelled),
        };
        let mut count = files.len();
        if settings.extract_nested && depth < MAX_NESTING {
            let names: Vec<String> = files.iter().map(|f| f.to_string_lossy().into_owned()).collect();
            for (_, inner_first) in scan::rar_first_volumes(names.iter().map(String::as_str)) {
                let inner = PathBuf::from(&inner_first);
                let inner_dir = inner.parent().unwrap_or(dest).to_path_buf();
                self.notice(Notice::NestedArchive {
                    file: scan::file_name(&inner),
                });
                let (extracted, inner_kept) =
                    self.extract_tree(id, &inner, &inner_dir, passwords, settings, depth + 1)?;
                kept += inner_kept;
                if let Some((base, _)) = scan::rar_part_info(&scan::file_name(&inner)) {
                    for volume in scan::rar_volumes_on_disk(&inner_dir, &base) {
                        if std::fs::remove_file(volume).is_ok() {
                            count = count.saturating_sub(1);
                        }
                    }
                }
                count += extracted;
            }
        }
        Ok((count, kept))
    }

    fn finish(&mut self, job: &Job, detail: Detail, settings: &Settings) {
        let set = &job.set;
        let root = settings.watch_path();
        if let (true, Some(root), Detail::Extracted { dest, .. }) =
            (settings.move_extra_files, &root, &detail)
        {
            self.move_extra_files(set, root, dest, settings.overwrite_existing);
        }
        let mut outcome = Outcome::Done;
        if settings.delete_after_success {
            // Verified non-RAR files (the useful content) are always kept.
            let sources = tidy::source_files(set);
            match trash::delete_all(&sources) {
                Ok(()) => {
                    if !sources.is_empty() {
                        outcome = Outcome::SourcesTrashed;
                    }
                    self.notice(Notice::SourcesTrashed { count: sources.len() });
                }
                Err(e) => self.notice(Notice::TrashFailed(e.to_string())),
            }
        }
        if let (true, Some(root)) = (settings.delete_empty_dirs, &root) {
            match tidy::remove_empty_dirs(&set.dir, root) {
                Ok(removed) if removed.is_empty() => {}
                Ok(removed) => self.notice(Notice::EmptyFoldersTrashed { count: removed.len() }),
                Err(e) => self.notice(Notice::TidyFailed(Arc::new(e))),
            }
        }
        // Persist first: if the app stops right after, the set must not be redone.
        self.remember(&set.key, outcome);
        self.notice(Notice::Finished {
            name: set.name.clone(),
        });
        self.update(job.id, |j| {
            j.status = Status::Done;
            j.progress = 1.0;
            j.detail = detail;
        });
    }

    /// Moves the files of the set's folder that are not sources (`.nfo`, samples…)
    /// to the destination. Stops at the first failure and reports it.
    fn move_extra_files(&self, set: &FileSet, root: &Path, dest: &Path, overwrite: bool) {
        let extras = match tidy::extra_files(set, root, dest) {
            Ok(Ok(extras)) => extras,
            Ok(Err(reason)) => {
                return self.notice(Notice::ExtraFilesKept {
                    name: set.name.clone(),
                    reason,
                });
            }
            Err(e) => return self.notice(Notice::TidyFailed(Arc::new(e))),
        };
        let mut count = 0;
        for file in &extras {
            match tidy::move_file(file, dest, overwrite) {
                Ok(Some(_)) => count += 1,
                Ok(None) => {} // already there, kept (overwrite off)
                Err(e) => {
                    self.notice(Notice::TidyFailed(Arc::new(e)));
                    break;
                }
            }
        }
        if count > 0 {
            self.notice(Notice::ExtraFilesMoved {
                count,
                dest: dest.to_path_buf(),
            });
        }
    }

    fn stop(&self, job: &Job, stop: Stop) {
        let name = job.set.name.clone();
        match stop {
            Stop::Cancelled => {
                self.cancel.store(false, Ordering::Relaxed);
                self.notice(Notice::Cancelled { name });
                self.update(job.id, |j| {
                    j.status = Status::Skipped;
                    j.detail = Detail::Cancelled;
                });
            }
            Stop::NeedPassword => {
                self.notice(Notice::PasswordRequired { name });
                let wrong = job.password.is_some();
                self.update(job.id, |j| {
                    j.status = Status::NeedPassword;
                    j.detail = if wrong {
                        Detail::WrongPassword
                    } else {
                        Detail::EnterPassword
                    };
                });
            }
            Stop::Failed(error) => {
                self.notice(Notice::Failed {
                    name,
                    error: error.clone(),
                });
                let signature = job.set.signature;
                self.update(job.id, |j| {
                    j.status = Status::Failed;
                    j.detail = Detail::Error(error);
                    j.failed_signature = Some(signature);
                });
            }
        }
    }
}

/// Status of a set seen for the first time in this session, from how it
/// ended before (if it did). Every set is processed, even one processed
/// before; only a set the user skipped is left alone (`None`).
fn first_seen(before: Option<&Outcome>, name: &str) -> Option<(Status, Detail, Option<Notice>)> {
    match before {
        None => Some((Status::Waiting, Detail::None, None)),
        Some(Outcome::Skipped) => None,
        Some(Outcome::Done | Outcome::SourcesTrashed) => Some((
            Status::Waiting,
            Detail::NewFiles,
            Some(Notice::ProcessingAgain {
                name: name.to_owned(),
            }),
        )),
    }
}

/// Turns a set name into a valid Windows folder name.
pub fn sanitize_folder_name(name: &str) -> String {
    const RESERVED: [&str; 22] = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9",
        "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    let cleaned: String = name
        .chars()
        .map(|c| {
            if matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') || c.is_control() {
                '_'
            } else {
                c
            }
        })
        .collect();
    let cleaned = cleaned.trim().trim_end_matches(['.', ' ']).to_owned();
    if cleaned.is_empty() || cleaned == "." || cleaned == ".." {
        return "extracted".into();
    }
    // Windows also reserves these names followed by an extension (`CON.txt`),
    // and ignores spaces before the dot (`CON .txt`).
    let stem = cleaned.split('.').next().unwrap_or_default().trim_end();
    if RESERVED.iter().any(|r| stem.eq_ignore_ascii_case(r)) {
        return format!("_{cleaned}");
    }
    cleaned
}

#[cfg(test)]
mod tests {
    use super::sanitize_folder_name;

    #[test]
    fn folder_names() {
        assert_eq!(sanitize_folder_name("Film: part 1?"), "Film_ part 1_");
        assert_eq!(sanitize_folder_name("a/b\\c"), "a_b_c");
        assert_eq!(sanitize_folder_name("name. . "), "name");
        assert_eq!(sanitize_folder_name(".."), "extracted");
        assert_eq!(sanitize_folder_name(""), "extracted");
        assert_eq!(sanitize_folder_name("con"), "_con");
        assert_eq!(sanitize_folder_name("Con.a"), "_Con.a");
        assert_eq!(sanitize_folder_name("NUL .tar"), "_NUL .tar");
        assert_eq!(sanitize_folder_name("Console.2026"), "Console.2026");
        assert_eq!(sanitize_folder_name("Vacances.2026"), "Vacances.2026");
    }
}
