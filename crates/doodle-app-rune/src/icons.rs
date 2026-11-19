//! Material Symbols Rounded (filled) icons, drawn by Rune's SVG icon engine.
//!
//! Path data is vendored from `google/material-design-icons` at commit
//! `10ecc14790612ca8fd164e3f4ffe9c16fc50d5b7` (Apache-2.0; license text in
//! `assets/icons/LICENSE-material-symbols.txt`). The set uses a 960-unit grid with negative Y;
//! each body translates the glyph into Rune's `0 0 view view` box, so no path data is rewritten.
//! The selection record is `docs/plans/icon-selection.md`.

use rune::{Color, El, Icon};

const fn filled(name: &'static str, path: &'static str) -> Icon {
    Icon {
        name,
        view: 960.0,
        body: path,
    }
}

// Tool glyphs.
static HAND: Icon = filled(
    "hand",
    "<path transform=\"translate(0 960)\" d=\"M402-40q-30 0-56-13.5T303-92L67-438q-8-12-7-26t12-24q19-19 45-22t47 12l116 81v-383q0-17 11.5-28.5T320-840q17 0 28.5 11.5T360-800v320h80v-400q0-17 11.5-28.5T480-920q17 0 28.5 11.5T520-880v400h80v-360q0-17 11.5-28.5T640-880q17 0 28.5 11.5T680-840v360h80v-280q0-17 11.5-28.5T800-800q17 0 28.5 11.5T840-760v560q0 66-47 113T680-40H402Z\"/>",
);
static POINTER: Icon = filled(
    "pointer",
    "<path transform=\"translate(0 960)\" d=\"M606-105q-23 11-46 2.5T526-134L406-392l-93 130q-17 24-45 15t-28-38v-513q0-25 22.5-36t42.5 5l404 318q23 17 13.5 44T684-440H516l119 255q11 23 2.5 46T606-105Z\"/>",
);
static RECTANGLE: Icon = filled(
    "rectangle",
    "<path transform=\"translate(0 960)\" d=\"M160-160q-33 0-56.5-23.5T80-240v-480q0-33 23.5-56.5T160-800h640q33 0 56.5 23.5T880-720v480q0 33-23.5 56.5T800-160H160Z\"/>",
);
// The diamond tool draws a rhombus, so its glyph is the set's square turned 45° and scaled to
// fit the 960 box; the glyph named "diamond" is a faceted gem and read as a kite at 16pt.
static DIAMOND: Icon = filled(
    "diamond",
    "<path transform=\"rotate(45 480 480) translate(480 480) scale(0.85) translate(-480 -480) translate(0 960)\" d=\"M200-120q-33 0-56.5-23.5T120-200v-560q0-33 23.5-56.5T200-840h560q33 0 56.5 23.5T840-760v560q0 33-23.5 56.5T760-120H200Z\"/>",
);
static CIRCLE: Icon = filled(
    "circle",
    "<path transform=\"translate(0 960)\" d=\"M480-80q-83 0-156-31.5T197-197q-54-54-85.5-127T80-480q0-83 31.5-156T197-763q54-54 127-85.5T480-880q83 0 156 31.5T763-763q54 54 85.5 127T880-480q0 83-31.5 156T763-197q-54 54-127 85.5T480-80Z\"/>",
);
static ARROW: Icon = filled(
    "arrow",
    "<path transform=\"translate(0 960)\" d=\"M680-624 244-188q-11 11-28 11t-28-11q-11-11-11-28t11-28l436-436H400q-17 0-28.5-11.5T360-720q0-17 11.5-28.5T400-760h320q17 0 28.5 11.5T760-720v320q0 17-11.5 28.5T720-360q-17 0-28.5-11.5T680-400v-224Z\"/>",
);
static LINE: Icon = filled(
    "line",
    "<path transform=\"translate(0 960)\" d=\"M180-380q-42 0-71-29t-29-71q0-42 29-71t71-29q31 0 56 17t36 43h568q17 0 28.5 11.5T880-480q0 17-11.5 28.5T840-440H272q-11 26-36 43t-56 17Z\"/>",
);
// A plain fountain-pen nib: the first two subpaths of the set's `ink_pen` glyph, with its
// underline and squiggle dropped. The full glyph read as "pen with decorative flourish"; the
// toolbar and cursor want the tool itself.
static PEN: Icon = filled(
    "pen",
    "<path transform=\"translate(0 960)\" d=\"M200-120q-17 0-28.5-11.5T160-160v-97q0-16 6-30.5t17-25.5l504-503q12-12 27-18t30-6q16 0 30.5 6t25.5 18l56 56q12 11 18 25.5t6 30.5q0 15-6 30t-18 27L353-143q-11 11-25.5 17t-30.5 6h-97Zm546-529 54-54-57-57-54 54 57 57Z\"/>",
);
static TEXT: Icon = filled(
    "text",
    "<path transform=\"translate(0 960)\" d=\"M420-680H260q-25 0-42.5-17.5T200-740q0-25 17.5-42.5T260-800h440q25 0 42.5 17.5T760-740q0 25-17.5 42.5T700-680H540v460q0 25-17.5 42.5T480-160q-25 0-42.5-17.5T420-220v-460Z\"/>",
);
static STICKY: Icon = filled(
    "sticky",
    "<path transform=\"translate(0 960)\" d=\"m560-200 200-200H600q-17 0-28.5 11.5T560-360v160Zm-360 80q-33 0-56.5-23.5T120-200v-560q0-33 23.5-56.5T200-840h560q33 0 56.5 23.5T840-760v367q0 16-6 30.5T817-337L623-143q-11 11-25.5 17t-30.5 6H200Zm120-280h120q17 0 28.5-11.5T480-440q0-17-11.5-28.5T440-480H320q-17 0-28.5 11.5T280-440q0 17 11.5 28.5T320-400Zm0-160h320q17 0 28.5-11.5T680-600q0-17-11.5-28.5T640-640H320q-17 0-28.5 11.5T280-600q0 17 11.5 28.5T320-560Z\"/>",
);
static ERASER: Icon = filled(
    "eraser",
    "<path transform=\"translate(0 960)\" d=\"M690-240h150q17 0 28.5 11.5T880-200q0 17-11.5 28.5T840-160H610l80-80Zm-483 80q-8 0-15.5-3t-13.5-9l-73-73q-23-23-23.5-57t22.5-58l440-456q23-24 56.5-24t56.5 23l199 199q23 23 23 57t-23 57L532-172q-6 6-13.5 9t-15.5 3H207Z\"/>",
);

