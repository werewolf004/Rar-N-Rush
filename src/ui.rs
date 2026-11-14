//! Desktop interface (egui).

use crate::engine::{Command, Engine, EngineState, Job, LogLine, Status};
use crate::i18n::{Lang, Localize, Texts};
use crate::settings::{
    LanguagePref, LoadIssue, SCAN_INTERVAL_RANGE, STABLE_RANGE, Settings, SettingsError, Store, ThemePref,
    WindowGeometry, is_writable,
};
use crate::tray::{self, Request, Tray, WindowAction, WindowInput};
use eframe::egui::{self, Color32, RichText};
use egui_extras::{Column, TableBuilder};
use std::collections::HashMap;
use std::path::Path;
use std::time::{Duration, Instant};

/// Main application.
pub struct App {
    engine: Engine,
    store: Store,
    lang: Lang,
    theme: ThemePref,
    /// Settings being edited (window open when `Some`).
    draft: Option<Settings>,
    passwords_text: String,
    password_inputs: HashMap<u64, String>,
    errors_only: bool,
    settings_error: Option<String>,
    /// Text of the watched-folder field in the main window.
    watch_input: String,
    /// `true` while the user edits the watched-folder field (not yet applied).
    watch_dirty: bool,
    /// Why the last watched-folder change was refused.
    watch_error: Option<String>,
    /// Problem found in the settings file at start-up, shown until dismissed.
    load_issue: Option<LoadIssue>,
    /// Warning raised while running (e.g. passwords kept in the file), shown in
    /// the same banner until dismissed.
    warning: Option<String>,
    /// Last set scrolled into view, so the list follows each new active set once
    /// and the user stays free to scroll in between.
    followed: Option<u64>,
    /// Notification-area icon, present while a tray option is enabled.
    tray: Option<(Tray, Lang)>,
    /// Why the tray icon could not be created (shown in the settings window).
    tray_error: Option<String>,
    /// Whether the window is shown, hidden in the tray, or closing for real.
    window: WindowState,
    /// Page shown in the settings window.
    settings_tab: SettingsTab,
    /// Settings and password text as they were when the settings window opened,
    /// to detect unsaved changes.
    draft_original: Option<(Settings, String)>,
    /// Question shown in place of Save / Cancel in the settings window.
    confirm: Option<Confirm>,
    /// Set selected in the list (keyboard navigation, Del, Enter).
    selected: Option<u64>,
    /// Scroll the list to the selected set on the next frame.
    scroll_to_selected: bool,
    /// Last status seen for each set, to notice when one finishes.
    seen: HashMap<u64, Status>,
    /// Short notices shown in the bottom-right corner.
    toasts: Vec<Toast>,
    /// Last known size and position of the main window.
    geometry: Option<WindowGeometry>,
}

/// Pages of the settings window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SettingsTab {
    /// Folders, par2.exe, timings, language and theme.
    General,
    /// Processing options, clean-up and passwords.
    Processing,
    /// Window and notification area.
    Window,
}

/// Question asked before an action in the settings window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Confirm {
    /// Close with unsaved changes?
    Discard,
    /// Turn on overwriting, which cannot be undone?
    Overwrite,
}

/// Height taken by the settings window around its page: title bar, tabs,
/// messages and buttons, plus a margin from the edges of the main window.
const SETTINGS_CHROME_HEIGHT: f32 = 190.0;

/// How long a notice stays on screen.
const TOAST_DURATION: Duration = Duration::from_secs(6);
/// At most this many notices at once; older ones go first.
const TOAST_MAX: usize = 4;

/// A short, non-blocking notice ("“Photos” done").
#[derive(Debug, Clone)]
struct Toast {
    text: String,
    status: Status,
    shown_at: Instant,
}

/// State of the main window with respect to the notification area.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WindowState {
    /// Normal window.
    Shown,
    /// Hidden in the notification area.
    Hidden,
    /// **Quit** was chosen in the tray menu: the next close is real.
    Quitting,
}

/// Snapshot of the engine state taken once per frame, so the lock is held briefly.
struct Snapshot {
    jobs: Vec<Job>,
    log: Vec<LogLine>,
    running: bool,
    settings: Settings,
    last_scan: Option<Instant>,
}

impl Snapshot {
    fn take(state: &EngineState) -> Self {
        Self {
            jobs: state.jobs.clone(),
            log: state.log.clone(),
            running: state.running,
            settings: state.settings.clone(),
            last_scan: state.last_scan,
        }
    }
}

/// Status colours chosen for at least 4.5:1 contrast on egui's default backgrounds.
fn status_color(status: Status, dark: bool) -> Color32 {
    match (status, dark) {
        (Status::Done, true) => Color32::from_rgb(110, 205, 125),
        (Status::Done, false) => Color32::from_rgb(25, 120, 45),
        (Status::Failed | Status::RepairImpossible, true) => Color32::from_rgb(255, 125, 115),
        (Status::Failed | Status::RepairImpossible, false) => Color32::from_rgb(185, 35, 25),
        (Status::NeedPassword, true) => Color32::from_rgb(245, 185, 85),
        (Status::NeedPassword, false) => Color32::from_rgb(150, 85, 0),
        (Status::Verifying | Status::Repairing | Status::Extracting, true) => {
            Color32::from_rgb(120, 175, 255)
        }
        (Status::Verifying | Status::Repairing | Status::Extracting, false) => Color32::from_rgb(20, 85, 195),
        (_, true) => Color32::from_gray(185),
        (_, false) => Color32::from_gray(80),
    }
}

/// Fill of the selected row: softer than egui's default selection colour, so every
/// status colour keeps a 4.5:1 contrast on it.
fn selection_fill(dark: bool) -> Color32 {
    if dark {
        Color32::from_rgb(35, 50, 70)
    } else {
        Color32::from_rgb(220, 236, 252)
    }
}

