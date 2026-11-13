//! PAR2 verification and repair through an external executable
//! (`par2cmdline` or `par2cmdline-turbo`).
//!
//! `par2 repair` verifies first and only repairs when needed, so a single
//! process covers both steps. Progress is read from its standard output.

use std::ffi::OsStr;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::Duration;

/// How often the cancel flag is checked while `par2` runs.
const POLL_INTERVAL: Duration = Duration::from_millis(150);

/// Phase reported by [`verify_and_repair`] progress callbacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Loading and verifying files.
    Verify,
    /// Rebuilding damaged or missing files.
    Repair,
}

/// Successful end states of a `par2` run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Every file is correct.
    AllCorrect,
    /// Damaged files were repaired.
    Repaired,
    /// Not enough recovery blocks; `details` holds par2's explanation.
    NotPossible {
        /// Explanation lines printed by par2 (may be empty).
        details: String,
    },
    /// The user cancelled the run.
    Cancelled,
}

/// Errors raised while running `par2`.
#[derive(Debug, thiserror::Error)]
pub enum Par2Error {
    /// The executable could not be started.
    #[error("cannot start {exe}: {source}")]
    Spawn {
        /// Executable path.
        exe: PathBuf,
        /// Underlying error.
        #[source]
        source: io::Error,
    },
    /// The process could not be waited for.
    #[error("par2 process error: {0}")]
    Wait(#[source] io::Error),
    /// par2 exited with an unexpected status.
    #[error("par2 failed (exit code {code:?}): {message}")]
    Failed {
        /// Exit code, if any.
        code: Option<i32>,
        /// First non-empty line printed on stderr.
        message: String,
    },
}

#[cfg(windows)]
fn hide_console(cmd: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    cmd.creation_flags(CREATE_NO_WINDOW);
}
#[cfg(not(windows))]
fn hide_console(_: &mut Command) {}

/// Last percentage found in a line (`"Repairing: 42.5%"` → `42.5`).
pub fn last_percent(line: &str) -> Option<f32> {
    let idx = line.rfind('%')?;
    let head = line.get(..idx)?;
    let start = head
        .rfind(|c: char| !(c.is_ascii_digit() || c == '.'))
        .map_or(0, |i| i + 1);
    head.get(start..)?.parse().ok()
}

/// Runs `par2 r -- <index>` in the index folder.
pub fn verify_and_repair(
    par2_exe: &Path,
    index: &Path,
    cancel: &AtomicBool,
    mut progress: impl FnMut(Phase, f32),
    mut log: impl FnMut(String),
) -> Result<Outcome, Par2Error> {
    let dir = index.parent().unwrap_or_else(|| Path::new("."));
    let mut cmd = Command::new(par2_exe);
    cmd.arg("r")
        .arg("--")
        .arg(index.file_name().unwrap_or_else(|| OsStr::new("")))
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    hide_console(&mut cmd);
    let mut child = cmd.spawn().map_err(|source| Par2Error::Spawn {
        exe: par2_exe.to_path_buf(),
        source,
    })?;

    let (tx, rx) = mpsc::channel::<String>();
    let stdout_reader = child
        .stdout
        .take()
        .map(|out| std::thread::spawn(move || split_lines(out, &tx)));
    let stderr_reader = child.stderr.take().map(|mut err| {
        std::thread::spawn(move || {
            let mut s = String::new();
            let _ = err.read_to_string(&mut s);
            s
        })
    });

    let mut parser = OutputParser::default();
    loop {
        match rx.recv_timeout(POLL_INTERVAL) {
            Ok(line) => parser.line(&line, &mut progress, &mut log),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
        if cancel.load(Ordering::Relaxed) {
            kill(&mut child);
            if let Some(h) = stdout_reader {
                let _ = h.join();
            }
            return Ok(Outcome::Cancelled);
        }
    }
    if let Some(h) = stdout_reader {
        let _ = h.join();
    }
    let status = child.wait().map_err(Par2Error::Wait)?;
    let stderr = stderr_reader.and_then(|h| h.join().ok()).unwrap_or_default();

    // par2 exit codes: 0 = success, 1 = repair possible (not done), 2 = repair impossible.
    match status.code() {
        Some(0) if parser.repaired => Ok(Outcome::Repaired),
        Some(0) => Ok(Outcome::AllCorrect),
        Some(2) => Ok(Outcome::NotPossible {
            details: parser.summary.join(" "),
        }),
        _ if parser.not_possible => Ok(Outcome::NotPossible {
            details: parser.summary.join(" "),
        }),
        code => Err(Par2Error::Failed {
            code,
            message: stderr
                .lines()
                .map(str::trim)
                .find(|l| !l.is_empty())
                .unwrap_or_default()
                .to_owned(),
        }),
    }
}

fn kill(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

/// Splits a byte stream on `\r` and `\n`: par2 redraws its progress with `\r`.
fn split_lines(mut input: impl Read, tx: &mpsc::Sender<String>) {
    let mut buf = [0u8; 4096];
    let mut pending = Vec::new();
    loop {
        let n = match input.read(&mut buf) {
            Ok(0) | Err(_) => break,
            Ok(n) => n,
        };
        for &b in buf.get(..n).unwrap_or_default() {
            if b == b'\r' || b == b'\n' {
                if !pending.is_empty() {
                    let _ = tx.send(String::from_utf8_lossy(&pending).into_owned());
                    pending.clear();
                }
            } else {
                pending.push(b);
            }
        }
    }
    if !pending.is_empty() {
        let _ = tx.send(String::from_utf8_lossy(&pending).into_owned());
    }
}

/// Interprets par2's text output, one line at a time.
#[derive(Debug, Default)]
struct OutputParser {
    repairing: bool,
    repaired: bool,
    not_possible: bool,
    summary: Vec<String>,
}

impl OutputParser {
    fn line(&mut self, line: &str, progress: &mut impl FnMut(Phase, f32), log: &mut impl FnMut(String)) {
        let t = line.trim();
        if t.is_empty() {
            return;
        }
        if t.starts_with("Repairing") {
            self.repairing = true;
        }
        if let Some(p) = last_percent(t) {
            let phase = if self.repairing {
                Phase::Repair
            } else {
                Phase::Verify
            };
            progress(phase, (p / 100.0).clamp(0.0, 1.0));
            return;
        }
        let lower = t.to_lowercase();
        if lower.contains("repair complete") {
            self.repaired = true;
        }
        if lower.contains("repair is not possible") {
            self.not_possible = true;
        }
        if lower.starts_with("target:") && !lower.ends_with("found.") {
            log(t.to_owned());
        }
        if lower.contains("you need") || lower.contains("repair is not possible") {
            self.summary.push(t.to_owned());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent() {
        assert_eq!(last_percent("Repairing: 42.5%"), Some(42.5));
        assert_eq!(last_percent("Scanning: \"a.rar\": 7.0%"), Some(7.0));
        assert_eq!(last_percent("100%"), Some(100.0));
        assert_eq!(last_percent("Loading."), None);
        assert_eq!(last_percent("%"), None);
    }

    #[test]
    fn parser_tracks_phases_and_results() {
        let mut p = OutputParser::default();
        let mut events = Vec::new();
        let mut logs = Vec::new();
        for l in [
            "Loading: 50.0%",
            "Target: \"a.rar\" - found.",
            "Target: \"b.rar\" - damaged. Found 9 of 10 data blocks.",
            "Repairing: 30.0%",
            "Repair complete.",
        ] {
            p.line(l, &mut |ph, v| events.push((ph, v)), &mut |s| logs.push(s));
        }
        assert_eq!(events, vec![(Phase::Verify, 0.5), (Phase::Repair, 0.3)]);
        assert_eq!(logs.len(), 1);
        assert!(p.repaired);
        assert!(!p.not_possible);
    }

    #[test]
    fn split_on_carriage_returns() {
        let (tx, rx) = mpsc::channel();
        split_lines(&b"a\r\rb\r\nc"[..], &tx);
        drop(tx);
        assert_eq!(rx.iter().collect::<Vec<_>>(), vec!["a", "b", "c"]);
    }
}
