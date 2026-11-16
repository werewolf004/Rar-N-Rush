//! Windows notifications ("toasts") for sets that finish, fail or need a
//! password while the window is not in front.
//!
//! A program without an installer has no identity Windows can show in a
//! notification, so one is registered for the current user: a key under
//! `HKEY_CURRENT_USER\Software\Classes\AppUserModelId\` holding the name
//! "Rar N Rush" and an icon written next to the settings. No administrator
//! rights are needed; turning the option off removes both. Windows' own
//! settings (Do not disturb, per-app notification switches) still apply.
//!
//! Clicking a notification brings the window to the front, through the same
//! request file a second start uses (see [`crate::instance`]).

use std::fs;
use std::path::{Path, PathBuf};

/// Identity shown by Windows for our notifications.
pub const APP_ID: &str = "RarNRush.App";
/// Where application identities are registered, under `HKEY_CURRENT_USER`.
pub const IDENTITY_ROOT: &str = r"Software\Classes\AppUserModelId";
/// Icon file written in the settings folder for the notifications.
pub const ICON_FILE: &str = "rar-n-rush.ico";

/// Errors raised while registering or showing a notification.
#[derive(Debug, thiserror::Error)]
pub enum NotifyError {
    /// The identity could not be written to or removed from the registry.
    #[error("registry: {0}")]
    Registry(String),
    /// The icon file could not be written.
    #[error("cannot write {path}: {source}")]
    Icon {
        /// Icon file.
        path: PathBuf,
        /// Underlying error.
        #[source]
        source: std::io::Error,
    },
    /// Windows refused to show the notification.
    #[error("{0}")]
    Show(String),
}

/// Something that can show notifications; the window talks to this trait so
/// tests can count notifications without showing any.
pub trait Notifier: Send {
    /// Shows one notification.
    fn notify(&self, title: &str, body: &str) -> Result<(), NotifyError>;
    /// Called when the option is turned on or off.
    fn set_enabled(&self, enabled: bool) -> Result<(), NotifyError>;
}

/// Real Windows notifications.
#[derive(Debug, Clone)]
pub struct WindowsNotifier {
    root: String,
    app_id: String,
    settings_dir: PathBuf,
}

impl WindowsNotifier {
    /// Notifications for the settings folder `settings_dir` (icon file, and
    /// show requests when a notification is clicked).
    pub fn new(settings_dir: &Path) -> Self {
        Self::at(IDENTITY_ROOT, APP_ID, settings_dir)
    }

    /// Same, with another registry root and identity (tests).
    pub fn at(root: &str, app_id: &str, settings_dir: &Path) -> Self {
        Self {
            root: root.to_owned(),
            app_id: app_id.to_owned(),
            settings_dir: settings_dir.to_path_buf(),
        }
    }

    fn key(&self) -> String {
        format!(r"{}\{}", self.root, self.app_id)
    }

    fn icon_path(&self) -> PathBuf {
        self.settings_dir.join(ICON_FILE)
    }

    /// Writes the icon and the identity, if they are not already there.
    pub fn register(&self) -> Result<(), NotifyError> {
        let icon = self.icon_path();
        let bytes = crate::icon::ico(&crate::icon::ICO_SIZES);
        if fs::read(&icon).ok().as_deref() != Some(bytes.as_slice()) {
            fs::write(&icon, &bytes).map_err(|source| NotifyError::Icon {
                path: icon.clone(),
                source,
            })?;
        }
        let key = windows_registry::CURRENT_USER
            .create(self.key())
            .map_err(|e| NotifyError::Registry(e.message()))?;
        key.set_string("DisplayName", "Rar N Rush")
            .and_then(|()| key.set_string("IconUri", icon.display().to_string()))
            .map_err(|e| NotifyError::Registry(e.message()))
    }

    /// Removes the identity and the icon file.
    pub fn unregister(&self) -> Result<(), NotifyError> {
        let _ = fs::remove_file(self.icon_path());
        match windows_registry::CURRENT_USER.remove_tree(self.key()) {
            Ok(()) => Ok(()),
            // Already gone.
            Err(e) if e.code().0 == 0x8007_0002_u32.cast_signed() => Ok(()),
            Err(e) => Err(NotifyError::Registry(e.message())),
        }
    }

    /// `true` when the identity is in the registry.
    pub fn is_registered(&self) -> bool {
        windows_registry::CURRENT_USER.open(self.key()).is_ok()
    }
}

impl Notifier for WindowsNotifier {
    fn notify(&self, title: &str, body: &str) -> Result<(), NotifyError> {
        self.register()?;
        let dir = self.settings_dir.clone();
        tauri_winrt_notification::Toast::new(&self.app_id)
            .title(title)
            .text1(body)
            .on_activated(move |_| {
                let _ = crate::instance::request_show(&dir);
                Ok(())
            })
            .show()
            .map_err(|e| NotifyError::Show(e.to_string()))
    }

    fn set_enabled(&self, enabled: bool) -> Result<(), NotifyError> {
        if enabled {
            self.register()
        } else {
            self.unregister()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Registers under a throw-away root, never under the real identity.
    fn notifier(tag: &str, dir: &Path) -> (WindowsNotifier, String) {
        let root = format!(r"Software\RarNRush-test-{}-{tag}", std::process::id());
        (WindowsNotifier::at(&root, "Test.App", dir), root)
    }

    #[test]
    fn register_then_unregister_leaves_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let (n, root) = notifier("notify", dir.path());
        assert!(!n.is_registered());

        n.set_enabled(true).unwrap();
        assert!(n.is_registered());
        let key = windows_registry::CURRENT_USER.open(n.key()).unwrap();
        assert_eq!(key.get_string("DisplayName").unwrap(), "Rar N Rush");
        let icon = PathBuf::from(key.get_string("IconUri").unwrap());
        assert_eq!(icon, dir.path().join(ICON_FILE));
        assert_eq!(
            fs::read(&icon).unwrap(),
            crate::icon::ico(&crate::icon::ICO_SIZES)
        );
        n.register().unwrap(); // idempotent

        n.set_enabled(false).unwrap();
        assert!(!n.is_registered());
        assert!(!icon.exists());
        n.set_enabled(false).unwrap(); // already off: still fine
        let _ = windows_registry::CURRENT_USER.remove_tree(root);
    }
}