fn theme_preference(theme: ThemePref) -> egui::ThemePreference {
    match theme {
        ThemePref::System => egui::ThemePreference::System,
        ThemePref::Light => egui::ThemePreference::Light,
        ThemePref::Dark => egui::ThemePreference::Dark,
    }
}

fn open_in_explorer(path: &Path) {
    // Fire and forget: failure to open Explorer is not worth interrupting the user.
    let _ = std::process::Command::new("explorer.exe").arg(path).spawn();
}

impl App {
    /// Creates the application and starts the engine.
    pub fn new(ctx: &egui::Context, engine: Engine, store: Store, load_issue: Option<LoadIssue>) -> Self {
        let settings = engine.with_state(|s| s.settings.clone());
        ctx.set_theme(theme_preference(settings.theme));
        Self {
            engine,
            store,
            lang: Lang::resolve(settings.language),
            theme: settings.theme,
            draft: None,
            passwords_text: String::new(),
            password_inputs: HashMap::new(),
            errors_only: false,
            settings_error: None,
            watch_input: settings.watch_dir.clone(),
            watch_dirty: false,
            watch_error: None,
            load_issue,
            warning: None,
            followed: None,
            tray: None,
            tray_error: None,
            window: WindowState::Shown,
            settings_tab: SettingsTab::General,
            draft_original: None,
            confirm: None,
            selected: None,
            scroll_to_selected: false,
            seen: HashMap::new(),
            toasts: Vec::new(),
            geometry: settings.window,
        }
    }

