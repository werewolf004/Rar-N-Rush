//! Optional check for a newer release on GitHub. Off by default: until the
//! user turns it on in the settings, Rar N Rush makes no network request.
//!
//! When it is on, the program asks GitHub for the number of the latest
//! release, about a minute after it starts and then at most once per chosen
//! period (day, week or month). Nothing else is sent, and nothing is
//! downloaded or installed: the window only says that a new version exists
//! and links to its release page.
//!
//! The request goes through `curl.exe`, which ships with Windows 10 and 11,
//! so the program itself carries no HTTP or TLS code (see `deny.toml`).
//! A failed check is silent and is tried again an hour later.

use crate::settings::UpdateFrequency;
use std::fmt;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::{Duration, Instant};

/// Project page, from `Cargo.toml`.
pub const REPO_URL: &str = env!("CARGO_PKG_REPOSITORY");
/// The only address ever contacted: latest release of the project on GitHub.
pub const LATEST_URL: &str = "https://api.github.com/repos/werewolf004/Rar-N-Rush/releases/latest";
/// Delay between the start of the program and the first check.
pub const FIRST_CHECK_DELAY: Duration = Duration::from_mins(1);
/// Delay before trying again after a failed check.
pub const RETRY_DELAY: Duration = Duration::from_hours(1);
/// Longest time a check may take.
const TIMEOUT_SECS: &str = "30";
/// Largest answer accepted, in bytes.
const MAX_ANSWER: &str = "1000000";

/// Why a check failed. Never shown as an error: the check is tried again later.
#[derive(Debug, thiserror::Error)]
pub enum UpdateError {
    /// `curl.exe` is missing (Windows older than 10 version 1803).
    #[error("curl.exe not found at {0}")]
    NoCurl(PathBuf),
    /// `curl.exe` could not be started.
    #[error("cannot run curl.exe: {0}")]
    Launch(#[from] std::io::Error),
    /// The request failed (no connection, timeout, HTTP error…).
    #[error("request failed ({code:?}): {message}")]
    Request {
        /// Exit code of `curl.exe`.
        code: Option<i32>,
        /// What `curl.exe` reported.
        message: String,
    },
    /// The answer holds no release number.
    #[error("no release number in the answer")]
    Unreadable,
}

/// A release number, `major.minor.patch`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version {
    /// Major number.
    pub major: u32,
    /// Minor number.
    pub minor: u32,
    /// Patch number.
    pub patch: u32,
}

impl Version {
    /// Reads `1.5.0` or `v1.5.0`. Anything else (pre-releases, extra parts,
    /// letters) is refused.
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.strip_prefix('v').unwrap_or(text);
        let mut parts = text.split('.').map(|part| {
            if part.is_empty() || part.len() > 9 || !part.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            part.parse().ok()
        });
        let version = Self {
            major: parts.next()??,
            minor: parts.next()??,
            patch: parts.next()??,
        };
        parts.next().is_none().then_some(version)
    }

    /// Version of this program.
    pub fn current() -> Self {
        Self::parse(env!("CARGO_PKG_VERSION")).unwrap_or(Self {
            major: 0,
            minor: 0,
            patch: 0,
        })
    }

    /// Release page of this version on GitHub.
    pub fn release_url(self) -> String {
        format!("{REPO_URL}/releases/tag/v{self}")
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Reads the release number (`"tag_name": "v1.5.0"`) from GitHub's answer.
pub fn parse_latest(answer: &str) -> Option<Version> {
    let (_, rest) = answer.split_once("\"tag_name\"")?;
    let rest = rest
        .trim_start()
        .strip_prefix(':')?
        .trim_start()
        .strip_prefix('"')?;
    let (tag, _) = rest.split_once('"')?;
    Version::parse(tag)
}

/// `true` when a check is due: never checked, or the period has passed since
/// the last one (`last` and `now` in seconds since 1970). A last check in the
/// future (clock moved back) also makes one due.
pub fn is_due(last: Option<u64>, now: u64, frequency: UpdateFrequency) -> bool {
    last.is_none_or(|last| last > now || now - last >= frequency.period().as_secs())
}

/// Seconds since 1970, from the system clock.
pub fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Something that can download a short text; tests use a fake one, so they
/// never go online.
pub trait Fetch: Send + Sync {
    /// Body of the answer to a GET request on `url`.
    fn get(&self, url: &str) -> Result<String, UpdateError>;
}

/// Downloads with the `curl.exe` shipped with Windows, HTTPS only, without
/// showing a console window.
#[derive(Debug, Clone, Copy, Default)]
pub struct Curl;

impl Curl {
    /// `%SystemRoot%\System32\curl.exe`: never a copy found through `PATH`.
    fn program() -> PathBuf {
        let root = std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into());
        PathBuf::from(root).join("System32").join("curl.exe")
    }
}

