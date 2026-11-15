#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;

use eframe::egui;

fn main() -> eframe::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1280.0, 820.0])
        .with_min_inner_size([760.0, 520.0])
        .with_title("Doodleverse");
    if let Some(icon) = app::assets::window_icon() {
        viewport = viewport.with_icon(std::sync::Arc::new(icon));
    }
    if let Some((x, y, width, height)) = doodle_platform::test_window_placement() {
        viewport = viewport
            .with_position([x, y])
            .with_inner_size([width, height]);
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    let creator: eframe::AppCreator<'_> =
        Box::new(|cc| Ok(Box::new(app::DoodleApp::new(cc)) as Box<dyn eframe::App>));
    eframe::run_native("Doodleverse", options, creator)
}
