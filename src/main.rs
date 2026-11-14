//! Rar N Rush desktop application entry point.

// No console window in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use rar_n_rush::autostart::{self, Autostart};
use rar_n_rush::engine::Engine;
use rar_n_rush::instance::{self, Instance};
use rar_n_rush::notify::WindowsNotifier;
use rar_n_rush::secrets::ChunkedVault;
use rar_n_rush::settings::Store;
use rar_n_rush::ui::App;
use std::sync::Arc;

fn main() -> eframe::Result {
    // Archive passwords live in the Windows Credential Manager. If it cannot be
    // opened, they stay in the settings file, as before.
    let store = match ChunkedVault::windows() {
        Ok(vault) => Store::locate().with_vault(Arc::new(vault)),
        Err(_) => Store::locate(),
    };
    // One instance per settings folder: a second start shows the running window
    // and exits. If the lock cannot be taken at all, start anyway rather than
    // refuse to run.
    let guard = match instance::acquire(store.dir()) {
        Ok(Instance::First(guard)) => Some(guard),
        Ok(Instance::AlreadyRunning) => {
            let _ = instance::request_show(store.dir());
            return Ok(());
        }
        Err(_) => None,
    };
    // Never fails: invalid values are reset one by one, the original file is backed
    // up, and the problem is shown to the user in a banner.
    let loaded = store.load();

    let mut viewport = egui::ViewportBuilder::default()
        .with_title("Rar N Rush")
        .with_inner_size([1100.0, 680.0])
        .with_min_inner_size([720.0, 420.0])
        .with_icon(rar_n_rush::icon::window_icon());
    // Values are clamped when the settings are loaded; Windows keeps the window
    // reachable if the monitor it was on is gone.
    if let Some(g) = loaded.settings.window {
        viewport = viewport
            .with_inner_size(g.size())
            .with_position(g.position())
            .with_maximized(g.maximized);
    }
    // Started by Windows at sign-in: created invisible, then minimised or hidden
    // in the notification area on the first frame (see `App::started_in_background`).
    let background = autostart::started_by_windows(std::env::args_os());
    if background {
        viewport = viewport.with_visible(false);
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        "Rar N Rush",
        options,
        Box::new(move |cc| {
            let ctx = cc.egui_ctx.clone();
            let engine = Engine::start(loaded.settings, store.clone(), move || ctx.request_repaint())?;
            let notifier = WindowsNotifier::new(store.dir());
            let mut app = App::new(&cc.egui_ctx, engine, store, loaded.issue).with_notifier(notifier);
            if let Ok(exe) = std::env::current_exe() {
                app = app.with_autostart(Autostart::current_user(&exe));
            }
            if background {
                app = app.started_in_background();
            }
            if let Some(guard) = guard {
                let ctx = cc.egui_ctx.clone();
                app = app.with_show_requests(guard.watch(move || ctx.request_repaint()));
            }
            Ok(Box::new(app))
        }),
    )
}