    /// The engine driven by this window (read access for tests and tooling).
    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    fn texts(&self) -> &'static Texts {
        self.lang.texts()
    }

    fn open_settings(&mut self, settings: &Settings) {
        if self.draft.is_some() {
            return; // already open: keep the edits in progress
        }
        self.passwords_text = settings.passwords.join("\n");
        self.draft = Some(settings.clone());
        self.draft_original = Some((settings.clone(), self.passwords_text.clone()));
        self.settings_error = None;
        self.confirm = None;
    }

    /// `true` when the settings window holds changes that are not saved.
    fn draft_is_dirty(&self, draft: &Settings) -> bool {
        self.draft_original
            .as_ref()
            .is_some_and(|(settings, passwords)| settings != draft || *passwords != self.passwords_text)
    }

    /// Records the size and position of the main window (not while minimised,
    /// hidden or maximised, so the normal size is kept).
    fn track_geometry(&mut self, ctx: &egui::Context) {
        if self.window != WindowState::Shown {
            return;
        }
        let (outer, inner, maximized, minimized) = ctx.input(|i| {
            let v = i.viewport();
            (
                v.outer_rect,
                v.inner_rect,
                v.maximized.unwrap_or(false),
                v.minimized.unwrap_or(false),
            )
        });
        if minimized {
            return;
        }
        let previous = self.geometry;
        self.geometry = match (outer, inner) {
            (Some(outer), Some(inner)) if !maximized => Some(
                WindowGeometry {
                    x: round(outer.min.x),
                    y: round(outer.min.y),
                    width: round(inner.width()),
                    height: round(inner.height()),
                    maximized: false,
                }
                .validated(),
            ),
            _ => previous.map(|g| WindowGeometry { maximized, ..g }),
        };
    }

    /// Saves the window geometry when it changed. Called when the app exits.
    fn save_geometry(&self) {
        let mut settings = self.engine.with_state(|s| s.settings.clone());
        if settings.window == self.geometry || self.geometry.is_none() {
            return;
        }
        settings.window = self.geometry;
        // Best effort: the app is closing, there is no one left to show an error to,
        // and every other setting was already saved when it was changed.
        if self.store.save_settings(&settings).is_ok() {
            self.engine.send(Command::UpdateSettings(settings));
        }
    }

    /// Keyboard shortcuts of the main window.
    fn shortcuts(&mut self, ctx: &egui::Context, snap: &Snapshot) {
        let (scan, settings, escape) = ctx.input_mut(|i| {
            (
                i.consume_key(egui::Modifiers::NONE, egui::Key::F5),
                i.consume_shortcut(&egui::KeyboardShortcut::new(
                    egui::Modifiers::COMMAND,
                    egui::Key::Comma,
                )),
                i.key_pressed(egui::Key::Escape),
            )
        });
        // List keys only when nothing else (text field, settings window) has them.
        if self.draft.is_none() && ctx.memory(|m| m.focused().is_none()) {
            self.list_keys(ctx, snap);
        }
        if scan && snap.running {
            self.engine.send(Command::ScanNow);
        }
        if settings {
            self.open_settings(&snap.settings);
        }
        // Esc closes the settings window, unless a text field uses it to cancel editing.
        if escape && self.draft.is_some() && ctx.memory(|m| m.focused().is_none()) {
            self.request_close_settings();
        }
    }

    /// ↑ / ↓ select a set, Del removes it from the list, Enter opens its folder.
    fn list_keys(&mut self, ctx: &egui::Context, snap: &Snapshot) {
        let (up, down, delete, enter) = ctx.input_mut(|i| {
            (
                i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp),
                i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown),
                i.consume_key(egui::Modifiers::NONE, egui::Key::Delete),
                i.consume_key(egui::Modifiers::NONE, egui::Key::Enter),
            )
        });
        if snap.jobs.is_empty() {
            self.selected = None;
            return;
        }
        let index = self
            .selected
            .and_then(|id| snap.jobs.iter().position(|j| j.id == id));
        let last = snap.jobs.len() - 1;
        let moved = match (index, up, down) {
            (None, true, _) => Some(last),
            (None, _, true) => Some(0),
            (Some(i), true, _) => Some(i.saturating_sub(1)),
            (Some(i), _, true) => Some((i + 1).min(last)),
            _ => None,
        };
        if let Some(job) = moved.and_then(|i| snap.jobs.get(i)) {
            self.selected = Some(job.id);
            self.scroll_to_selected = true;
        }
        let Some((i, job)) = index.and_then(|i| snap.jobs.get(i).map(|j| (i, j))) else {
            return;
        };
        if delete && !job.status.is_active() {
            self.engine.send(Command::Forget(job.id));
            // Keep a selection so Del can be pressed again.
            let next = snap
                .jobs
                .get(i + 1)
                .or_else(|| i.checked_sub(1).and_then(|p| snap.jobs.get(p)));
            self.selected = next.map(|j| j.id);
        }
        if enter {
            open_in_explorer(&job.set.dir);
        }
    }

    /// Adds a notice for each set that just finished, failed or needs a password.
    fn collect_toasts(&mut self, snap: &Snapshot) {
        let lang = self.lang;
        for job in &snap.jobs {
            let previous = self.seen.insert(job.id, job.status);
            let worth_telling = matches!(
                job.status,
                Status::Done | Status::Failed | Status::NeedPassword | Status::RepairImpossible
            );
            if snap.settings.show_toasts && worth_telling && previous.is_some_and(|p| p != job.status) {
                self.toasts.push(Toast {
                    text: format!("{} — {}", job.set.name, job.status.localize(lang)),
                    status: job.status,
                    shown_at: Instant::now(),
                });
            }
        }
        self.seen.retain(|id, _| snap.jobs.iter().any(|j| j.id == *id));
        self.toasts.retain(|t| t.shown_at.elapsed() < TOAST_DURATION);
        let excess = self.toasts.len().saturating_sub(TOAST_MAX);
        self.toasts.drain(..excess);
    }

    /// Draws the notices in the bottom-right corner, newest at the bottom.
    fn show_toasts(&self, ctx: &egui::Context, dark: bool) {
        if self.toasts.is_empty() {
            return;
        }
        ctx.request_repaint_after(Duration::from_millis(250));
        egui::Area::new(egui::Id::unique("toasts"))
            .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-16.0, -16.0))
            .order(egui::Order::Foreground)
            .interactable(false)
            .show(ctx, |ui| {
                ui.with_layout(egui::Layout::top_down(egui::Align::Max), |ui| {
                    for toast in &self.toasts {
                        egui::Frame::popup(ui.style()).show(ui, |ui| {
                            let text = RichText::new(&toast.text).color(status_color(toast.status, dark));
                            ui.add(egui::Label::new(text).wrap_mode(egui::TextWrapMode::Extend));
                        });
                        ui.add_space(6.0);
                    }
                });
            });
    }

    /// Closes the settings window, or asks first when there are unsaved changes.
    fn request_close_settings(&mut self) {
        match &self.draft {
            Some(draft) if self.draft_is_dirty(draft) => self.confirm = Some(Confirm::Discard),
            _ => self.close_settings(),
        }
    }

    fn close_settings(&mut self) {
        self.draft = None;
        self.draft_original = None;
        self.confirm = None;
        self.settings_error = None;
    }

    /// Saves the settings, then applies them. Nothing is applied when saving fails,
    /// so what the user sees always matches the file. Returns the localised error.
    fn apply_settings(&mut self, ctx: &egui::Context, settings: Settings) -> Result<(), String> {
        let settings = settings.validated();
        match self.store.save_settings(&settings) {
            Ok(()) => {}
            // Saved, with the passwords in the file: apply, and say so.
            Err(SettingsError::PasswordsNotStored(e)) => {
                self.warning = Some(self.lang.passwords_in_file(&e.to_string()));
            }
            Err(e) => return Err(self.lang.save_failed(&e.to_string())),
        }
        self.lang = Lang::resolve(settings.language);
        if self.theme != settings.theme {
            self.theme = settings.theme;
            ctx.set_theme(theme_preference(settings.theme));
        }
        self.engine.send(Command::UpdateSettings(settings));
        Ok(())
    }

    /// Opens the folder picker, starting from the current folder when it exists.
    fn pick_watch_dir(&self, current: &Settings) -> Option<String> {
        let mut dialog = rfd::FileDialog::new().set_title(self.texts().pick_watch_dir);
        if let Some(dir) = current.watch_path().filter(|d| d.is_dir()) {
            dialog = dialog.set_directory(dir);
        }
        dialog.pick_folder().map(|d| d.to_string_lossy().into_owned())
    }

    /// Watches `dir` from now on. Refused (with a message) when it is not an existing
    /// folder. Monitoring starts if no folder was watched before; otherwise the
    /// running/paused state is kept.
    fn set_watch_dir(&mut self, ctx: &egui::Context, current: &Settings, dir: &str) {
        let dir = dir.trim();
        if !Path::new(dir).is_dir() {
            dir.clone_into(&mut self.watch_input);
            self.watch_dirty = true;
            self.watch_error = Some(self.texts().folder_missing.to_owned());
            return;
        }
        let first_folder = current.watch_path().is_none();
        let settings = current.clone().with_watch_dir(dir);
        match self.apply_settings(ctx, settings.clone()) {
            Ok(()) => {
                self.watch_input = settings.watch_dir;
                self.watch_dirty = false;
                self.watch_error = None;
                // An open settings window must not put the old folder back on save.
                if let Some(draft) = &mut self.draft {
                    draft.watch_dir.clone_from(&self.watch_input);
                    draft.recent_watch_dirs = settings.recent_watch_dirs;
                }
                if first_folder {
                    self.engine.send(Command::SetRunning(true));
                }
            }
            Err(e) => self.watch_error = Some(e),
        }
    }

    fn toolbar(&mut self, ui: &mut egui::Ui, snap: &Snapshot) {
        let t = self.texts();
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.heading(RichText::new("Rar N Rush").strong());
            ui.separator();
            if snap.running {
                if ui.button(t.pause).on_hover_text(t.pause_hint).clicked() {
                    self.engine.send(Command::SetRunning(false));
                }
            } else {
                let can_run = snap.settings.watch_path().is_some();
                if ui
                    .add_enabled(can_run, egui::Button::new(t.start))
                    .on_hover_text(t.start_hint)
                    .clicked()
                {
                    self.engine.send(Command::SetRunning(true));
                }
            }
            if ui
                .add_enabled(snap.running, egui::Button::new(t.scan_now))
                .on_hover_text(t.scan_now_hint)
                .clicked()
            {
                self.engine.send(Command::ScanNow);
            }
            if ui.button(t.settings).on_hover_text(t.settings_hint).clicked() {
                self.open_settings(&snap.settings);
            }
            ui.separator();
            if snap.settings.watch_path().is_none() {
                ui.label(RichText::new(t.no_folder).italics());
            } else {
                ui.label(if snap.running { t.watching } else { t.paused });
                if let (true, Some(at)) = (snap.running, snap.last_scan) {
                    ui.weak(format!("· {}", self.lang.scanned_ago(at.elapsed().as_secs())));
                }
            }
        });
        ui.add_space(4.0);
        self.watch_bar(ui, snap);
        ui.add_space(6.0);
    }

    /// Watched-folder bar: editable path, recent folders, browse and open buttons.
    /// The folder can be changed at any time, even while sets are being processed.
    fn watch_bar(&mut self, ui: &mut egui::Ui, snap: &Snapshot) {
        let t = self.texts();
        let dark = ui.visuals().dark_mode;
        if !self.watch_dirty {
            self.watch_input.clone_from(&snap.settings.watch_dir);
        }
        let mut chosen: Option<String> = None;
        ui.horizontal(|ui| {
            let label = ui.label(t.watch_dir);
            let width = (ui.available_width() - 300.0).clamp(160.0, 640.0);
            let field = ui
                .add(
                    egui::TextEdit::singleline(&mut self.watch_input)
                        .hint_text(t.watch_hint)
                        .desired_width(width),
                )
                .labelled_by(label.id);
            if field.changed() {
                self.watch_dirty = true;
                self.watch_error = None;
            }
            if field.lost_focus() {
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    self.watch_dirty = false;
                    self.watch_error = None;
                } else if self.watch_dirty && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    chosen = Some(self.watch_input.clone());
                }
            }
            if self.watch_dirty && ui.button(t.ok).on_hover_text(t.watch_apply_hint).clicked() {
                chosen = Some(self.watch_input.clone());
            }
            if let Some(dir) = recent_folders_menu(ui, &snap.settings, t) {
                chosen = Some(dir);
            }
            if ui.button(t.browse).on_hover_text(t.pick_watch_dir).clicked()
                && let Some(dir) = self.pick_watch_dir(&snap.settings)
            {
                chosen = Some(dir);
            }
            if let Some(dir) = snap.settings.watch_path()
                && ui.button(t.open).on_hover_text(t.open_in_explorer).clicked()
            {
                open_in_explorer(&dir);
            }
        });
        if let Some(e) = &self.watch_error {
            ui.colored_label(status_color(Status::Failed, dark), e);
        }
        if let Some(dir) = chosen {
            self.set_watch_dir(ui.ctx(), &snap.settings, &dir);
        }
    }

    /// Warning shown when the settings file had a problem at start-up.
    fn load_issue_banner(&mut self, ui: &mut egui::Ui, dark: bool) {
        let Some(text) = self
            .warning
            .clone()
            .or_else(|| self.load_issue.as_ref().map(|i| i.localize(self.lang)))
        else {
            return;
        };
        let mut dismiss = false;
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            ui.colored_label(
                status_color(Status::NeedPassword, dark),
                RichText::new("⚠").strong(),
            );
            ui.label(RichText::new(text).color(status_color(Status::NeedPassword, dark)));
            dismiss = ui.button(self.texts().ok).clicked();
        });
        ui.add_space(4.0);
        // The warning (newest) goes first, then the start-up problem.
        if dismiss && self.warning.take().is_none() {
            self.load_issue = None;
        }
    }

    fn log_panel(&mut self, ui: &mut egui::Ui, snap: &Snapshot, dark: bool) {
        let t = self.texts();
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.strong(t.log);
            ui.checkbox(&mut self.errors_only, t.errors_only);
            if ui
                .add_enabled(!snap.log.is_empty(), egui::Button::new(t.export_log))
                .on_hover_text(t.export_log_hint)
                .clicked()
            {
                self.export_log(snap);
            }
        });
        egui::ScrollArea::vertical()
            .stick_to_bottom(true)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for line in snap
                    .log
                    .iter()
                    .filter(|l| !self.errors_only || l.notice.is_error())
                {
                    let text = RichText::new(format!("{}  {}", line.time, line.notice.localize(self.lang)))
                        .monospace();
                    ui.label(if line.notice.is_error() {
                        text.color(status_color(Status::Failed, dark))
                    } else {
                        text
                    });
                }
            });
    }

    /// Text of the log lines shown (same filter and language as the window).
    fn visible_log(&self, snap: &Snapshot) -> String {
        use std::fmt::Write as _;
        snap.log
            .iter()
            .filter(|l| !self.errors_only || l.notice.is_error())
            .fold(String::new(), |mut text, l| {
                // Writing to a String cannot fail.
                let _ = write!(text, "{}  {}\r\n", l.time, l.notice.localize(self.lang));
                text
            })
    }

    fn export_log(&mut self, snap: &Snapshot) {
        let t = self.texts();
        let name = format!(
            "rar-n-rush-log-{}.txt",
            chrono::Local::now().format("%Y%m%d-%H%M%S")
        );
        let Some(path) = rfd::FileDialog::new()
            .set_file_name(name)
            .add_filter(t.text_filter, &["txt"])
            .save_file()
        else {
            return;
        };
        if let Err(e) = std::fs::write(&path, self.visible_log(snap)) {
            self.warning = Some(self.lang.export_failed(&e.to_string()));
        }
    }

    fn jobs_table(&mut self, ui: &mut egui::Ui, snap: &Snapshot, dark: bool) {
        let t = self.texts();
        let lang = self.lang;
        let mut actions: Vec<Command> = Vec::new();
        ui.visuals_mut().selection.bg_fill = selection_fill(dark);
        let mut table = TableBuilder::new(ui).sense(egui::Sense::click());
        if self.scroll_to_selected
            && let Some(row) = self
                .selected
                .and_then(|id| snap.jobs.iter().position(|j| j.id == id))
        {
            self.scroll_to_selected = false;
            table = table.scroll_to_row(row, None);
        }
        if snap.settings.follow_active
            && let Some((row, job)) = snap.jobs.iter().enumerate().find(|(_, j)| j.status.is_active())
            && self.followed != Some(job.id)
        {
            self.followed = Some(job.id);
            table = table.scroll_to_row(row, Some(egui::Align::Center));
        }
        table
            .striped(true)
            .resizable(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::initial(220.0).at_least(120.0).clip(true))
            .column(Column::initial(85.0).at_least(60.0))
            .column(Column::initial(150.0).at_least(90.0))
            .column(Column::initial(120.0).at_least(80.0))
            .column(Column::remainder().at_least(120.0).clip(true))
            .column(Column::exact(250.0))
            .header(24.0, |mut header| {
                for title in [
                    t.col_name,
                    t.col_type,
                    t.col_status,
                    t.col_progress,
                    t.col_detail,
                    "",
                ] {
                    header.col(|ui| {
                        ui.strong(title);
                    });
                }
            })
            .body(|mut body| {
                for job in &snap.jobs {
                    body.row(28.0, |mut row| {
                        row.col(|ui| {
                            ui.label(&job.set.name).on_hover_text(
                                lang.set_tooltip(&job.set.dir.display().to_string(), job.set.files.len()),
                            );
                        });
                        row.col(|ui| {
                            ui.label(job.set.kind().label());
                        });
                        row.col(|ui| {
                            ui.label(
                                RichText::new(job.status.localize(lang))
                                    .color(status_color(job.status, dark)),
                            );
                        });
                        row.col(|ui| {
                            if job.status.is_active() || job.status == Status::Done {
                                ui.add(
                                    egui::ProgressBar::new(job.progress)
                                        .show_percentage()
                                        .desired_height(16.0),
                                );
                            }
                        });
                        row.col(|ui| {
                            let detail = job.detail.localize(lang);
                            ui.label(&detail).on_hover_text(&detail);
                        });
                        row.col(|ui| self.row_actions(ui, job, &mut actions));
                        row.set_selected(self.selected == Some(job.id));
                        if row.response().clicked() {
                            self.selected = Some(job.id);
                        }
                    });
                }
            });
        for action in actions {
            self.engine.send(action);
        }
    }

    fn row_actions(&mut self, ui: &mut egui::Ui, job: &Job, actions: &mut Vec<Command>) {
        let t = self.texts();
        if job.status == Status::NeedPassword {
            let input = self.password_inputs.entry(job.id).or_default();
            let response = ui.add(
                egui::TextEdit::singleline(input)
                    .password(true)
                    .hint_text(t.password_hint)
                    .desired_width(110.0),
            );
            let submitted = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if (ui.button(t.ok).clicked() || submitted) && !input.is_empty() {
                actions.push(Command::Password(job.id, std::mem::take(input)));
            }
        }
        match job.status {
            Status::Failed | Status::RepairImpossible | Status::Skipped | Status::Done => {
                if ui.small_button(t.retry).clicked() {
                    actions.push(Command::Retry(job.id));
                }
            }
            Status::NeedPassword => {}
            _ => {
                if ui.small_button(t.skip).on_hover_text(t.skip_hint).clicked() {
                    actions.push(Command::Skip(job.id));
                }
            }
        }
        if ui.small_button(t.open).on_hover_text(t.open_folder).clicked() {
            open_in_explorer(&job.set.dir);
        }
        if !job.status.is_active() && ui.small_button("✖").on_hover_text(t.remove).clicked() {
            actions.push(Command::Forget(job.id));
        }
    }

    fn settings_window(&mut self, ctx: &egui::Context, dark: bool) {
        let Some(mut draft) = self.draft.take() else {
            return;
        };
        let t = self.texts();
        let lang = self.lang;
        let mut open = true;
        let (mut save, mut cancel, mut accept, mut keep) = (false, false, false, false);
        let problems = draft_problems(&draft, t);
        egui::Window::new(t.settings_title)
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(560.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let tab = &mut self.settings_tab;
                    ui.selectable_value(tab, SettingsTab::General, t.tab_general);
                    ui.selectable_value(tab, SettingsTab::Processing, t.group_processing);
                    ui.selectable_value(tab, SettingsTab::Window, t.group_window);
                });
                ui.separator();
                // The page scrolls when the window is small; the tabs and the
                // Save / Cancel buttons always stay visible.
                let page_height = (ctx.content_rect().height() - SETTINGS_CHROME_HEIGHT).max(80.0);
                egui::ScrollArea::vertical()
                    .max_height(page_height)
                    .auto_shrink([false, true])
                    .show(ui, |ui| match self.settings_tab {
                        SettingsTab::General => {
                            egui::Grid::new("settings_grid")
                                .num_columns(2)
                                .spacing([12.0, 8.0])
                                .show(ui, |ui| settings_grid(ui, &mut draft, t, lang, dark));
                        }
                        SettingsTab::Processing => {
                            processing_toggles(ui, &mut draft, t);
                            ui.add_space(6.0);
                            ui.strong(t.passwords);
                            ui.add(
                                egui::TextEdit::multiline(&mut self.passwords_text)
                                    .desired_rows(3)
                                    .desired_width(f32::INFINITY),
                            );
                            ui.weak(if self.store.has_vault() {
                                t.passwords_hint_vault
                            } else {
                                t.passwords_hint
                            });
                        }
                        SettingsTab::Window => window_toggles(ui, &mut draft, t),
                    });
                ui.separator();
                let error = status_color(Status::Failed, dark);
                for problem in &problems {
                    ui.colored_label(error, *problem);
                }
                if let Some(e) = &self.tray_error {
                    ui.colored_label(status_color(Status::NeedPassword, dark), e);
                }
                if let Some(e) = &self.settings_error {
                    ui.colored_label(error, e);
                }
                ui.add_space(8.0);
                if let Some(confirm) = self.confirm {
                    let (question, yes, no) = match confirm {
                        Confirm::Discard => (t.discard_question, t.discard, t.keep_editing),
                        Confirm::Overwrite => (t.overwrite_question, t.enable, t.cancel),
                    };
                    ui.colored_label(status_color(Status::NeedPassword, dark), question);
                    ui.horizontal(|ui| {
                        accept = ui.button(yes).clicked();
                        keep = ui.button(no).clicked();
                    });
                } else {
                    ui.horizontal(|ui| {
                        save = ui
                            .add_enabled(problems.is_empty(), egui::Button::new(t.save))
                            .clicked();
                        cancel = ui.button(t.cancel).clicked();
                    });
                }
            });
        self.draft = Some(draft);
        if keep {
            self.confirm = None;
        } else if accept {
            match self.confirm.take() {
                Some(Confirm::Discard) => self.close_settings(),
                Some(Confirm::Overwrite) => self.save_draft(ctx),
                None => {}
            }
        } else if save {
            if self.turns_on_overwrite() {
                self.confirm = Some(Confirm::Overwrite);
            } else {
                self.save_draft(ctx);
            }
        } else if cancel || !open {
            self.request_close_settings();
        }
    }

    /// `true` when the settings being saved switch overwriting on.
    fn turns_on_overwrite(&self) -> bool {
        let before = self
            .draft_original
            .as_ref()
            .is_some_and(|(s, _)| s.overwrite_existing);
        let after = self.draft.as_ref().is_some_and(|d| d.overwrite_existing);
        after && !before
    }

    /// Saves the settings window; it stays open with the error when saving fails.
    fn save_draft(&mut self, ctx: &egui::Context) {
        let Some(mut draft) = self.draft.clone() else {
            return;
        };
        draft.passwords = self
            .passwords_text
            .lines()
            .map(str::trim)
            .map(str::to_owned)
            .collect();
        match self.apply_settings(ctx, draft) {
            Ok(()) => self.close_settings(),
            Err(e) => self.settings_error = Some(e),
        }
    }
}

