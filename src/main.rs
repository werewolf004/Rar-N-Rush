//! Rar N Rush desktop application entry point.

// No console window in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use rar_n_rush::engine::Engine;
use rar_n_rush::settings::Store;
use rar_n_rush::ui::App;

fn main() -> eframe::Result {
    let store = Store::locate();
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
            Ok(Box::new(App::new(&cc.egui_ctx, engine, store, loaded.issue)))
        }),
    )
}
