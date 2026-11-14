//! Start with Windows: a value under the current user's `Run` registry key.
//!
//! No administrator rights are needed and nothing else is installed: turning
//! the option off removes the value. The command line carries
//! [`AUTOSTART_ARG`] so the program starts minimised (or hidden in the
//! notification area when that option is on).
//!
//! The value is checked again at every start, so a program that was moved
//! still starts from its new place, and an entry left by a crash or by an
//! older copy cannot contradict the setting.

use std::path::Path;

/// Registry key read by Windows at sign-in, under `HKEY_CURRENT_USER`.
pub const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
/// Name of the value written under [`RUN_KEY`].
pub const VALUE_NAME: &str = "Rar N Rush";
/// Command-line argument added to the autostart command.
pub const AUTOSTART_ARG: &str = "--autostart";

/// `HRESULT_FROM_WIN32(ERROR_FILE_NOT_FOUND)`: the key or value does not exist.
const NOT_FOUND: i32 = 0x8007_0002_u32.cast_signed();

/// Errors raised while reading or writing the registry.
#[derive(Debug, thiserror::Error)]
#[error("registry ({key}\\{name}): {message}")]
pub struct AutostartError {
    /// Key path under `HKEY_CURRENT_USER`.
    pub key: String,
    /// Value name.
    pub name: String,
    /// Message from Windows.
    pub message: String,
}

/// What [`Autostart::apply`] changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    /// The registry already matched the setting.
    None,
    /// The value was written (new, or pointing to another place before).
    Written,
    /// The value was removed.
    Removed,
}

/// The autostart entry of one executable.
#[derive(Debug, Clone)]
pub struct Autostart {
    key: String,
    name: String,
    command: String,
}

impl Autostart {
    /// The real entry: `HKEY_CURRENT_USER\` [`RUN_KEY`] `\` [`VALUE_NAME`].
    pub fn current_user(exe: &Path) -> Self {
        Self::at(RUN_KEY, VALUE_NAME, exe)
    }

    /// Entry at another key and name (tests use a throw-away key).
    pub fn at(key: &str, name: &str, exe: &Path) -> Self {
        Self {
            key: key.to_owned(),
            name: name.to_owned(),
            command: command_line(exe),
        }
    }

    /// The command line written to the registry.
    pub fn command(&self) -> &str {
        &self.command
    }

    fn error(&self, message: String) -> AutostartError {
        AutostartError {
            key: self.key.clone(),
            name: self.name.clone(),
            message,
        }
    }

    /// The command currently registered, if any.
    pub fn registered(&self) -> Result<Option<String>, AutostartError> {
        let key = match windows_registry::CURRENT_USER.open(&self.key) {
            Ok(key) => key,
            Err(e) if e.code().0 == NOT_FOUND => return Ok(None),
            Err(e) => return Err(self.error(e.message())),
        };
        match key.get_string(&self.name) {
            Ok(value) => Ok(Some(value)),
            Err(e) if e.code().0 == NOT_FOUND => Ok(None),
            Err(e) => Err(self.error(e.message())),
        }
    }

    /// Makes the registry match `enabled`, touching it only when needed.
    pub fn apply(&self, enabled: bool) -> Result<Change, AutostartError> {
        let current = self.registered()?;
        match (enabled, current) {
            (true, Some(cmd)) if cmd == self.command => Ok(Change::None),
            (true, _) => {
                windows_registry::CURRENT_USER
                    .create(&self.key)
                    .and_then(|key| key.set_string(&self.name, &self.command))
                    .map_err(|e| self.error(e.message()))?;
                Ok(Change::Written)
            }
            (false, None) => Ok(Change::None),
            (false, Some(_)) => {
                windows_registry::CURRENT_USER
                    .create(&self.key)
                    .and_then(|key| key.remove_value(&self.name))
                    .map_err(|e| self.error(e.message()))?;
                Ok(Change::Removed)
            }
        }
    }
}

/// `"C:\path\rar-n-rush.exe" --autostart`: quoted, since the path may contain spaces.
fn command_line(exe: &Path) -> String {
    format!("\"{}\" {AUTOSTART_ARG}", exe.display())
}

/// `true` when the program was started by Windows at sign-in.
pub fn started_by_windows(mut args: impl Iterator<Item = std::ffi::OsString>) -> bool {
    args.any(|a| a == AUTOSTART_ARG)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// A throw-away key under `HKEY_CURRENT_USER\Software`, removed on drop.
    struct TestKey(String);

    impl TestKey {
        fn new(tag: &str) -> Self {
            Self(format!(r"Software\RarNRush-test-{}-{tag}", std::process::id()))
        }
    }

    impl Drop for TestKey {
        fn drop(&mut self) {
            let _ = windows_registry::CURRENT_USER.remove_tree(&self.0);
        }
    }

    #[test]
    fn command_is_quoted_and_marked() {
        let a = Autostart::current_user(Path::new(r"C:\Program Files\Rar N Rush\rar-n-rush.exe"));
        assert_eq!(
            a.command(),
            r#""C:\Program Files\Rar N Rush\rar-n-rush.exe" --autostart"#
        );
    }

    #[test]
    fn autostart_argument_is_recognised() {
        assert!(started_by_windows(
            ["x.exe", "--autostart"].iter().map(Into::into)
        ));
        assert!(!started_by_windows(["x.exe"].iter().map(Into::into)));
    }

    #[test]
    fn enable_disable_round_trip_in_the_registry() {
        let key = TestKey::new("round-trip");
        let a = Autostart::at(&key.0, "Rar N Rush", Path::new(r"C:\Apps\rar-n-rush.exe"));
        assert_eq!(a.registered().unwrap(), None, "missing key reads as off");
        assert_eq!(a.apply(false).unwrap(), Change::None);

        assert_eq!(a.apply(true).unwrap(), Change::Written);
        assert_eq!(a.registered().unwrap().as_deref(), Some(a.command()));
        assert_eq!(a.apply(true).unwrap(), Change::None, "nothing rewritten");

        assert_eq!(a.apply(false).unwrap(), Change::Removed);
        assert_eq!(a.registered().unwrap(), None);
    }

    #[test]
    fn moved_program_updates_its_entry() {
        let key = TestKey::new("moved");
        let old = Autostart::at(&key.0, "Rar N Rush", Path::new(r"C:\Old\rar-n-rush.exe"));
        old.apply(true).unwrap();
        let new_place = PathBuf::from(r"D:\New place\rar-n-rush.exe");
        let new = Autostart::at(&key.0, "Rar N Rush", &new_place);
        assert_eq!(new.apply(true).unwrap(), Change::Written);
        assert_eq!(new.registered().unwrap().as_deref(), Some(new.command()));
    }

    #[test]
    fn other_values_of_the_key_are_left_alone() {
        let key = TestKey::new("others");
        windows_registry::CURRENT_USER
            .create(&key.0)
            .unwrap()
            .set_string("Other app", "other.exe")
            .unwrap();
        let a = Autostart::at(&key.0, "Rar N Rush", Path::new(r"C:\a.exe"));
        a.apply(true).unwrap();
        a.apply(false).unwrap();
        let other = windows_registry::CURRENT_USER
            .open(&key.0)
            .unwrap()
            .get_string("Other app")
            .unwrap();
        assert_eq!(other, "other.exe");
    }
}