/// Problems that prevent saving the settings, shown above the Save button.
fn draft_problems(draft: &Settings, t: &Texts) -> Vec<&'static str> {
    let mut problems = Vec::new();
    if draft.watch_path().is_some_and(|dir| !dir.is_dir()) {
        problems.push(t.problem_watch_dir);
    }
    if let Some(dest) = draft.extract_path().filter(|d| d.exists()) {
        if !dest.is_dir() {
            problems.push(t.problem_dest_not_dir);
        } else if !is_writable(&dest) {
            problems.push(t.problem_dest_read_only);
        }
    }
    if !draft.par2_path.trim().is_empty() && !Path::new(draft.par2_path.trim()).is_file() {
        problems.push(t.problem_par2);
    }
    problems
}

/// Rounds a logical coordinate to whole pixels, saturating.
fn round(x: f32) -> i32 {
    // Clamped first, so the conversion cannot truncate.
    #[allow(clippy::cast_possible_truncation)]
    let value = x.round().clamp(-1.0e9, 1.0e9) as i32;
    value
}

/// Two-column part of the settings window: folders, timings, language and theme.
fn settings_grid(ui: &mut egui::Ui, draft: &mut Settings, t: &Texts, lang: Lang, dark: bool) {
    folder_row(ui, t.watch_dir, &mut draft.watch_dir, "", t.browse);
    folder_row(
        ui,
        t.destination,
        &mut draft.extract_dir,
        t.destination_hint,
        t.browse,
    );

    ui.label(t.par2_exe);
    ui.horizontal(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut draft.par2_path)
                .hint_text(t.par2_auto)
                .desired_width(300.0),
        );
        if ui.button(t.browse).clicked()
            && let Some(f) = rfd::FileDialog::new()
                .add_filter(t.program_filter, &["exe"])
                .pick_file()
        {
            draft.par2_path = f.to_string_lossy().into_owned();
        }
    });
    ui.end_row();
    ui.label("");
    match draft.resolve_par2() {
        Some(p) => ui.weak(lang.par2_used(&p.display().to_string())),
        None => ui.colored_label(status_color(Status::NeedPassword, dark), t.par2_missing),
    };
    ui.end_row();

    ui.label(t.scan_every);
    ui.add(
        egui::DragValue::new(&mut draft.scan_interval_secs)
            .range(SCAN_INTERVAL_RANGE)
            .suffix(t.seconds),
    );
    ui.end_row();
    ui.label(t.wait_stable);
    ui.add(
        egui::DragValue::new(&mut draft.stable_secs)
            .range(STABLE_RANGE)
            .suffix(t.seconds_unchanged),
    );
    ui.end_row();

    ui.label(t.language);
    egui::ComboBox::from_id_salt("language")
        .selected_text(match draft.language {
            LanguagePref::System => t.follow_system,
            LanguagePref::English => t.english,
            LanguagePref::French => t.french,
        })
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut draft.language, LanguagePref::System, t.follow_system);
            ui.selectable_value(&mut draft.language, LanguagePref::English, t.english);
            ui.selectable_value(&mut draft.language, LanguagePref::French, t.french);
        });
    ui.end_row();
    ui.label(t.theme);
    ui.horizontal(|ui| {
        ui.selectable_value(&mut draft.theme, ThemePref::System, t.follow_system);
        ui.selectable_value(&mut draft.theme, ThemePref::Light, t.light);
        ui.selectable_value(&mut draft.theme, ThemePref::Dark, t.dark);
    });
    ui.end_row();
}