// Actions and chrome.
static LOCK: Icon = filled(
    "lock",
    "<path transform=\"translate(0 960)\" d=\"M240-80q-33 0-56.5-23.5T160-160v-400q0-33 23.5-56.5T240-640h40v-80q0-83 58.5-141.5T480-920q83 0 141.5 58.5T680-720v80h40q33 0 56.5 23.5T800-560v400q0 33-23.5 56.5T720-80H240Zm240-200q33 0 56.5-23.5T560-360q0-33-23.5-56.5T480-440q-33 0-56.5 23.5T400-360q0 33 23.5 56.5T480-280ZM360-640h240v-80q0-50-35-85t-85-35q-50 0-85 35t-35 85v80Z\"/>",
);
static MENU: Icon = filled(
    "menu",
    "<path transform=\"translate(0 960)\" d=\"M160-240q-17 0-28.5-11.5T120-280q0-17 11.5-28.5T160-320h640q17 0 28.5 11.5T840-280q0 17-11.5 28.5T800-240H160Zm0-200q-17 0-28.5-11.5T120-480q0-17 11.5-28.5T160-520h640q17 0 28.5 11.5T840-480q0 17-11.5 28.5T800-440H160Zm0-200q-17 0-28.5-11.5T120-680q0-17 11.5-28.5T160-720h640q17 0 28.5 11.5T840-680q0 17-11.5 28.5T800-640H160Z\"/>",
);
static MORE: Icon = filled(
    "more",
    "<path transform=\"translate(0 960)\" d=\"M240-400q-33 0-56.5-23.5T160-480q0-33 23.5-56.5T240-560q33 0 56.5 23.5T320-480q0 33-23.5 56.5T240-400Zm240 0q-33 0-56.5-23.5T400-480q0-33 23.5-56.5T480-560q33 0 56.5 23.5T560-480q0 33-23.5 56.5T480-400Zm240 0q-33 0-56.5-23.5T640-480q0-33 23.5-56.5T720-560q33 0 56.5 23.5T800-480q0 33-23.5 56.5T720-400Z\"/>",
);
static UNDO: Icon = filled(
    "undo",
    "<path transform=\"translate(0 960)\" d=\"M320-200q-17 0-28.5-11.5T280-240q0-17 11.5-28.5T320-280h244q63 0 109.5-40T720-420q0-60-46.5-100T564-560H312l76 76q11 11 11 28t-11 28q-11 11-28 11t-28-11L188-572q-6-6-8.5-13t-2.5-15q0-8 2.5-15t8.5-13l144-144q11-11 28-11t28 11q11 11 11 28t-11 28l-76 76h252q97 0 166.5 63T800-420q0 94-69.5 157T564-200H320Z\"/>",
);
static REDO: Icon = filled(
    "redo",
    "<path transform=\"translate(0 960)\" d=\"M648-560H396q-63 0-109.5 40T240-420q0 60 46.5 100T396-280h244q17 0 28.5 11.5T680-240q0 17-11.5 28.5T640-200H396q-97 0-166.5-63T160-420q0-94 69.5-157T396-640h252l-76-76q-11-11-11-28t11-28q11-11 28-11t28 11l144 144q6 6 8.5 13t2.5 15q0 8-2.5 15t-8.5 13L628-428q-11 11-28 11t-28-11q-11-11-11-28t11-28l76-76Z\"/>",
);
static SETTINGS: Icon = filled(
    "settings",
    "<path transform=\"translate(0 960)\" d=\"M433-80q-27 0-46.5-18T363-142l-9-66q-13-5-24.5-12T307-235l-62 26q-25 11-50 2t-39-32l-47-82q-14-23-8-49t27-43l53-40q-1-7-1-13.5v-27q0-6.5 1-13.5l-53-40q-21-17-27-43t8-49l47-82q14-23 39-32t50 2l62 26q11-8 23-15t24-12l9-66q4-26 23.5-44t46.5-18h94q27 0 46.5 18t23.5 44l9 66q13 5 24.5 12t22.5 15l62-26q25-11 50-2t39 32l47 82q14 23 8 49t-27 43l-53 40q1 7 1 13.5v27q0 6.5-2 13.5l53 40q21 17 27 43t-8 49l-48 82q-14 23-39 32t-50-2l-60-26q-11 8-23 15t-24 12l-9 66q-4 26-23.5 44T527-80h-94Zm49-260q58 0 99-41t41-99q0-58-41-99t-99-41q-59 0-99.5 41T342-480q0 58 40.5 99t99.5 41Z\"/>",
);
static MAP: Icon = filled(
    "map",
    "<path transform=\"translate(0 960)\" d=\"m574-129-214-75-186 72q-10 4-19.5 2.5T137-136q-8-5-12.5-13.5T120-169v-561q0-13 7.5-23t20.5-15l186-63q6-2 12.5-3t13.5-1q7 0 13.5 1t12.5 3l214 75 186-72q10-4 19.5-2.5T823-824q8 5 12.5 13.5T840-791v561q0 13-7.5 23T812-192l-186 63q-6 2-12.5 3t-13.5 1q-7 0-13.5-1t-12.5-3Zm-14-89v-468l-160-56v468l160 56Z\"/>",
);
static HELP: Icon = filled(
    "help",
    "<path transform=\"translate(0 960)\" d=\"M478-240q21 0 35.5-14.5T528-290q0-21-14.5-35.5T478-340q-21 0-35.5 14.5T428-290q0 21 14.5 35.5T478-240Zm2 160q-83 0-156-31.5T197-197q-54-54-85.5-127T80-480q0-83 31.5-156T197-763q54-54 127-85.5T480-880q83 0 156 31.5T763-763q54 54 85.5 127T880-480q0 83-31.5 156T763-197q-54 54-127 85.5T480-80Zm4-572q25 0 43.5 16t18.5 40q0 22-13.5 39T502-525q-23 20-40.5 44T444-427q0 14 10.5 23.5T479-394q15 0 25.5-10t13.5-25q4-21 18-37.5t30-31.5q23-22 39.5-48t16.5-58q0-51-41.5-83.5T484-720q-38 0-72.5 16T359-655q-7 12-4.5 25.5T368-609q14 8 29 5t25-17q11-15 27.5-23t34.5-8Z\"/>",
);
static CLOSE: Icon = filled(
    "close",
    "<path transform=\"translate(0 960)\" d=\"M480-424 284-228q-11 11-28 11t-28-11q-11-11-11-28t11-28l196-196-196-196q-11-11-11-28t11-28q11-11 28-11t28 11l196 196 196-196q11-11 28-11t28 11q11 11 11 28t-11 28L536-480l196 196q11 11 11 28t-11 28q-11 11-28 11t-28-11L480-424Z\"/>",
);
static ADD: Icon = filled(
    "add",
    "<path transform=\"translate(0 960)\" d=\"M440-440H240q-17 0-28.5-11.5T200-480q0-17 11.5-28.5T240-520h200v-200q0-17 11.5-28.5T480-760q17 0 28.5 11.5T520-720v200h200q17 0 28.5 11.5T760-480q0 17-11.5 28.5T720-440H520v200q0 17-11.5 28.5T480-200q-17 0-28.5-11.5T440-240v-200Z\"/>",
);
static REMOVE: Icon = filled(
    "remove",
    "<path transform=\"translate(0 960)\" d=\"M240-440q-17 0-28.5-11.5T200-480q0-17 11.5-28.5T240-520h480q17 0 28.5 11.5T760-480q0 17-11.5 28.5T720-440H240Z\"/>",
);
static CHECK: Icon = filled(
    "check",
    "<path transform=\"translate(0 960)\" d=\"m382-354 339-339q12-12 28-12t28 12q12 12 12 28.5T777-636L410-268q-12 12-28 12t-28-12L182-440q-12-12-11.5-28.5T183-497q12-12 28.5-12t28.5 12l142 143Z\"/>",
);
static DOWNLOAD: Icon = filled(
    "download",
    "<path transform=\"translate(0 960)\" d=\"M480-337q-8 0-15-2.5t-13-8.5L308-492q-12-12-11.5-28t11.5-28q12-12 28.5-12.5T365-549l75 75v-286q0-17 11.5-28.5T480-800q17 0 28.5 11.5T520-760v286l75-75q12-12 28.5-11.5T652-548q11 12 11.5 28T652-492L508-348q-6 6-13 8.5t-15 2.5ZM240-160q-33 0-56.5-23.5T160-240v-80q0-17 11.5-28.5T200-360q17 0 28.5 11.5T240-320v80h480v-80q0-17 11.5-28.5T760-360q17 0 28.5 11.5T800-320v80q0 33-23.5 56.5T720-160H240Z\"/>",
);
static SPARKLES: Icon = Icon {
    name: "sparkles",
    view: 24.0,
    body: "<path d=\"M19 8.3q-.125 0-.262-.075Q18.6 8.15 18.55 8l-.8-1.75-1.75-.8q-.15-.05-.225-.188Q15.7 5.125 15.7 5t.075-.263Q15.85 4.6 16 4.55l1.75-.8.8-1.75q.05-.15.188-.225.137-.075.262-.075t.263.075q.137.075.187.225l.8 1.75 1.75.8q.15.05.225.187.075.138.075.263t-.075.262Q22.15 5.4 22 5.45l-1.75.8-.8 1.75q-.05.15-.187.225-.138.075-.263.075Zm0 14q-.125 0-.262-.075-.138-.075-.188-.225l-.8-1.75-1.75-.8q-.15-.05-.225-.188-.075-.137-.075-.262t.075-.262q.075-.138.225-.188l1.75-.8.8-1.75q.05-.15.188-.225.137-.075.262-.075t.263.075q.137.075.187.225l.8 1.75 1.75.8q.15.05.225.188.075.137.075.262t-.075.262q-.075.138-.225.188l-1.75.8-.8 1.75q-.05.15-.187.225-.138.075-.263.075ZM9 18.575q-.275 0-.525-.15T8.1 18l-1.6-3.5L3 12.9q-.275-.125-.425-.375-.15-.25-.15-.525t.15-.525q.15-.25.425-.375l3.5-1.6L8.1 6q.125-.275.375-.425.25-.15.525-.15t.525.15q.25.15.375.425l1.6 3.5 3.5 1.6q.275.125.425.375.15.25.15.525t-.15.525q-.15.25-.425.375l-3.5 1.6L9.9 18q-.125.275-.375.425-.25.15-.525.15Z\"/>",
};

