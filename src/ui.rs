//! Desktop interface (egui).

use crate::engine::{Command, Engine, EngineState, Job, LogLine, Status};
use crate::i18n::{Lang, Localize, Texts};
use crate::settings::{
    LanguagePref, LoadIssue, SCAN_INTERVAL_RANGE, STABLE_RANGE, Settings, Store, ThemePref,
};
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
        self.passwords_text = settings.passwords.join("\n");
        self.draft = Some(settings.clone());
        self.settings_error = None;
    }

    /// Saves the settings, then applies them. Nothing is applied when saving fails,
    /// so what the user sees always matches the file. Returns the localised error.
    fn apply_settings(&mut self, ctx: &egui::Context, settings: Settings) -> Result<(), String> {
        let settings = settings.validated();
        self.store
            .save_settings(&settings)
            .map_err(|e| self.lang.save_failed(&e.to_string()))?;
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
            if ui.button(t.settings).clicked() {
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
        let Some(issue) = &self.load_issue else { return };
        let text = issue.localize(self.lang);
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
        if dismiss {
            self.load_issue = None;
        }
    }

    fn log_panel(&mut self, ui: &mut egui::Ui, snap: &Snapshot, dark: bool) {
        let t = self.texts();
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.strong(t.log);
            ui.checkbox(&mut self.errors_only, t.errors_only);
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

    fn jobs_table(&mut self, ui: &mut egui::Ui, snap: &Snapshot, dark: bool) {
        let t = self.texts();
        let lang = self.lang;
        let mut actions: Vec<Command> = Vec::new();
        TableBuilder::new(ui)
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
        let (mut save, mut cancel) = (false, false);
        egui::Window::new(t.settings_title)
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(560.0)
            .show(ctx, |ui| {
                egui::Grid::new("settings_grid")
                    .num_columns(2)
                    .spacing([12.0, 8.0])
                    .show(ui, |ui| settings_grid(ui, &mut draft, t, lang, dark));
                ui.add_space(6.0);
                settings_toggles(ui, &mut draft, t);
                ui.add_space(6.0);
                ui.label(t.passwords);
                ui.add(
                    egui::TextEdit::multiline(&mut self.passwords_text)
                        .desired_rows(3)
                        .desired_width(f32::INFINITY),
                );
                ui.weak(t.passwords_hint);
                if let Some(e) = &self.settings_error {
                    ui.colored_label(status_color(Status::Failed, dark), e);
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    save = ui.button(t.save).clicked();
                    cancel = ui.button(t.cancel).clicked();
                });
            });
        if save {
            draft.passwords = self
                .passwords_text
                .lines()
                .map(str::trim)
                .map(str::to_owned)
                .collect();
            if let Err(e) = self.apply_settings(ctx, draft.clone()) {
                self.settings_error = Some(e);
                self.draft = Some(draft);
            }
        } else if open && !cancel {
            self.draft = Some(draft);
        }
    }
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

/// Check boxes of the settings window.
fn settings_toggles(ui: &mut egui::Ui, draft: &mut Settings, t: &Texts) {
    ui.checkbox(&mut draft.recursive, t.recursive);
    ui.checkbox(&mut draft.create_subfolder, t.subfolder);
    ui.checkbox(&mut draft.extract_nested, t.nested);
    ui.checkbox(&mut draft.overwrite_existing, t.overwrite);
    ui.checkbox(&mut draft.delete_after_success, t.trash_after);
    ui.checkbox(&mut draft.autostart, t.autostart);
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

impl eframe::App for App {
    fn ui(&mut self, root: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = root.ctx().clone();
        let snap = self.engine.with_state(Snapshot::take);
        let dark = ctx.global_style().visuals.dark_mode;
        ctx.request_repaint_after(Duration::from_secs(1));

        egui::Panel::top("toolbar").show(root, |ui| self.toolbar(ui, &snap));
        if self.load_issue.is_some() {
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
    }
}