/// Drop-down listing the recently watched folders. Returns the folder picked, if any.
fn recent_folders_menu(ui: &mut egui::Ui, settings: &Settings, t: &Texts) -> Option<String> {
    let recents: Vec<&String> = settings
        .recent_watch_dirs
        .iter()
        .filter(|d| **d != settings.watch_dir)
        .collect();
    let mut picked = None;
    ui.add_enabled_ui(!recents.is_empty(), |ui| {
        ui.menu_button(t.recent_folders, |ui| {
            ui.set_min_width(360.0);
            for dir in recents {
                if ui.button(dir.as_str()).clicked() {
                    picked = Some(dir.clone());
                }
            }
        });
    });
    picked
}

/// Check boxes of the Processing page.
fn processing_toggles(ui: &mut egui::Ui, draft: &mut Settings, t: &Texts) {
    ui.checkbox(&mut draft.recursive, t.recursive);
    ui.checkbox(&mut draft.create_subfolder, t.subfolder);
    ui.checkbox(&mut draft.extract_nested, t.nested);
    ui.checkbox(&mut draft.overwrite_existing, t.overwrite);
    ui.checkbox(&mut draft.autostart, t.autostart);
    ui.add_space(6.0);
    ui.strong(t.group_cleanup);
    ui.checkbox(&mut draft.move_extra_files, t.move_extra_files)
        .on_hover_text(t.move_extra_files_hint);
    ui.checkbox(&mut draft.delete_after_success, t.trash_after);
    ui.checkbox(&mut draft.delete_empty_dirs, t.delete_empty_dirs);
}