impl Fetch for Curl {
    fn get(&self, url: &str) -> Result<String, UpdateError> {
        let program = Self::program();
        if !program.is_file() {
            return Err(UpdateError::NoCurl(program));
        }
        let agent = format!("rar-n-rush/{}", env!("CARGO_PKG_VERSION"));
        let mut command = Command::new(&program);
        command.args([
            "--silent",
            "--show-error",
            "--fail",
            "--location",
            "--proto",
            "=https",
            "--proto-redir",
            "=https",
            "--max-time",
            TIMEOUT_SECS,
            "--max-filesize",
            MAX_ANSWER,
            "--header",
            "Accept: application/vnd.github+json",
            "--user-agent",
            &agent,
            "--",
            url,
        ]);
        hide_console(&mut command);
        let output = command.output()?;
        if !output.status.success() {
            return Err(UpdateError::Request {
                code: output.status.code(),
                message: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
            });
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }
}

/// Keeps `curl.exe` from opening a console window.
#[cfg(windows)]
fn hide_console(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn hide_console(_: &mut Command) {}

/// What the settings say about checking, read on each [`Checker::poll`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Schedule {
    /// The user turned checking on.
    pub enabled: bool,
    /// How often.
    pub frequency: UpdateFrequency,
    /// Last successful check, in seconds since 1970.
    pub last: Option<u64>,
}

/// Runs the checks in the background, one at a time, when they are due.
pub struct Checker {
    fetch: Arc<dyn Fetch>,
    /// No check starts before this time (first-check delay, then retry delay).
    not_before: Instant,
    pending: Option<Receiver<Result<Version, UpdateError>>>,
}

impl Checker {
    /// Checks through `fetch`, the first time [`FIRST_CHECK_DELAY`] from now.
    pub fn new(fetch: impl Fetch + 'static) -> Self {
        Self::with_delay(fetch, FIRST_CHECK_DELAY)
    }

    /// Same, with another first-check delay (tests).
    pub fn with_delay(fetch: impl Fetch + 'static, delay: Duration) -> Self {
        Self {
            fetch: Arc::new(fetch),
            not_before: Instant::now() + delay,
            pending: None,
        }
    }

    /// Called often (each frame). Starts a check in the background when one is
    /// due, and returns its result once it has finished: the latest release
    /// number, or why it could not be read. `wake` is called when the result
    /// is ready.
    pub fn poll(
        &mut self,
        schedule: Schedule,
        wake: impl FnOnce() + Send + 'static,
    ) -> Option<Result<Version, UpdateError>> {
        if let Some(pending) = &self.pending {
            let result = match pending.try_recv() {
                Ok(result) => result,
                Err(TryRecvError::Empty) => return None,
                Err(TryRecvError::Disconnected) => Err(UpdateError::Unreadable),
            };
            self.pending = None;
            self.not_before = Instant::now() + RETRY_DELAY;
            return Some(result);
        }
        if schedule.enabled
            && Instant::now() >= self.not_before
            && is_due(schedule.last, unix_now(), schedule.frequency)
        {
            let (tx, rx) = mpsc::channel();
            let fetch = Arc::clone(&self.fetch);
            std::thread::spawn(move || {
                let result = fetch
                    .get(LATEST_URL)
                    .and_then(|answer| parse_latest(&answer).ok_or(UpdateError::Unreadable));
                let _ = tx.send(result);
                wake();
            });
            self.pending = Some(rx);
        }
        None
    }

