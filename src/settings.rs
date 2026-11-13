//! User settings and persistent state (list of already processed sets).
//!
//! Files are stored next to the executable (portable mode). When that folder is
//! not writable (for example under `Program Files`), `%APPDATA%\RarNRush` is used.
//! Every write is atomic: data goes to a temporary file which then replaces the
//! target, so a crash never leaves a truncated settings file behind.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Settings file name.
pub const SETTINGS_FILE: &str = "rar-n-rush.toml";
/// File listing the keys of sets that were already processed.
pub const PROCESSED_FILE: &str = "rar-n-rush-processed.txt";

/// Allowed range for [`Settings::scan_interval_secs`].
pub const SCAN_INTERVAL_RANGE: std::ops::RangeInclusive<u64> = 2..=3600;
/// Allowed range for [`Settings::stable_secs`].
pub const STABLE_RANGE: std::ops::RangeInclusive<u64> = 0..=3600;

/// Errors raised while reading or writing settings and state.
#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    /// A file could not be read or written.
    #[error("I/O error on {path}: {source}")]
    Io {
        /// File concerned.
        path: PathBuf,
        /// Underlying error.
        #[source]
        source: io::Error,
    },
    /// The settings file is not valid TOML or has wrong value types.
    #[error("invalid settings file {path}: {source}")]
    Parse {
        /// File concerned.
        path: PathBuf,
        /// Underlying error.
        #[source]
        source: toml::de::Error,
    },
    /// The settings could not be serialised.
    #[error("cannot serialise settings: {0}")]
    Serialize(#[from] toml::ser::Error),
}

/// User settings. Every field has a default, so older files keep loading.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
#[allow(clippy::struct_excessive_bools)] // independent user toggles
pub struct Settings {
    /// Folder being watched.
    pub watch_dir: String,
    /// Extraction folder. Empty means "next to the source files".
    pub extract_dir: String,
    /// Extract each set into a sub-folder named after the set.
    pub create_subfolder: bool,
    /// Also scan sub-folders of the watched folder.
    pub recursive: bool,
    /// Path to `par2.exe`. Empty means automatic lookup.
    pub par2_path: String,
    /// Send PAR2/SFV/RAR source files to the recycle bin after a successful run.
    pub delete_after_success: bool,
    /// Also extract RAR archives found inside an extracted archive.
    pub extract_nested: bool,
    /// Overwrite files that already exist in the destination.
    /// When `false`, a free name such as `file (1).ext` is used instead.
    pub overwrite_existing: bool,
    /// Seconds between two scans of the watched folder.
    pub scan_interval_secs: u64,
    /// Seconds a set must stay unchanged before it is processed.
    pub stable_secs: u64,
    /// Passwords tried automatically on protected archives.
    pub passwords: Vec<String>,
    /// Start monitoring when the application starts.
    pub autostart: bool,
    /// Interface language.
    pub language: LanguagePref,
    /// Colour theme.
    pub theme: ThemePref,
}

/// Interface language preference.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LanguagePref {
    /// Follow the Windows display language.
    #[default]
    System,
    /// English.
    English,
    /// French.
    French,
}

/// Colour theme preference.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ThemePref {
    /// Follow the Windows light/dark setting, live.
    #[default]
    System,
    /// Always light.
    Light,
    /// Always dark.
    Dark,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            watch_dir: String::new(),
            extract_dir: String::new(),
            create_subfolder: true,
            recursive: true,
            par2_path: String::new(),
            delete_after_success: false,
            extract_nested: true,
            overwrite_existing: false,
            scan_interval_secs: 15,
            stable_secs: 10,
            passwords: Vec::new(),
            autostart: true,
            language: LanguagePref::System,
            theme: ThemePref::System,
        }
    }
}

impl Settings {
    /// Returns a copy with numbers clamped to their allowed ranges and text trimmed.
    #[must_use]
    pub fn validated(mut self) -> Self {
        self.scan_interval_secs = self
            .scan_interval_secs
            .clamp(*SCAN_INTERVAL_RANGE.start(), *SCAN_INTERVAL_RANGE.end());
        self.stable_secs = self.stable_secs.clamp(*STABLE_RANGE.start(), *STABLE_RANGE.end());
        self.watch_dir = self.watch_dir.trim().to_owned();
        self.extract_dir = self.extract_dir.trim().to_owned();
        self.par2_path = self.par2_path.trim().to_owned();
        self.passwords.retain(|p| !p.is_empty());
        self.passwords.dedup();
        self
    }

    /// Watched folder, if one is configured.
    pub fn watch_path(&self) -> Option<PathBuf> {
        non_empty_path(&self.watch_dir)
    }

    /// Extraction folder, if one is configured.
    pub fn extract_path(&self) -> Option<PathBuf> {
        non_empty_path(&self.extract_dir)
    }

    /// Location of `par2.exe`: explicit setting, then next to our executable, then `PATH`.
    pub fn resolve_par2(&self) -> Option<PathBuf> {
        if let Some(p) = non_empty_path(&self.par2_path) {
            return p.is_file().then_some(p);
        }
        let beside_exe = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(|d| d.join("par2.exe")))
            .filter(|p| p.is_file());
        if beside_exe.is_some() {
            return beside_exe;
        }
        let path_var = std::env::var_os("PATH")?;
        std::env::split_paths(&path_var)
            .map(|dir| dir.join("par2.exe"))
            .find(|p| p.is_file())
    }
}

fn non_empty_path(s: &str) -> Option<PathBuf> {
    let t = s.trim();
    (!t.is_empty()).then(|| PathBuf::from(t))
}

/// Folder holding the settings and state files.
#[derive(Debug, Clone)]
pub struct Store {
    dir: PathBuf,
}