/// Check boxes of the Window page.
fn window_toggles(ui: &mut egui::Ui, draft: &mut Settings, t: &Texts) {
    ui.checkbox(&mut draft.follow_active, t.follow_active);
    ui.checkbox(&mut draft.log_to_file, t.log_to_file);
    ui.checkbox(&mut draft.show_toasts, t.show_toasts);
    ui.checkbox(&mut draft.minimize_to_tray, t.minimize_to_tray);
    ui.checkbox(&mut draft.close_to_tray, t.close_to_tray)
        .on_hover_text(t.close_to_tray_hint);
}

fn folder_row(ui: &mut egui::Ui, label: &str, value: &mut String, hint: &str, browse: &str) {
    ui.label(label);
    ui.horizontal(|ui| {
        ui.add(
            egui::TextEdit::singleline(value)
                .hint_text(hint)
                .desired_width(300.0),
        );
        if ui.button(browse).clicked()
            && let Some(dir) = rfd::FileDialog::new().pick_folder()
        {
            *value = dir.to_string_lossy().into_owned();
        }
    });
    ui.end_row();
}

impl App {
    /// Creates or removes the tray icon to match the settings (and the language).
    fn sync_tray(&mut self, ctx: &egui::Context, wanted: bool) {
        let current_lang = self.tray.as_ref().map(|(_, lang)| *lang);
        if !wanted {
            self.tray = None;
            self.tray_error = None;
            return;
        }
        if current_lang == Some(self.lang) || self.tray_error.is_some() {
            return;
        }
        self.tray = None; // the old icon must go before the new one appears
        let wake = ctx.clone();
        match Tray::new(self.texts(), move || wake.request_repaint()) {
            Ok(icon) => self.tray = Some((icon, self.lang)),
            Err(e) => self.tray_error = Some(format!("{} {e}", self.texts().tray_failed)),
        }
    }

