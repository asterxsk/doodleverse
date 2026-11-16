//! Doodleverse's native UI on Rune, built beside the eframe app while the two reach parity.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod assets;
mod canvas;
mod cards;
mod icons;
mod placement;
mod theme;
mod view;

use app::DoodleApp;
use rune::{App, AppSpec, WindowOptions};

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn"))
        .filter_module("doodleverse_rune", log::LevelFilter::Info)
        .init();

    if let Err(error) = rune::set_app_icon(include_bytes!("../../../assets/app-icon.png").to_vec())
    {
        log::warn!("app icon rejected: {error}");
    }

    let mut spec = AppSpec::new(
        "Doodleverse",
        (app::INITIAL_WINDOW_SIZE[0], app::INITIAL_WINDOW_SIZE[1]),
        DoodleApp::new,
        view::view,
    );
    spec.min_size = Some((880.0, 600.0));
    spec.poll = Some(app::poll);
    spec.on_key = Some(app::key_event);

    let application = App::new(spec)
        .on_close(app::on_close)
        .on_modifiers(app::on_modifiers)
        .window_options(window_options());
    let handle = application.handle();
    application
        .init(move || {
            let mut state = DoodleApp::new();
            state.handle = Some(handle);
            state
        })
        .keep_running()
        .on_window(|window| {
            let installed = window.with_raw_handle(doodle_platform::install_pen_input);
            match installed {
                Ok(true) => log::info!("pen pressure input installed"),
                Ok(false) => {
                    log::info!("pen pressure unavailable; mouse pressure simulation in use")
                }
                Err(error) => log::warn!("window handle unavailable for pen input: {error}"),
            }
        })
        .run();
}

/// `DOODLEVERSE_WINDOW_POS=x,y` opens the window at those physical desktop pixels. It exists so a
/// developer can put a build on a second monitor without moving it by hand each launch.
fn window_options() -> WindowOptions {
    let mut options = WindowOptions::default();
    if let Ok(value) = std::env::var("DOODLEVERSE_WINDOW_POS")
        && let Some((x, y)) = value.split_once(',')
        && let (Ok(x), Ok(y)) = (x.trim().parse::<i32>(), y.trim().parse::<i32>())
    {
        options.position = Some((x, y));
    }
    options
}
