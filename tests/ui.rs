//! Headless UI tests (no window is ever shown) and documentation screenshots.
//!
//! The tests drive the real [`App`] through `egui_kittest` and its accessibility
//! tree. Images are rendered with wgpu on Windows' software rasterizer (WARP),
//! so no GPU is needed, in CI or locally:
//!
//! - `snapshots` compares the main screens pixel by pixel with
//!   `tests/snapshots/*.png` (`UPDATE_SNAPSHOTS=1` rewrites them);
//! - `screenshots` writes `docs/screenshots/*.png` only when
//!   `UPDATE_SCREENSHOTS=1` is set.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use rar_n_rush::engine::{Engine, Status};
use rar_n_rush::settings::{LanguagePref, LoadIssue, Settings, Store, ThemePref};
use rar_n_rush::ui::App;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const SIZE: egui::Vec2 = egui::vec2(1100.0, 640.0);

/// Folder with a demo download set: done, password, failed and waiting entries.
struct Demo {
    temp: Option<tempfile::TempDir>,
    root: PathBuf,
}

impl Demo {
    /// `neutral_path` puts the demo under `C:\Users\Public` so screenshots never
    /// show a personal user name.
    fn new(neutral_path: bool) -> Self {
        Self::in_folder(neutral_path.then_some(r"C:\Users\Public\RarNRush-demo"))
    }

    /// Demo in a fixed folder (`Some`) or in a temporary one (`None`).
    fn in_folder(fixed: Option<&str>) -> Self {
        let (guard, root) = if let Some(fixed) = fixed {
            let root = PathBuf::from(fixed);
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(&root).unwrap();
            (None, root)
        } else {
            let dir = tempfile::tempdir().unwrap();
            let root = dir.path().to_path_buf();
            (Some(dir), root)
        };
        let watch = root.join("Downloads");
        fs::create_dir_all(&watch).unwrap();
        for (from, to) in [
            ("multi.part1.rar", "Holidays.2026.part1.rar"),
            ("multi.part2.rar", "Holidays.2026.part2.rar"),
            ("multi.part3.rar", "Holidays.2026.part3.rar"),
            ("password.rar", "Private.Documents.rar"),
            ("plain.rar", "Photos.rar"),
            ("corrupt.rar", "Damaged.Archive.rar"),
        ] {
            fs::copy(common::fixtures().join(from), watch.join(to)).unwrap();
        }
        common::write_sfv(
            &watch,
            "Holidays.2026.sfv",
            &[
                "Holidays.2026.part1.rar",
                "Holidays.2026.part2.rar",
                "Holidays.2026.part3.rar",
            ],
        );
        Self { temp: guard, root }
    }

    fn settings(&self, language: LanguagePref, theme: ThemePref) -> Settings {
        Settings {
            watch_dir: self.root.join("Downloads").display().to_string(),
            extract_dir: self.root.join("Extracted").display().to_string(),
            stable_secs: 0,
            scan_interval_secs: 30,
            language,
            theme,
            ..Settings::default()
        }
    }

