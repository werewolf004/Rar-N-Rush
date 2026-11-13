//! Headless UI tests (no window is ever shown) and documentation screenshots.
//!
//! The tests drive the real [`App`] through `egui_kittest` and its accessibility
//! tree. Screenshots are only written when `UPDATE_SCREENSHOTS=1` is set; they
//! need a GPU adapter (wgpu) and are therefore not taken in CI.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use rar_n_rush::engine::{Engine, Status};
use rar_n_rush::settings::{LanguagePref, Settings, Store, ThemePref};
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
        let (guard, root) = if neutral_path {
            let root = PathBuf::from(r"C:\Users\Public\RarNRush-demo");
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
    let theme = match settings.theme {
        ThemePref::Dark => egui::Theme::Dark,
        _ => egui::Theme::Light,
    };
    let store = Store::at(store_dir);
    let mut builder = Harness::builder()
        .with_size(SIZE)
        .with_theme(theme)
        .with_render_cursor(false);
    if gpu {
        builder = builder.wgpu();
    }
    builder.build_eframe(move |cc| {
        let ctx = cc.egui_ctx.clone();
        let engine = Engine::start(settings, store.clone(), move || ctx.request_repaint()).unwrap();
        App::new(&cc.egui_ctx, engine, store)
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
    h.get_by_label("Watched folder");
    h.get_by_label("Follow Windows");
    h.get_by_label("Overwrite existing files (otherwise use “name (1).ext”)");
    h.get_by_label("Cancel").click();
    frames(&mut h, 3);
    assert!(
        h.query_by_label("Watched folder").is_none(),
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
    h.get_by_role(egui::accesskit::Role::ComboBox).click(); // language
    frames(&mut h, 3);
    h.get_by_label("Français").click();
    frames(&mut h, 3);
    h.get_by_label("Save").click();
    frames(&mut h, 3);
    h.get_by_label("⚙  Réglages");
    let saved = Store::at(&state).load_settings().unwrap();
    assert_eq!(saved.language, LanguagePref::French);
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
            drop(h);
        }
        let state = tempfile::tempdir().unwrap();
        let settings = Settings {
            language: lang,
            theme: ThemePref::Light,
            ..Settings::default()
        };
        let mut h = harness(settings, state.path(), true);
        for _ in 0..3 {
            h.step();
        }
        save(&mut h, &format!("onboarding-{lang_tag}"));
    }
}
