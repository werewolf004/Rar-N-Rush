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

    /// Finds the settings folder for this computer (see [`Store::locate_in`]):
    /// next to the executable (portable mode) or `%APPDATA%\RarNRush`.
    pub fn locate() -> Self {
        let beside_exe = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(Path::to_path_buf));
        let appdata = std::env::var_os("APPDATA")
            .map_or_else(|| PathBuf::from("."), PathBuf::from)
            .join("RarNRush");
        Self::locate_in(beside_exe, appdata)
    }

    /// Chooses between the `portable` folder and the `fallback` folder.
    ///
    /// An **existing** settings file always wins, so the configuration is found
    /// again even if the executable was moved or its folder became read-only:
    /// 1. settings file in `portable`;
    /// 2. settings file in `fallback`;
    /// 3. no file yet: `portable` if writable, otherwise `fallback`.
    pub fn locate_in(portable: Option<PathBuf>, fallback: PathBuf) -> Self {
        if let Some(dir) = &portable
            && dir.join(SETTINGS_FILE).is_file()
        {
            return Self { dir: dir.clone() };
        }
        if fallback.join(SETTINGS_FILE).is_file() {
            return Self { dir: fallback };
        }
        match portable {
            Some(dir) if is_writable(&dir) => Self { dir },
            _ => Self { dir: fallback },
        }
    }

    /// Loads settings without ever failing, and **never loses** the user's values:
    /// - a missing file gives the defaults;
    /// - each key is read on its own: valid values are kept, invalid ones are reset
    ///   to their default and reported;
    /// - a file that is not valid TOML is kept aside under a backup name.
    ///
    /// Whenever something is reset, the original file is copied to a backup
    /// before anything can overwrite it.
    pub fn load(&self) -> Loaded {
        let path = self.dir.join(SETTINGS_FILE);
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Loaded::clean(Settings::default()),
            Err(e) => {
                return Loaded {
                    settings: Settings::default(),
                    issue: Some(LoadIssue::Unreadable {
                        path,
                        error: e.to_string(),
                    }),
                };
            }
        };
        // Notepad may save UTF-8 with a byte-order mark, which TOML rejects.
        let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
        let Ok(table) = text.parse::<toml::Table>() else {
            return Loaded {
                settings: Settings::default(),
                issue: Some(LoadIssue::Corrupt {
                    backup: self.backup(&path),
                }),
            };
        };
        let (settings, invalid_keys) = merge_valid_keys(table);
        if invalid_keys.is_empty() {
            return Loaded::clean(settings);
        }
        Loaded {
            settings,
            issue: Some(LoadIssue::InvalidValues {
                keys: invalid_keys,
                backup: self.backup(&path),
            }),
        }
    }

    /// Copies `path` to `rar-n-rush.toml.<date-time>.bak`. `None` if the copy failed.
    fn backup(&self, path: &Path) -> Option<PathBuf> {
        let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
        let backup = self.dir.join(format!("{SETTINGS_FILE}.{stamp}.bak"));
        fs::copy(path, &backup).ok().map(|_| backup)
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

/// Result of [`Store::load`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Loaded {
    /// Settings to use.
    pub settings: Settings,
    /// Problem found in the settings file, to show to the user.
    pub issue: Option<LoadIssue>,
}

impl Loaded {
    fn clean(settings: Settings) -> Self {
        Self {
            settings,
            issue: None,
        }
    }
}

/// Problem found while loading the settings file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadIssue {
    /// The file could not be read (permissions, lock…). Defaults are used.
    Unreadable {
        /// Settings file.
        path: PathBuf,
        /// System error message.
        error: String,
    },
    /// The file is not valid TOML. Defaults are used; the file was copied to `backup`.
    Corrupt {
        /// Copy of the original file, if it could be made.
        backup: Option<PathBuf>,
    },
    /// Some values were invalid and reset to their default; the others were kept.
    InvalidValues {
        /// Keys that were reset.
        keys: Vec<String>,
        /// Copy of the original file, if it could be made.
        backup: Option<PathBuf>,
    },
}