impl Store {
    /// Store in an explicit folder (used by tests and portable setups).
    pub fn at(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    /// Portable store next to the executable, or `%APPDATA%\RarNRush` as a fallback.
    pub fn portable() -> Self {
        if let Some(dir) = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(Path::to_path_buf))
            && is_writable(&dir)
        {
            return Self { dir };
        }
        let base = std::env::var_os("APPDATA").map_or_else(|| PathBuf::from("."), PathBuf::from);
        Self {
            dir: base.join("RarNRush"),
        }
    }

    /// Folder used by this store.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Loads settings. A missing file yields the defaults.
    pub fn load_settings(&self) -> Result<Settings, SettingsError> {
        let path = self.dir.join(SETTINGS_FILE);
        match fs::read_to_string(&path) {
            Ok(text) => toml::from_str::<Settings>(&text)
                .map(Settings::validated)
                .map_err(|source| SettingsError::Parse { path, source }),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Settings::default()),
            Err(source) => Err(SettingsError::Io { path, source }),
        }
    }

    /// Saves settings atomically.
    pub fn save_settings(&self, settings: &Settings) -> Result<(), SettingsError> {
        let text = toml::to_string_pretty(settings)?;
        self.write_atomic(SETTINGS_FILE, text.as_bytes())
    }

    /// Loads the keys of already processed sets. A missing file yields an empty set.
    pub fn load_processed(&self) -> Result<HashSet<String>, SettingsError> {
        let path = self.dir.join(PROCESSED_FILE);
        match fs::read_to_string(&path) {
            Ok(text) => Ok(text
                .lines()
                .filter(|l| !l.is_empty())
                .map(str::to_owned)
                .collect()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(HashSet::new()),
            Err(source) => Err(SettingsError::Io { path, source }),
        }
    }

    /// Records a processed set key (append-only).
    pub fn append_processed(&self, key: &str) -> Result<(), SettingsError> {
        let path = self.dir.join(PROCESSED_FILE);
        let io_err = |source| SettingsError::Io {
            path: path.clone(),
            source,
        };
        fs::create_dir_all(&self.dir).map_err(io_err)?;
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(io_err)?;
        // Keys never contain line breaks: they come from file names, which cannot.
        writeln!(file, "{}", key.replace(['\r', '\n'], " ")).map_err(io_err)
    }

    fn write_atomic(&self, name: &str, bytes: &[u8]) -> Result<(), SettingsError> {
        let path = self.dir.join(name);
        let tmp = self.dir.join(format!("{name}.tmp"));
        let io_err = |p: &Path| {
            let p = p.to_path_buf();
            move |source| SettingsError::Io { path: p, source }
        };
        fs::create_dir_all(&self.dir).map_err(io_err(&self.dir))?;
        {
            let mut f = fs::File::create(&tmp).map_err(io_err(&tmp))?;
            f.write_all(bytes).map_err(io_err(&tmp))?;
            f.sync_all().map_err(io_err(&tmp))?;
        }
        fs::rename(&tmp, &path).map_err(io_err(&path))
    }
}

fn is_writable(dir: &Path) -> bool {
    let probe = dir.join(".rnr-write-test");
    let ok = fs::write(&probe, b"").is_ok();
    let _ = fs::remove_file(&probe);
    ok
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_valid() {
        let s = Settings::default();
        assert_eq!(s.clone().validated(), s);
    }

    #[test]
    fn validation_clamps_and_trims() {
        let s = Settings {
            scan_interval_secs: 0,
            stable_secs: 999_999,
            watch_dir: "  C:\\dl  ".into(),
            passwords: vec![String::new(), "a".into(), "a".into()],
            ..Settings::default()
        }
        .validated();
        assert_eq!(s.scan_interval_secs, 2);
        assert_eq!(s.stable_secs, 3600);
        assert_eq!(s.watch_dir, "C:\\dl");
        assert_eq!(s.passwords, vec!["a".to_owned()]);
    }

    #[test]
    fn round_trip_and_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::at(dir.path());
        assert_eq!(store.load_settings().unwrap(), Settings::default());
        let s = Settings {
            watch_dir: "D:\\in".into(),
            passwords: vec!["pw".into()],
            ..Settings::default()
        };
        store.save_settings(&s).unwrap();
        assert_eq!(store.load_settings().unwrap(), s);
        assert!(!dir.path().join("rar-n-rush.toml.tmp").exists());
    }

    #[test]
    fn partial_file_uses_defaults_for_missing_fields() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(SETTINGS_FILE), "watch_dir = 'X'\n").unwrap();
        let s = Store::at(dir.path()).load_settings().unwrap();
        assert_eq!(s.watch_dir, "X");
        assert_eq!(s.scan_interval_secs, Settings::default().scan_interval_secs);
    }

    #[test]
    fn enums_use_lowercase_names() {
        let s = Settings {
            language: LanguagePref::French,
            theme: ThemePref::Dark,
            ..Settings::default()
        };
        let text = toml::to_string(&s).unwrap();
        assert!(text.contains("language = \"french\""));
        assert!(text.contains("theme = \"dark\""));
    }

    #[test]
    fn invalid_file_is_a_typed_error() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(SETTINGS_FILE), "scan_interval_secs = 'abc'").unwrap();
        assert!(matches!(
            Store::at(dir.path()).load_settings(),
            Err(SettingsError::Parse { .. })
        ));
    }

    #[test]
    fn processed_keys_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::at(dir.path());
        assert!(store.load_processed().unwrap().is_empty());
        store.append_processed("a|b").unwrap();
        store.append_processed("c\nd").unwrap();
        let keys = store.load_processed().unwrap();
        assert!(keys.contains("a|b"));
        assert!(keys.contains("c d"));
        assert_eq!(keys.len(), 2);
    }
}