    /// Time until a check may start, to wake the window up then.
    pub fn wait(&self) -> Duration {
        self.not_before.saturating_duration_since(Instant::now())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[test]
    fn versions_are_read_strictly() {
        let v = |a, b, c| Version {
            major: a,
            minor: b,
            patch: c,
        };
        assert_eq!(Version::parse("1.5.0"), Some(v(1, 5, 0)));
        assert_eq!(Version::parse("v10.20.30"), Some(v(10, 20, 30)));
        for bad in [
            "",
            "v",
            "1.5",
            "1.5.0.1",
            "1.5.0-rc1",
            "1..0",
            "a.b.c",
            "vv1.0.0",
            "1.5.+0",
        ] {
            assert_eq!(Version::parse(bad), None, "{bad}");
        }
        assert!(v(1, 10, 0) > v(1, 9, 9));
        assert!(v(2, 0, 0) > v(1, 99, 99));
        assert_eq!(v(1, 5, 0).to_string(), "1.5.0");
        assert_eq!(Version::current().to_string(), env!("CARGO_PKG_VERSION"));
        assert_eq!(
            v(1, 5, 0).release_url(),
            "https://github.com/werewolf004/Rar-N-Rush/releases/tag/v1.5.0"
        );
    }

    #[test]
    fn latest_release_is_read_from_the_answer() {
        let answer = r#"{"url":"x","html_url":"y","id":1,"tag_name" : "v1.5.0","name":"v1.5.0"}"#;
        assert_eq!(
            parse_latest(answer).map(|v| v.to_string()).as_deref(),
            Some("1.5.0")
        );
        assert_eq!(parse_latest(r#"{"tag_name":"nightly"}"#), None);
        assert_eq!(parse_latest(r#"{"message":"Not Found"}"#), None);
        assert_eq!(parse_latest(""), None);
    }

    #[test]
    fn only_this_project_is_contacted() {
        let repo = REPO_URL.strip_prefix("https://github.com/").unwrap();
        assert_eq!(
            LATEST_URL,
            format!("https://api.github.com/repos/{repo}/releases/latest")
        );
    }

    #[test]
    fn checks_follow_the_chosen_period() {
        let day = 24 * 60 * 60;
        let now = 1_000 * day;
        assert!(is_due(None, now, UpdateFrequency::Monthly));
        assert!(!is_due(Some(now - day + 1), now, UpdateFrequency::Daily));
        assert!(is_due(Some(now - day), now, UpdateFrequency::Daily));
        assert!(!is_due(Some(now - 6 * day), now, UpdateFrequency::Weekly));
        assert!(is_due(Some(now - 7 * day), now, UpdateFrequency::Weekly));
        assert!(!is_due(Some(now - 29 * day), now, UpdateFrequency::Monthly));
        assert!(is_due(Some(now - 30 * day), now, UpdateFrequency::Monthly));
        assert!(
            is_due(Some(now + day), now, UpdateFrequency::Monthly),
            "clock moved back"
        );
    }

    /// Answers with a fixed text and counts the requests.
    struct Fake {
        answer: Option<&'static str>,
        calls: Arc<Mutex<Vec<String>>>,
    }

    impl Fetch for Fake {
        fn get(&self, url: &str) -> Result<String, UpdateError> {
            self.calls.lock().unwrap().push(url.to_owned());
            self.answer.map(str::to_owned).ok_or(UpdateError::Request {
                code: Some(6),
                message: "offline".into(),
            })
        }
    }

    fn fake(answer: Option<&'static str>) -> (Fake, Arc<Mutex<Vec<String>>>) {
        let calls = Arc::new(Mutex::new(Vec::new()));
        (
            Fake {
                answer,
                calls: Arc::clone(&calls),
            },
            calls,
        )
    }

    /// Polls until the check finishes.
    fn finish(checker: &mut Checker, schedule: Schedule) -> Result<Version, UpdateError> {
        for _ in 0..500 {
            if let Some(result) = checker.poll(schedule, || {}) {
                return result;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        panic!("the check never finished");
    }

    const ON: Schedule = Schedule {
        enabled: true,
        frequency: UpdateFrequency::Weekly,
        last: None,
    };

    #[test]
    fn nothing_is_requested_while_checking_is_off_or_not_due() {
        let (fetch, calls) = fake(Some(r#"{"tag_name":"v9.0.0"}"#));
        let mut checker = Checker::with_delay(fetch, Duration::ZERO);
        let off = Schedule { enabled: false, ..ON };
        let recent = Schedule {
            last: Some(unix_now()),
            ..ON
        };
        for _ in 0..20 {
            assert!(checker.poll(off, || {}).is_none());
            assert!(checker.poll(recent, || {}).is_none());
        }
        std::thread::sleep(Duration::from_millis(50));
        assert!(checker.poll(off, || {}).is_none());
        assert!(calls.lock().unwrap().is_empty());
    }

    #[test]
    fn the_first_check_waits_for_the_delay() {
        let (fetch, calls) = fake(Some(r#"{"tag_name":"v9.0.0"}"#));
        let mut checker = Checker::with_delay(fetch, Duration::from_secs(3600));
        assert!(checker.poll(ON, || {}).is_none());
        assert!(checker.wait() > Duration::from_secs(3500));
        assert!(calls.lock().unwrap().is_empty());
    }

    #[test]
    fn a_due_check_asks_github_once() {
        let (fetch, calls) = fake(Some(r#"{"tag_name":"v9.0.0"}"#));
        let mut checker = Checker::with_delay(fetch, Duration::ZERO);
        let latest = finish(&mut checker, ON).unwrap();
        assert_eq!(latest.to_string(), "9.0.0");
        assert_eq!(*calls.lock().unwrap(), [LATEST_URL]);
        // Right after a check, nothing more until the retry delay.
        for _ in 0..20 {
            assert!(checker.poll(ON, || {}).is_none());
        }
        assert_eq!(calls.lock().unwrap().len(), 1);
        assert!(checker.wait() > Duration::from_mins(59));
    }

    #[test]
    fn a_failed_check_is_reported_and_retried_later() {
        let (fetch, calls) = fake(None);
        let mut checker = Checker::with_delay(fetch, Duration::ZERO);
        assert!(matches!(
            finish(&mut checker, ON),
            Err(UpdateError::Request { .. })
        ));
        assert!(checker.poll(ON, || {}).is_none());
        assert_eq!(calls.lock().unwrap().len(), 1);
    }

    /// Goes online: `cargo test --lib -- --ignored real_github`.
    #[test]
    #[ignore = "network access"]
    fn real_github_answers_with_a_release() {
        let answer = Curl.get(LATEST_URL).unwrap();
        assert!(parse_latest(&answer).is_some(), "{answer}");
    }

    #[test]
    fn an_unexpected_answer_is_unreadable() {
        let (fetch, _) = fake(Some("<html>rate limited</html>"));
        let mut checker = Checker::with_delay(fetch, Duration::ZERO);
        assert!(matches!(finish(&mut checker, ON), Err(UpdateError::Unreadable)));
    }
}