    fn state_dir(&self, tag: &str) -> PathBuf {
        let dir = self.root.join(format!("state-{tag}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }
}

impl Drop for Demo {
    fn drop(&mut self) {
        if self.temp.is_none() {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}

fn harness(settings: Settings, store_dir: &Path, gpu: bool) -> Harness<'static, App> {
    harness_with_issue(settings, store_dir, gpu, None)
}

fn harness_with_issue(
    settings: Settings,
    store_dir: &Path,
    gpu: bool,
    issue: Option<LoadIssue>,
) -> Harness<'static, App> {
    let theme = match settings.theme {
        ThemePref::Dark => egui::Theme::Dark,
        _ => egui::Theme::Light,
    };
    let mut store = Store::at(store_dir);
    let mut builder = Harness::builder()
        .with_size(SIZE)
        .with_theme(theme)
        .with_render_cursor(false);
    if gpu {
        builder = builder.wgpu();
        // Images show what users see: passwords in a vault, not "stored in plain text".
        store = store.with_vault(std::sync::Arc::new(rar_n_rush::secrets::ChunkedVault::in_memory()));
    }
    builder.build_eframe(move |cc| {
        let ctx = cc.egui_ctx.clone();
        let engine = Engine::start(settings, store.clone(), move || ctx.request_repaint()).unwrap();
        App::new(&cc.egui_ctx, engine, store, issue)
    })
}

/// Steps the UI until the engine reached a stable picture of the demo folder.
fn settle(h: &mut Harness<'static, App>, expected_jobs: usize) {
    let start = Instant::now();
    loop {
        h.step();
        let settled = h.state().engine().with_state(|s| {
            s.jobs
                .iter()
                .filter(|j| matches!(j.status, Status::Done | Status::Failed | Status::NeedPassword))
                .count()
        });
        if settled >= expected_jobs {
            break;
        }
        assert!(start.elapsed() < Duration::from_secs(60), "engine did not settle");
        std::thread::sleep(Duration::from_millis(50));
    }
    for _ in 0..3 {
        h.step();
    }
}

/// Runs a few frames: egui windows stay invisible on their first frame.
fn frames(h: &mut Harness<'static, App>, n: usize) {
    for _ in 0..n {
        h.step();
    }
}

fn save(h: &mut Harness<'static, App>, name: &str) {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("docs")
        .join("screenshots");
    fs::create_dir_all(&dir).unwrap();
    h.render().unwrap().save(dir.join(format!("{name}.png"))).unwrap();
}

#[test]
fn onboarding_screen_offers_to_choose_a_folder() {
    let state = tempfile::tempdir().unwrap();
    let settings = Settings {
        language: LanguagePref::English,
        ..Settings::default()
    };
    let mut h = harness(settings, state.path(), false);
    h.step();
    h.get_by_label("Choose a folder…");
    h.get_by_label("⚙  Settings");
    // Start is disabled while no folder is configured, Pause is absent.
    assert!(h.query_by_label("⏸  Pause").is_none());
}

#[test]
fn main_screen_lists_sets_with_their_state_in_french() {
    let demo = Demo::new(false);
    let mut h = harness(
        demo.settings(LanguagePref::French, ThemePref::Light),
        &demo.state_dir("fr"),
        false,
    );
    settle(&mut h, 4);
    for label in ["Holidays.2026", "Private.Documents", "Photos", "Damaged.Archive"] {
        h.get_by_label(label);
    }
    assert_eq!(h.get_all_by_label("Terminé").count(), 2);
    h.get_by_label("Mot de passe requis");
    h.get_by_label("Erreur");
    h.get_by_label("⏸  Pause");
    h.get_by_label("Journal");
}

#[test]
fn settings_window_opens_and_closes() {
    let demo = Demo::new(false);
    let mut h = harness(
        demo.settings(LanguagePref::English, ThemePref::Dark),
        &demo.state_dir("en"),
        false,
    );
    h.step();
    h.get_by_label("⚙  Settings").click();
    frames(&mut h, 3);
    // In the main window and in the settings window.
    assert_eq!(h.get_all_by_label("Extraction folder").count(), 2);
    h.get_by_label("Follow Windows");
    h.get_by_label("Processing").click();
    frames(&mut h, 2);
    for label in [
        "Overwrite existing files (otherwise they are kept as they are)",
        "Move the other files of the set (.nfo, samples…) to the destination",
        "Move folders left empty to the Recycle Bin",
        "Passwords to try (one per line)",
    ] {
        h.get_by_label(label);
    }
    h.get_by_label("Window").click();
    frames(&mut h, 2);
    for label in [
        "Keep the set being processed in view",
        "Minimise to the notification area",
        "Close to the notification area",
    ] {
        h.get_by_label(label);
    }
    h.get_by_label("Cancel").click();
    frames(&mut h, 3);
    assert!(
        h.query_by_label("Destination").is_none(),
        "settings window must close"
    );
}

#[test]
fn saving_settings_switches_language_live() {
    let demo = Demo::new(false);
    let state = demo.state_dir("lang");
    let mut h = harness(
        demo.settings(LanguagePref::English, ThemePref::Light),
        &state,
        false,
    );
    h.step();
    h.get_by_label("⚙  Settings").click();
    frames(&mut h, 3);
    h.get_by_label("Language").click();
    frames(&mut h, 3);
    h.get_by_label("Français").click();
    frames(&mut h, 3);
    h.get_by_label("Save").click();
    frames(&mut h, 3);
    h.get_by_label("⚙  Réglages");
    let saved = Store::at(&state).load_settings().unwrap();
    assert_eq!(saved.language, LanguagePref::French);
}

#[test]
fn settings_problem_is_shown_until_dismissed() {
    let state = tempfile::tempdir().unwrap();
    let issue = LoadIssue::InvalidValues {
        keys: vec!["theme".into()],
        backup: None,
    };
    let settings = Settings {
        language: LanguagePref::English,
        ..Settings::default()
    };
    let mut h = harness_with_issue(settings, state.path(), false, Some(issue));
    frames(&mut h, 2);
    h.get_by_label_contains("Invalid values were reset to their default: theme");
    h.get_by_label("OK").click();
    frames(&mut h, 2);
    assert!(h.query_by_label_contains("Invalid values").is_none());
}

/// The editable watched-folder field of the main window.
fn watch_field<'a>(h: &'a Harness<'static, App>) -> egui_kittest::Node<'a> {
    h.get_by_role_and_label(egui::accesskit::Role::TextInput, "Watched folder")
}

/// Types `text` into the watched-folder field and presses Enter.
fn type_watch_dir(h: &mut Harness<'static, App>, text: &str) {
    watch_field(h).focus();
    frames(h, 2);
    watch_field(h).set_text_value_accesskit(text);
    frames(h, 2);
    h.key_press(egui::Key::Enter);
    frames(h, 3);
}

#[test]
fn watched_folder_is_changed_from_the_main_window() {
    let demo = Demo::new(false);
    let state = demo.state_dir("watch");
    let other = demo.root.join("Other");
    fs::create_dir_all(&other).unwrap();
    let settings = demo.settings(LanguagePref::English, ThemePref::Light);
    let first = settings.watch_dir.clone();
    let mut h = harness(settings, &state, false);
    frames(&mut h, 2);
    assert_eq!(watch_field(&h).value().as_deref(), Some(first.as_str()));

    type_watch_dir(&mut h, &other.display().to_string());

    let saved = Store::at(&state).load_settings().unwrap();
    assert_eq!(saved.watch_dir, other.display().to_string());
    assert_eq!(saved.recent_watch_dirs, vec![other.display().to_string()]);
    let live = h.state().engine().with_state(|s| s.settings.watch_dir.clone());
    assert_eq!(live, other.display().to_string());
    assert!(h.query_by_label("This folder does not exist.").is_none());
}

#[test]
fn missing_folder_is_refused_and_settings_are_kept() {
    let demo = Demo::new(false);
    let state = demo.state_dir("missing");
    let settings = demo.settings(LanguagePref::English, ThemePref::Light);
    let first = settings.watch_dir.clone();
    Store::at(&state).save_settings(&settings).unwrap();
    let mut h = harness(settings, &state, false);
    frames(&mut h, 2);

    type_watch_dir(&mut h, &demo.root.join("does-not-exist").display().to_string());

    h.get_by_label("This folder does not exist.");
    assert_eq!(Store::at(&state).load_settings().unwrap().watch_dir, first);
    let live = h.state().engine().with_state(|s| s.settings.watch_dir.clone());
    assert_eq!(live, first);
}

#[test]
fn recent_folder_is_picked_from_the_drop_down() {
    let demo = Demo::new(false);
    let state = demo.state_dir("recent");
    let other = demo.root.join("Other");
    fs::create_dir_all(&other).unwrap();
    let mut settings = demo.settings(LanguagePref::English, ThemePref::Light);
    settings.recent_watch_dirs = vec![settings.watch_dir.clone(), other.display().to_string()];
    let mut h = harness(settings, &state, false);
    frames(&mut h, 2);

    // The first "Recent folders" menu is the watched folder's.
    h.get_all_by_label("Recent folders").next().unwrap().click();
    frames(&mut h, 3);
    h.get_by_label(&other.display().to_string()).click();
    frames(&mut h, 3);

    let saved = Store::at(&state).load_settings().unwrap();
    assert_eq!(saved.watch_dir, other.display().to_string());
    assert_eq!(saved.recent_watch_dirs[0], other.display().to_string());
}

#[test]
fn extraction_folder_is_picked_from_its_own_recent_folders() {
    let demo = Demo::new(false);
    let state = demo.state_dir("recent-extract");
    let other = demo.root.join("Elsewhere");
    fs::create_dir_all(&other).unwrap();
    let mut settings = demo.settings(LanguagePref::English, ThemePref::Light);
    let watched = settings.watch_dir.clone();
    settings.recent_extract_dirs = vec![other.display().to_string()];
    let mut h = harness(settings, &state, false);
    frames(&mut h, 2);

    // The second "Recent folders" menu is the extraction folder's.
    h.get_all_by_label("Recent folders").nth(1).unwrap().click();
    frames(&mut h, 3);
    h.get_by_label(&other.display().to_string()).click();
    frames(&mut h, 3);

    let saved = Store::at(&state).load_settings().unwrap();
    assert_eq!(saved.extract_dir, other.display().to_string());
    assert_eq!(saved.watch_dir, watched, "the watched folder is unchanged");
}

#[test]
fn ctrl_comma_opens_the_settings() {
    let demo = Demo::new(false);
    let mut h = harness(
        demo.settings(LanguagePref::English, ThemePref::Light),
        &demo.state_dir("shortcut"),
        false,
    );
    frames(&mut h, 2);
    assert!(h.query_by_label("General").is_none());
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Comma);
    frames(&mut h, 3);
    h.get_by_label("General");
    h.key_press(egui::Key::Escape);
    frames(&mut h, 3);
    assert!(
        h.query_by_label("General").is_none(),
        "Esc closes unchanged settings"
    );
}

#[test]
fn unsaved_changes_are_not_lost_by_accident() {
    let demo = Demo::new(false);
    let state = demo.state_dir("guard");
    let mut h = harness(
        demo.settings(LanguagePref::English, ThemePref::Light),
        &state,
        false,
    );
    frames(&mut h, 2);
    h.get_by_label("⚙  Settings").click();
    frames(&mut h, 3);
    h.get_by_label("Dark").click(); // theme: a change
    frames(&mut h, 2);
    h.get_by_label("Cancel").click();
    frames(&mut h, 3);
    h.get_by_label("Discard your changes?");
    h.get_by_label("Keep editing").click();
    frames(&mut h, 3);
    assert!(h.query_by_label("Discard your changes?").is_none());
    h.get_by_label("General"); // still open, change kept
    h.get_by_label("Cancel").click();
    frames(&mut h, 3);
    h.get_by_label("Discard").click();
    frames(&mut h, 3);
    assert!(h.query_by_label("General").is_none());
    assert!(!state.join("rar-n-rush.toml").exists(), "nothing was saved");
}

#[test]
fn invalid_folder_disables_save() {
    let demo = Demo::new(false);
    let state = demo.state_dir("invalid");
    let mut h = harness(
        demo.settings(LanguagePref::English, ThemePref::Light),
        &state,
        false,
    );
    frames(&mut h, 2);
    h.get_by_label("⚙  Settings").click();
    frames(&mut h, 3);
    let field = h
        .get_all_by_role(egui::accesskit::Role::TextInput)
        .find(|n| n.value().is_some_and(|v| v.ends_with("Downloads")) && n.rect().min.y > 60.0)
        .expect("watched folder field of the settings window");
    field.focus();
    frames(&mut h, 1);
    h.get_all_by_role(egui::accesskit::Role::TextInput)
        .find(egui_kittest::Node::is_focused)
        .unwrap()
        .set_text_value_accesskit(&demo.root.join("missing").display().to_string());
    frames(&mut h, 3);
    h.get_by_label("Watched folder: this folder does not exist.");
    h.get_by_label("Save").click(); // disabled: nothing happens
    frames(&mut h, 3);
    h.get_by_label("General");
    assert!(!state.join("rar-n-rush.toml").exists(), "nothing was saved");
}

#[test]
fn keyboard_selects_and_removes_sets() {
    let demo = Demo::new(false);
    let mut h = harness(
        demo.settings(LanguagePref::English, ThemePref::Light),
        &demo.state_dir("keys"),
        false,
    );
    settle(&mut h, 4);
    let jobs = |h: &Harness<'static, App>| h.state().engine().with_state(|s| s.jobs.len());
    assert_eq!(jobs(&h), 4);
    h.key_press(egui::Key::ArrowDown); // first set
    h.key_press(egui::Key::ArrowDown); // second set
    frames(&mut h, 2);
    let second = h.state().engine().with_state(|s| s.jobs[1].set.name.clone());
    h.key_press(egui::Key::Delete);
    frames(&mut h, 4);
    std::thread::sleep(Duration::from_millis(300)); // the worker handles the command
    frames(&mut h, 2);
    let names: Vec<String> = h
        .state()
        .engine()
        .with_state(|s| s.jobs.iter().map(|j| j.set.name.clone()).collect());
    assert_eq!(names.len(), 3, "{names:?}");
    assert!(!names.contains(&second), "{second} should be removed: {names:?}");
}

#[test]
fn finished_sets_raise_a_short_notice() {
    let demo = Demo::new(false);
    let mut h = harness(
        demo.settings(LanguagePref::English, ThemePref::Light),
        &demo.state_dir("toasts"),
        false,
    );
    settle(&mut h, 4);
    h.get_by_label("Photos — Done");
    h.get_by_label("Private.Documents — Password required");
}

#[test]
fn turning_on_overwrite_asks_first() {
    let demo = Demo::new(false);
    let state = demo.state_dir("overwrite");
    let mut h = harness(
        demo.settings(LanguagePref::English, ThemePref::Light),
        &state,
        false,
    );
    frames(&mut h, 2);
    h.get_by_label("⚙  Settings").click();
    frames(&mut h, 3);
    h.get_by_label("Processing").click();
    frames(&mut h, 2);
    h.get_by_label("Overwrite existing files (otherwise they are kept as they are)")
        .click();
    frames(&mut h, 2);
    h.get_by_label("Save").click();
    frames(&mut h, 3);
    h.get_by_label_contains("replaces existing files for good");
    assert!(
        !state.join("rar-n-rush.toml").exists(),
        "nothing saved before the answer"
    );
    h.get_by_label("Enable").click();
    frames(&mut h, 3);
    assert!(Store::at(&state).load_settings().unwrap().overwrite_existing);
}

/// At the smallest window size, every page of the settings keeps Save reachable.
#[test]
fn settings_fit_the_minimum_window() {
    let demo = Demo::new(false);
    let store = Store::at(demo.state_dir("min"));
    let settings = demo.settings(LanguagePref::French, ThemePref::Light); // longer labels
    let mut h = Harness::builder()
        .with_size(egui::vec2(720.0, 420.0))
        .with_render_cursor(false)
        .build_eframe(move |cc| {
            let ctx = cc.egui_ctx.clone();
            let engine = Engine::start(settings, store.clone(), move || ctx.request_repaint()).unwrap();
            App::new(&cc.egui_ctx, engine, store, None)
        });
    frames(&mut h, 2);
    h.get_by_label("⚙  Réglages").click();
    frames(&mut h, 3);
    for page in ["Général", "Traitement", "Fenêtre"] {
        h.get_by_label(page).click();
        frames(&mut h, 3);
        let save = h.get_by_label("Enregistrer").rect();
        assert!(
            save.max.y <= 420.0 && save.max.x <= 720.0,
            "page {page}: Save at {save:?} is outside the window"
        );
    }
}

/// Regenerates `docs/screenshots/*.png`. Run with:
/// `UPDATE_SCREENSHOTS=1 cargo test --test ui screenshots -- --test-threads=1`
#[test]
fn screenshots() {
    if std::env::var_os("UPDATE_SCREENSHOTS").is_none() {
        return;
    }
    for (lang, lang_tag) in [(LanguagePref::English, "en"), (LanguagePref::French, "fr")] {
        for (theme, theme_tag) in [(ThemePref::Light, "light"), (ThemePref::Dark, "dark")] {
            let demo = Demo::new(true);
            let tag = format!("{lang_tag}-{theme_tag}");
            let mut h = harness(demo.settings(lang, theme), &demo.state_dir(&tag), true);
            h.state_mut().freeze_clock();
            settle(&mut h, 4);
            save(&mut h, &format!("main-{tag}"));
            let settings_label = if lang == LanguagePref::English {
                "⚙  Settings"
            } else {
                "⚙  Réglages"
            };
            h.get_by_label(settings_label).click();
            frames(&mut h, 3);
            save(&mut h, &format!("settings-{tag}"));
            if theme == ThemePref::Light {
                let tabs = if lang == LanguagePref::English {
                    [("Processing", "processing"), ("Window", "window")]
                } else {
                    [("Traitement", "processing"), ("Fenêtre", "window")]
                };
                for (label, page) in tabs {
                    h.get_by_label(label).click();
                    frames(&mut h, 3);
                    save(&mut h, &format!("settings-{page}-{lang_tag}"));
                }
            }
            drop(h);
        }
        let state = tempfile::tempdir().unwrap();
        let settings = Settings {
            language: lang,
            theme: ThemePref::Light,
            ..Settings::default()
        };
        let mut h = harness(settings, state.path(), true);
        h.state_mut().freeze_clock();
        for _ in 0..3 {
            h.step();
        }
        save(&mut h, &format!("onboarding-{lang_tag}"));
        drop(h);
        guide_screenshots(lang, lang_tag);
    }
}

/// Extra screens of the user guide: settings-file banner and the two questions.
fn guide_screenshots(lang: LanguagePref, lang_tag: &str) {
    let t = match lang {
        LanguagePref::French => rar_n_rush::i18n::Lang::Fr,
        _ => rar_n_rush::i18n::Lang::En,
    }
    .texts();

    // A value the user typed by hand was out of range: it is reset, the rest kept.
    let demo = Demo::new(true);
    let issue = LoadIssue::InvalidValues {
        keys: vec!["scan_interval_secs".into()],
        backup: Some(demo.root.join("rar-n-rush.toml.20261114-120000.bak")),
    };
    let settings = demo.settings(lang, ThemePref::Light);
    let mut h = harness_with_issue(settings, &demo.state_dir("banner"), true, Some(issue));
    h.state_mut().freeze_clock();
    settle(&mut h, 4);
    save(&mut h, &format!("banner-{lang_tag}"));

    // Turning on overwriting asks first.
    h.get_by_label(t.settings).click();
    frames(&mut h, 3);
    h.get_by_label(t.group_processing).click();
    frames(&mut h, 2);
    h.get_by_label(t.overwrite).click();
    frames(&mut h, 2);
    h.get_by_label(t.save).click();
    frames(&mut h, 3);
    save(&mut h, &format!("confirm-overwrite-{lang_tag}"));

    // Closing the settings with unsaved changes asks too.
    h.get_by_label(t.cancel).click();
    frames(&mut h, 2);
    h.get_by_label(t.cancel).click();
    frames(&mut h, 3);
    save(&mut h, &format!("confirm-discard-{lang_tag}"));
    drop(h);

    // About window.
    let mut h = harness(
        demo.settings(lang, ThemePref::Light),
        &demo.state_dir("about"),
        true,
    );
    h.state_mut().freeze_clock();
    settle(&mut h, 4);
    h.get_by_label(t.about).click();
    frames(&mut h, 3);
    save(&mut h, &format!("about-{lang_tag}"));
}

/// Fixed folder of the snapshot demo: the paths shown on screen must not change
/// between runs (a temporary folder would).
const SNAPSHOT_ROOT: &str = r"C:\Users\Public\RarNRush-snapshots";

/// Pixel comparison of the main screens with `tests/snapshots/*.png`.
///
/// Times are frozen and notices are off, so every run renders the same image.
/// After an intended visual change, review the `.diff.png` files, then run
/// `UPDATE_SNAPSHOTS=1 cargo test --test ui snapshots` and commit the new images.
#[test]
fn snapshots() {
    let mut results = egui_kittest::SnapshotResults::new();
    let cases = [
        (LanguagePref::English, ThemePref::Light, "en-light"),
        (LanguagePref::English, ThemePref::Dark, "en-dark"),
        (LanguagePref::French, ThemePref::Light, "fr-light"),
    ];
    for (lang, theme, tag) in cases {
        let demo = Demo::in_folder(Some(SNAPSHOT_ROOT));
        let settings = Settings {
            show_toasts: false,
            ..demo.settings(lang, theme)
        };
        let mut h = harness(settings, &demo.state_dir(tag), true);
        h.state_mut().freeze_clock();
        settle(&mut h, 4);
        results.add(h.try_snapshot(format!("main-{tag}")));

        let english = lang == LanguagePref::English;
        h.get_by_label(if english {
            "⚙  Settings"
        } else {
            "⚙  Réglages"
        })
        .click();
        frames(&mut h, 3);
        results.add(h.try_snapshot(format!("settings-general-{tag}")));
        if tag == "en-light" {
            for page in ["Processing", "Window"] {
                h.get_by_label(page).click();
                frames(&mut h, 3);
                results.add(h.try_snapshot(format!("settings-{}-{tag}", page.to_lowercase())));
            }
        }
        drop(h);
        drop(demo);
    }

    for (lang, tag) in [(LanguagePref::English, "en"), (LanguagePref::French, "fr")] {
        let state = tempfile::tempdir().unwrap();
        let settings = Settings {
            language: lang,
            theme: ThemePref::Light,
            ..Settings::default()
        };
        let mut h = harness(settings, state.path(), true);
        h.state_mut().freeze_clock();
        frames(&mut h, 3);
        results.add(h.try_snapshot(format!("onboarding-{tag}")));
        h.get_by_label(if lang == LanguagePref::English {
            "ℹ  About"
        } else {
            "ℹ  À propos"
        })
        .click();
        frames(&mut h, 3);
        results.add(h.try_snapshot(format!("about-{tag}")));
    }
}

#[test]
fn about_shows_the_version_and_links() {
    let demo = Demo::new(false);
    let mut h = harness(
        demo.settings(LanguagePref::English, ThemePref::Light),
        &demo.state_dir("about"),
        false,
    );
    frames(&mut h, 2);
    let version = env!("CARGO_PKG_VERSION");

    h.get_by_label("ℹ  About").click();
    frames(&mut h, 3);
    h.get_by_label(&format!("Version {version}"));
    h.get_by_label("Source code on GitHub");
    h.get_by_label("Close").click();
    frames(&mut h, 3);
    assert!(h.query_by_label("Source code on GitHub").is_none(), "closed");

    // The version in the bottom-right corner opens it too.
    h.get_by_label(&format!("Rar N Rush v{version}")).click();
    frames(&mut h, 3);
    h.get_by_label("Source code on GitHub");
}

/// Records notifications instead of showing them.
#[derive(Clone, Default)]
struct FakeNotifier {
    shown: std::sync::Arc<std::sync::Mutex<Vec<(String, String)>>>,
    enabled: std::sync::Arc<std::sync::Mutex<Vec<bool>>>,
}

impl rar_n_rush::notify::Notifier for FakeNotifier {
    fn notify(&self, title: &str, body: &str) -> Result<(), rar_n_rush::notify::NotifyError> {
        self.shown
            .lock()
            .unwrap()
            .push((title.to_owned(), body.to_owned()));
        Ok(())
    }
    fn set_enabled(&self, enabled: bool) -> Result<(), rar_n_rush::notify::NotifyError> {
        self.enabled.lock().unwrap().push(enabled);
        Ok(())
    }
}

fn notifier_harness(system_notifications: bool) -> (Demo, Harness<'static, App>, FakeNotifier) {
    let demo = Demo::new(false);
    let store = Store::at(demo.state_dir("notify"));
    let settings = Settings {
        system_notifications,
        ..demo.settings(LanguagePref::English, ThemePref::Light)
    };
    let fake = FakeNotifier::default();
    let for_app = fake.clone();
    // The test window never has the keyboard focus: it counts as not in front.
    let h = Harness::builder()
        .with_size(SIZE)
        .with_render_cursor(false)
        .build_eframe(move |cc| {
            let ctx = cc.egui_ctx.clone();
            let engine = Engine::start(settings, store.clone(), move || ctx.request_repaint()).unwrap();
            App::new(&cc.egui_ctx, engine, store, None).with_notifier(for_app)
        });
    (demo, h, fake)
}

#[test]
fn windows_notifications_when_the_window_is_not_in_front() {
    let (_demo, mut h, fake) = notifier_harness(true);
    settle(&mut h, 4);
    assert_eq!(*fake.enabled.lock().unwrap(), vec![true], "registered at start");
    let shown = fake.shown.lock().unwrap().clone();
    assert!(
        shown.contains(&("Private.Documents".into(), "Password required".into())),
        "{shown:?}"
    );
    assert!(shown.contains(&("Photos".into(), "Done".into())), "{shown:?}");
    assert!(
        shown.iter().any(|(t, b)| t == "Damaged.Archive" && b == "Error"),
        "{shown:?}"
    );
    // One notification per change, not one per frame.
    let count = shown.len();
    frames(&mut h, 10);
    assert_eq!(fake.shown.lock().unwrap().len(), count);
}

#[test]
fn no_windows_notification_when_turned_off() {
    let (_demo, mut h, fake) = notifier_harness(false);
    settle(&mut h, 4);
    assert_eq!(*fake.enabled.lock().unwrap(), vec![false], "registration removed");
    assert!(fake.shown.lock().unwrap().is_empty());
}

/// Answers update checks with a fixed release number, without going online.
struct FakeGitHub {
    tag: &'static str,
    calls: std::sync::Arc<std::sync::Mutex<usize>>,
}

impl rar_n_rush::update::Fetch for FakeGitHub {
    fn get(&self, url: &str) -> Result<String, rar_n_rush::update::UpdateError> {
        assert_eq!(url, rar_n_rush::update::LATEST_URL);
        *self.calls.lock().unwrap() += 1;
        Ok(format!(r#"{{"tag_name":"{}"}}"#, self.tag))
    }
}

/// App checking for updates against a fake GitHub answering `tag`.
fn update_harness(
    check_updates: bool,
    tag: &'static str,
) -> (
    Demo,
    Store,
    Harness<'static, App>,
    FakeNotifier,
    std::sync::Arc<std::sync::Mutex<usize>>,
) {
    let demo = Demo::new(false);
    let store = Store::at(demo.state_dir("update"));
    let settings = Settings {
        check_updates,
        ..demo.settings(LanguagePref::English, ThemePref::Light)
    };
    let (fake, calls) = (FakeNotifier::default(), std::sync::Arc::default());
    let (for_app, store_for_app) = (fake.clone(), store.clone());
    let github = FakeGitHub {
        tag,
        calls: std::sync::Arc::clone(&calls),
    };
    let h = Harness::builder()
        .with_size(SIZE)
        .with_render_cursor(false)
        .build_eframe(move |cc| {
            let ctx = cc.egui_ctx.clone();
            let store = store_for_app;
            let engine = Engine::start(settings, store.clone(), move || ctx.request_repaint()).unwrap();
            App::new(&cc.egui_ctx, engine, store, None)
                .with_notifier(for_app)
                .with_update_check(rar_n_rush::update::Checker::with_delay(github, Duration::ZERO))
        });
    (demo, store, h, fake, calls)
}

/// Steps the UI until `done` holds (at most 10 s).
fn step_until(h: &mut Harness<'static, App>, mut done: impl FnMut(&mut Harness<'static, App>) -> bool) {
    let start = Instant::now();
    while !done(h) {
        assert!(start.elapsed() < Duration::from_secs(10), "timed out");
        h.step();
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn a_newer_release_is_announced_once() {
    let (_demo, store, mut h, fake, calls) = update_harness(true, "v99.0.0");
    let current = env!("CARGO_PKG_VERSION");
    let text = format!("Rar N Rush 99.0.0 is available (you have {current}).");
    step_until(&mut h, |h| h.query_by_label(&text).is_some());
    h.get_by_label("See the release");
    // Windows notification (the test window is never in front).
    assert_eq!(
        *fake.shown.lock().unwrap(),
        vec![("Rar N Rush".to_owned(), text.clone())]
    );
    // The date of the check is saved, so the next start does not ask again.
    let saved = store.load_settings().unwrap().last_update_check;
    assert!(saved.is_some_and(|t| t > 1_700_000_000), "{saved:?}");
    frames(&mut h, 20);
    assert_eq!(*calls.lock().unwrap(), 1, "one request");
    // "Later" hides the banner.
    h.get_by_label("Later").click();
    frames(&mut h, 3);
    assert!(h.query_by_label(&text).is_none());
}

#[test]
fn no_banner_when_up_to_date() {
    let (_demo, store, mut h, fake, calls) = update_harness(true, concat!("v", env!("CARGO_PKG_VERSION")));
    step_until(&mut h, |_| *calls.lock().unwrap() == 1);
    step_until(&mut h, |_| {
        store.load_settings().unwrap().last_update_check.is_some()
    });
    frames(&mut h, 5);
    assert!(h.query_by_label("See the release").is_none());
    assert!(fake.shown.lock().unwrap().is_empty());
}

#[test]
fn nothing_is_checked_while_the_option_is_off() {
    let (_demo, store, mut h, _fake, calls) = update_harness(false, "v99.0.0");
    for _ in 0..30 {
        h.step();
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(*calls.lock().unwrap(), 0, "no request");
    assert_eq!(store.load_settings().unwrap().last_update_check, None);
    assert!(h.query_by_label("See the release").is_none());
}

#[test]
fn update_option_in_the_settings() {
    let demo = Demo::new(false);
    let dir = demo.state_dir("update-option");
    let mut h = harness(demo.settings(LanguagePref::French, ThemePref::Light), &dir, false);
    frames(&mut h, 2);
    h.get_by_label("⚙  Réglages").click();
    frames(&mut h, 3);
    h.get_by_label("Vérifier les mises à jour").click();
    frames(&mut h, 2);
    h.get_by_label("Mises à jour").click();
    frames(&mut h, 2);
    h.get_by_label("Une fois par mois").click();
    frames(&mut h, 2);
    h.get_by_label("Enregistrer").click();
    frames(&mut h, 3);
    let saved = Store::at(&dir).load_settings().unwrap();
    assert!(saved.check_updates);
    assert_eq!(
        saved.update_frequency,
        rar_n_rush::settings::UpdateFrequency::Monthly
    );
}