/// Starts from the defaults and applies every key of `table` that deserialises
/// correctly on its own. Returns the settings and the names of rejected keys.
/// Unknown keys (e.g. from a newer version) are ignored, not reported.
fn merge_valid_keys(table: toml::Table) -> (Settings, Vec<String>) {
    let Ok(toml::Value::Table(mut merged)) = toml::Value::try_from(Settings::default()) else {
        return (Settings::default(), Vec::new());
    };
    let mut invalid = Vec::new();
    for (key, value) in table {
        let mut candidate = merged.clone();
        candidate.insert(key.clone(), value);
        if toml::Value::Table(candidate.clone())
            .try_into::<Settings>()
            .is_ok()
        {
            merged = candidate;
        } else {
            invalid.push(key);
        }
    }
    let settings = toml::Value::Table(merged)
        .try_into::<Settings>()
        .unwrap_or_default()
        .validated();
    (settings, invalid)
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

    fn issue_backup(issue: Option<&LoadIssue>) -> PathBuf {
        match issue {
            Some(
                LoadIssue::Corrupt { backup: Some(b) } | LoadIssue::InvalidValues { backup: Some(b), .. },
            ) => b.clone(),
            other => panic!("expected a backup, got {other:?}"),
        }
    }

    #[test]
    fn load_keeps_valid_values_and_resets_only_invalid_ones() {
        let dir = tempfile::tempdir().unwrap();
        let original = "watch_dir = 'D:\\dl'\nscan_interval_secs = 'often'\ntheme = 'purple'\n\
                        passwords = ['a', 'b']\nfuture_option = 42\n";
        fs::write(dir.path().join(SETTINGS_FILE), original).unwrap();
        let loaded = Store::at(dir.path()).load();
        assert_eq!(loaded.settings.watch_dir, "D:\\dl");
        assert_eq!(loaded.settings.passwords, vec!["a".to_owned(), "b".to_owned()]);
        assert_eq!(
            loaded.settings.scan_interval_secs,
            Settings::default().scan_interval_secs
        );
        assert_eq!(loaded.settings.theme, ThemePref::System);
        match &loaded.issue {
            Some(LoadIssue::InvalidValues { keys, .. }) => {
                let mut keys = keys.clone();
                keys.sort();
                assert_eq!(keys, vec!["scan_interval_secs".to_owned(), "theme".to_owned()]);
            }
            other => panic!("unexpected {other:?}"),
        }
        // The original file is copied before anything can overwrite it.
        let backup = issue_backup(loaded.issue.as_ref());
        assert_eq!(fs::read_to_string(backup).unwrap(), original);
        assert_eq!(
            fs::read_to_string(dir.path().join(SETTINGS_FILE)).unwrap(),
            original
        );
    }

    #[test]
    fn corrupt_file_is_backed_up_not_lost() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(SETTINGS_FILE), "watch_dir = 'unterminated").unwrap();
        let loaded = Store::at(dir.path()).load();
        assert_eq!(loaded.settings, Settings::default());
        let backup = issue_backup(loaded.issue.as_ref());
        assert_eq!(fs::read_to_string(backup).unwrap(), "watch_dir = 'unterminated");
    }

    #[test]
    fn utf8_bom_from_notepad_is_accepted() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(SETTINGS_FILE), "\u{feff}watch_dir = 'X'\n").unwrap();
        let loaded = Store::at(dir.path()).load();
        assert_eq!(loaded.issue, None);
        assert_eq!(loaded.settings.watch_dir, "X");
    }

    #[test]
    fn unknown_keys_are_not_an_issue() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join(SETTINGS_FILE),
            "watch_dir = 'X'\nnew_in_v9 = true\n",
        )
        .unwrap();
        assert_eq!(Store::at(dir.path()).load().issue, None);
    }

    #[test]
    fn every_setting_survives_save_and_load() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::at(dir.path());
        let s = Settings {
            watch_dir: "D:\\in".into(),
            extract_dir: "E:\\out".into(),
            create_subfolder: false,
            recursive: false,
            par2_path: "C:\\tools\\par2.exe".into(),
            delete_after_success: true,
            extract_nested: false,
            overwrite_existing: true,
            scan_interval_secs: 42,
            stable_secs: 7,
            passwords: vec!["p1".into(), "p é 2".into()],
            autostart: false,
            language: LanguagePref::French,
            theme: ThemePref::Dark,
        };
        for _ in 0..3 {
            store.save_settings(&s).unwrap(); // repeated saves replace the file in place
        }
        assert_eq!(store.load(), Loaded::clean(s));
    }

    #[test]
    fn locate_finds_the_existing_configuration() {
        let root = tempfile::tempdir().unwrap();
        let (portable, fallback) = (root.path().join("app"), root.path().join("appdata"));
        fs::create_dir_all(&portable).unwrap();
        fs::create_dir_all(&fallback).unwrap();

        // Fresh install: portable folder when writable, fallback otherwise.
        assert_eq!(
            Store::locate_in(Some(portable.clone()), fallback.clone()).dir(),
            portable
        );
        assert_eq!(Store::locate_in(None, fallback.clone()).dir(), fallback);

        // A file in the fallback folder is found even though the portable folder is writable.
        fs::write(fallback.join(SETTINGS_FILE), "").unwrap();
        assert_eq!(
            Store::locate_in(Some(portable.clone()), fallback.clone()).dir(),
            fallback
        );

        // A file next to the executable wins over the fallback.
        fs::write(portable.join(SETTINGS_FILE), "").unwrap();
        assert_eq!(Store::locate_in(Some(portable.clone()), fallback).dir(), portable);
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
