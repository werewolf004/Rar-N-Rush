//! Clean-up run by the Windows Installer package when the program is removed.
//!
//! The package starts `rar-n-rush.exe` [`UNINSTALL_ARG`] just before deleting
//! its files (never during an upgrade). The program then removes what it wrote
//! in the registry itself, which the package does not know about:
//! - its autostart entry, only if it starts this very copy;
//! - its notification identity and icon file.
//!
//! Settings, the list of processed sets and the passwords in the Credential
//! Manager are **kept**: reinstalling finds everything again.

use crate::autostart::{Autostart, AutostartError, Change};
use crate::notify::{NotifyError, WindowsNotifier};

/// Command-line argument used by the package at uninstall time.
pub const UNINSTALL_ARG: &str = "--uninstall";

/// `true` when the program was started by the package to clean up.
pub fn requested(mut args: impl Iterator<Item = std::ffi::OsString>) -> bool {
    args.any(|a| a == UNINSTALL_ARG)
}

/// What could not be cleaned up. The uninstall goes on regardless.
#[derive(Debug, thiserror::Error)]
pub enum UninstallError {
    /// The autostart entry could not be read or removed.
    #[error(transparent)]
    Autostart(#[from] AutostartError),
    /// The notification identity could not be removed.
    #[error(transparent)]
    Notify(#[from] NotifyError),
}

/// Removes the autostart entry (when it is ours) and the notification identity.
/// Both are attempted even if the first fails; the first error is returned.
pub fn clean_up(autostart: &Autostart, notifier: &WindowsNotifier) -> Result<Change, UninstallError> {
    let autostart = autostart.remove_if_ours();
    let notify = notifier.unregister();
    let change = autostart?;
    notify?;
    Ok(change)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    struct TestKey(String);

    impl Drop for TestKey {
        fn drop(&mut self) {
            let _ = windows_registry::CURRENT_USER.remove_tree(&self.0);
        }
    }

    #[test]
    fn argument_is_recognised() {
        assert!(requested(["x.exe", "--uninstall"].iter().map(Into::into)));
        assert!(!requested(["x.exe", "--autostart"].iter().map(Into::into)));
    }

    #[test]
    fn removes_autostart_and_notification_identity() {
        let key = TestKey(format!(r"Software\RarNRush-test-{}-clean-up", std::process::id()));
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("rar-n-rush.exe");
        let autostart = Autostart::at(&format!(r"{}\Run", key.0), "Rar N Rush", &exe);
        let notifier = WindowsNotifier::at(&format!(r"{}\Ids", key.0), "RarNRush.Test", dir.path());

        autostart.apply(true).unwrap();
        notifier.register().unwrap();
        assert!(notifier.is_registered());

        assert_eq!(clean_up(&autostart, &notifier).unwrap(), Change::Removed);
        assert_eq!(autostart.registered().unwrap(), None);
        assert!(!notifier.is_registered());
        assert!(!dir.path().join(crate::notify::ICON_FILE).exists());

        // Running it again (or with nothing registered) is harmless.
        assert_eq!(clean_up(&autostart, &notifier).unwrap(), Change::None);
        assert!(!Path::new(&exe).exists(), "nothing else was created");
    }
}
