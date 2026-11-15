//! Native platform integration.
//!
//! Board files use the OS dialog. No board content, preferences, or credentials are persisted by
//! this crate. The OS credential-vault adapter is intentionally absent from this milestone: no
//! credential is stored rather than falling back to plaintext.

use std::path::PathBuf;

pub mod pen;
pub use pen::{install_pen_input, pen_available, pen_pressure};

pub const BOARD_EXTENSION: &str = "doodleverse";

/// Opens the native file picker. Returns `None` on cancel or an unavailable dialog.
pub fn open_board_dialog() -> Option<PathBuf> {
    let result = rfd::FileDialog::new()
        .add_filter("Doodleverse board", &[BOARD_EXTENSION])
        .set_title("Open Doodleverse board")
        .pick_file();
    log::info!("open dialog finished: {}", result.is_some());
    result
}

/// Opens the native save picker with a suggested filename.
pub fn save_board_dialog(suggested_name: &str) -> Option<PathBuf> {
    let mut name = suggested_name.trim();
    if name.is_empty() {
        name = "doodleverse-board";
    }
    let filename = format!("{name}.{BOARD_EXTENSION}");
    let result = rfd::FileDialog::new()
        .add_filter("Doodleverse board", &[BOARD_EXTENSION])
        .set_file_name(filename)
        .set_title("Export Doodleverse board")
        .save_file();
    log::info!("save dialog finished: {}", result.is_some());
    result
}

/// Test-only window placement from `DOODLE_TEST_WINDOW="x,y,w,h"`, so unattended runs never take
/// over the primary display.
#[must_use]
pub fn test_window_placement() -> Option<(f32, f32, f32, f32)> {
    let raw = std::env::var("DOODLE_TEST_WINDOW").ok()?;
    let parts: Vec<f32> = raw
        .split(',')
        .map(|part| part.trim().parse::<f32>())
        .collect::<Result<_, _>>()
        .ok()?;
    if parts.len() != 4 || parts.iter().any(|value| !value.is_finite()) {
        return None;
    }
    Some((parts[0], parts[1], parts[2], parts[3]))
}

/// Normalizes a path chosen by the user so an extension is present without double-appending it.
#[must_use]
pub fn ensure_board_extension(path: PathBuf) -> PathBuf {
    if path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case(BOARD_EXTENSION))
    {
        path
    } else {
        path.with_extension(BOARD_EXTENSION)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extension_is_added_once() {
        assert_eq!(
            ensure_board_extension(PathBuf::from("C:/tmp/board")),
            PathBuf::from("C:/tmp/board.doodleverse")
        );
        assert_eq!(
            ensure_board_extension(PathBuf::from("C:/tmp/board.DOODLEVERSE")),
            PathBuf::from("C:/tmp/board.DOODLEVERSE")
        );
    }
}