    fn show_window(&mut self, ctx: &egui::Context) {
        self.window = WindowState::Shown;
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
    }

    fn hide_window(&mut self, ctx: &egui::Context) {
        self.window = WindowState::Hidden;
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
    }
}

impl eframe::App for App {
    /// Runs every frame, and also while the window is hidden in the tray.
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let (minimize_to_tray, close_to_tray) = self
            .engine
            .with_state(|s| (s.settings.minimize_to_tray, s.settings.close_to_tray));
        self.sync_tray(ctx, minimize_to_tray || close_to_tray);
        while let Some(request) = self.tray.as_ref().and_then(|(icon, _)| icon.poll()) {
            match request {
                Request::Show => self.show_window(ctx),
                Request::Quit => {
                    self.save_geometry();
                    self.window = WindowState::Quitting;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        }
        self.track_geometry(ctx);
        if self.tray.is_none() && self.window == WindowState::Hidden {
            self.show_window(ctx); // never leave the window hidden without an icon
        }
        let (close_requested, minimized) = ctx.input(|i| {
            let viewport = i.viewport();
            (viewport.close_requested(), viewport.minimized.unwrap_or(false))
        });
        let action = tray::window_action(WindowInput {
            tray: self.tray.is_some(),
            minimize_to_tray,
            close_to_tray,
            close_requested,
            minimized,
            hidden: self.window == WindowState::Hidden,
            quitting: self.window == WindowState::Quitting,
        });
        match action {
            WindowAction::None if close_requested && self.window != WindowState::Quitting => {
                self.save_geometry();
            }
            WindowAction::None => {}
            WindowAction::HideInsteadOfClosing => {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.hide_window(ctx);
            }
            WindowAction::HideMinimized => self.hide_window(ctx),
        }
    }

    fn ui(&mut self, root: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = root.ctx().clone();
        let snap = self.engine.with_state(Snapshot::take);
        let dark = ctx.global_style().visuals.dark_mode;
        ctx.request_repaint_after(Duration::from_secs(1));
        self.shortcuts(&ctx, &snap);
        self.collect_toasts(&snap);

        egui::Panel::top("toolbar").show(root, |ui| self.toolbar(ui, &snap));
        if self.load_issue.is_some() || self.warning.is_some() {
            egui::Panel::top("load_issue").show(root, |ui| self.load_issue_banner(ui, dark));
        }
        egui::Panel::bottom("log")
            .resizable(true)
            .default_size(170.0)
            .min_size(80.0)
            .show(root, |ui| {
                self.log_panel(ui, &snap, dark);
            });
        egui::CentralPanel::default().show(root, |ui| {
            let t = self.texts();
            if snap.settings.watch_path().is_none() {
                ui.vertical_centered(|ui| {
                    ui.add_space(ui.available_height() / 3.0);
                    ui.heading(t.onboarding_title);
                    ui.add_space(4.0);
                    ui.label(t.onboarding_body);
                    ui.add_space(12.0);
                    if ui.button(RichText::new(t.choose_folder).size(16.0)).clicked()
                        && let Some(dir) = self.pick_watch_dir(&snap.settings)
                    {
                        self.set_watch_dir(&ctx, &snap.settings, &dir);
                    }
                });
            } else if snap.jobs.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(ui.available_height() / 3.0);
                    ui.weak(if snap.running {
                        t.empty_running
                    } else {
                        t.empty_paused
                    });
                });
            } else {
                self.jobs_table(ui, &snap, dark);
            }
        });
        self.settings_window(&ctx, dark);
        self.show_toasts(&ctx, dark);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Relative luminance (WCAG 2.2).
    fn luminance(c: Color32) -> f64 {
        let channel = |v: u8| {
            let v = f64::from(v) / 255.0;
            if v <= 0.040_45 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * channel(c.r()) + 0.7152 * channel(c.g()) + 0.0722 * channel(c.b())
    }

    /// Premultiplied "source over destination" compositing.
    fn over(src: Color32, dst: Color32) -> Color32 {
        let keep = 255 - u16::from(src.a());
        let mix = |s: u8, d: u8| {
            let v = u16::from(s) + u16::from(d) * keep / 255;
            u8::try_from(v.min(255)).unwrap()
        };
        Color32::from_rgb(
            mix(src.r(), dst.r()),
            mix(src.g(), dst.g()),
            mix(src.b(), dst.b()),
        )
    }

    fn contrast(a: Color32, b: Color32) -> f64 {
        let (la, lb) = (luminance(a), luminance(b));
        (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
    }

    const ALL: [Status; 10] = [
        Status::Waiting,
        Status::Queued,
        Status::Verifying,
        Status::Repairing,
        Status::Extracting,
        Status::NeedPassword,
        Status::RepairImpossible,
        Status::Done,
        Status::Failed,
        Status::Skipped,
    ];

    /// Every status colour (also used for errors and warnings) reaches the WCAG 2.2
    /// AA ratio for normal text, 4.5:1, on every background it is drawn on.
    #[test]
    fn status_colours_meet_wcag_aa_in_both_themes() {
        for (visuals, dark) in [(egui::Visuals::light(), false), (egui::Visuals::dark(), true)] {
            // Striped rows draw `faint_bg_color` (premultiplied, possibly additive) over the panel.
            let stripe = over(visuals.faint_bg_color, visuals.panel_fill);
            // A selected row uses `selection_fill`; notices use `window_fill`.
            let backgrounds = [
                visuals.panel_fill,
                visuals.window_fill,
                stripe,
                selection_fill(dark),
            ];
            for status in ALL {
                for bg in backgrounds {
                    let ratio = contrast(status_color(status, dark), bg);
                    assert!(
                        ratio >= 4.5,
                        "{status:?} (dark: {dark}) on {bg:?}: contrast {ratio:.2} < 4.5"
                    );
                }
            }
        }
    }

    #[test]
    fn contrast_formula_matches_known_values() {
        assert!((contrast(Color32::BLACK, Color32::WHITE) - 21.0).abs() < 1e-9);
        assert!((contrast(Color32::WHITE, Color32::WHITE) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn draft_problems_block_saving() {
        let t = Lang::En.texts();
        let dir = tempfile::tempdir().unwrap();
        let ok = Settings {
            watch_dir: dir.path().display().to_string(),
            extract_dir: dir.path().join("new").display().to_string(), // created later
            ..Settings::default()
        };
        assert_eq!(draft_problems(&ok, t), Vec::<&str>::new());

        let file = dir.path().join("file.txt");
        std::fs::write(&file, b"x").unwrap();
        let bad = Settings {
            watch_dir: dir.path().join("missing").display().to_string(),
            extract_dir: file.display().to_string(),
            par2_path: dir.path().join("nope.exe").display().to_string(),
            ..Settings::default()
        };
        assert_eq!(
            draft_problems(&bad, t),
            vec![t.problem_watch_dir, t.problem_dest_not_dir, t.problem_par2]
        );
    }
}