/// The icon named `name` in `ink`, laid out at `size` points square. Panics on an icon that is
/// not vendored, so a typo fails loudly in development instead of drawing an empty box.
pub fn icon(name: &str, size: f32, ink: Color) -> El {
    // The surrounding control carries the accessible name; the glyph itself is decoration.
    rune::icon(named(name), size).color(ink).role("none")
}

fn named(name: &str) -> &'static Icon {
    match name {
        "hand" => &HAND,
        "pointer" => &POINTER,
        "rectangle" => &RECTANGLE,
        "diamond" => &DIAMOND,
        "circle" => &CIRCLE,
        "arrow" => &ARROW,
        "line" => &LINE,
        "pen" => &PEN,
        "text" => &TEXT,
        "sticky" => &STICKY,
        "eraser" => &ERASER,
        "lock" => &LOCK,
        "menu" => &MENU,
        "more" => &MORE,
        "undo" => &UNDO,
        "redo" => &REDO,
        "settings" => &SETTINGS,
        "map" => &MAP,
        "help" => &HELP,
        "close" => &CLOSE,
        "add" => &ADD,
        "remove" => &REMOVE,
        "check" => &CHECK,
        "download" => &DOWNLOAD,
        "sparkles" => &SPARKLES,
        other => panic!("icon {other:?} is not vendored"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXPECTED: &[&str] = &[
        "hand",
        "pointer",
        "rectangle",
        "diamond",
        "circle",
        "arrow",
        "line",
        "pen",
        "text",
        "sticky",
        "eraser",
        "lock",
        "menu",
        "more",
        "undo",
        "redo",
        "settings",
        "map",
        "help",
        "close",
        "add",
        "remove",
        "check",
        "download",
        "sparkles",
    ];

    #[test]
    fn every_expected_icon_renders_a_glyph() {
        for name in EXPECTED {
            let coverage = rune::text::icon_coverage(named(name), 24);
            assert!(
                coverage.iter().any(|&c| c != 0),
                "{name} produced no coverage"
            );
        }
    }

    #[test]
    fn unknown_icons_panic_in_development() {
        let panicked = std::panic::catch_unwind(|| named("not-an-icon"));
        assert!(panicked.is_err());
    }

    /// Repaints `assets/cursors/{pen,sticky-note}.png` from the same Material glyphs the toolbar
    /// draws, so the cursor and its tool button can never drift apart. Run with
    /// `cargo test -p doodle-app-rune regenerate_cursor_pngs -- --ignored`.
    #[test]
    #[ignore = "writes the checked-in cursor assets"]
    fn regenerate_cursor_pngs() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/cursors");
        write_cursor(&dir.join("pen.png"), named("pen"));
        write_cursor(&dir.join("sticky-note.png"), named("sticky"));
    }

    /// 64×64 RGBA at 2x: the glyph at 52px centred in a 6px margin, dark ink over a 2px white
    /// halo so it reads on both the dark board and a pale note.
    fn write_cursor(path: &std::path::Path, icon: &Icon) {
        const BOX: u32 = 64;
        const GLYPH: u32 = 52;
        const OFFSET: u32 = 6;
        let coverage = rune::text::icon_coverage(icon, GLYPH);
        let mut alpha = vec![0u8; (BOX * BOX) as usize];
        for y in 0..GLYPH {
            for x in 0..GLYPH {
                alpha[((y + OFFSET) * BOX + x + OFFSET) as usize] =
                    coverage[(y * GLYPH + x) as usize];
            }
        }
        let mut halo = alpha.clone();
        for y in 0..BOX {
            for x in 0..BOX {
                let mut peak = 0u8;
                for dy in -2i32..=2 {
                    for dx in -2i32..=2 {
                        let nx = x as i32 + dx;
                        let ny = y as i32 + dy;
                        if (0..BOX as i32).contains(&nx) && (0..BOX as i32).contains(&ny) {
                            peak = peak.max(alpha[(ny as u32 * BOX + nx as u32) as usize]);
                        }
                    }
                }
                halo[(y * BOX + x) as usize] = peak;
            }
        }
        let mut image = image::RgbaImage::new(BOX, BOX);
        for y in 0..BOX {
            for x in 0..BOX {
                let cov = f32::from(alpha[(y * BOX + x) as usize]) / 255.0;
                let wht = f32::from(halo[(y * BOX + x) as usize]) / 255.0;
                let out_a = (cov + wht * (1.0 - cov)).clamp(0.0, 1.0);
                if out_a <= 0.0 {
                    continue;
                }
                let ink = 26.0f32;
                let white = 255.0f32;
                let channel = (ink * cov + white * wht * (1.0 - cov)) / out_a;
                let level = channel.clamp(0.0, 255.0) as u8;
                image.put_pixel(
                    x,
                    y,
                    image::Rgba([level, level, level, (out_a * 255.0) as u8]),
                );
            }
        }
        image.save(path).expect("cursor png writes");
    }
}
