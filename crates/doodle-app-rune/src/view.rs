use doodle_render::GridKind;
use rune::{Cx, El, ScrollSpec, col, el, row, text};

use crate::app::{DoodleApp, NoticeKind, Target, Tool};
use crate::theme::{self, Palette};

/// Default ink follows the board so a new stroke stays readable on a dark canvas.
fn ink_rgba(light: bool) -> [u8; 4] {
    if light {
        [32, 30, 35, 255]
    } else {
        [238, 234, 227, 255]
    }
}

fn swatches(light: bool) -> [(&'static str, [u8; 4]); 6] {
    [
        ("Ink", ink_rgba(light)),
        ("Coral", [222, 108, 85, 255]),
        ("Teal", [55, 145, 141, 255]),
        ("Gold", [214, 164, 52, 255]),
        ("Sky", [86, 132, 196, 255]),
        ("Plum", [136, 86, 150, 255]),
    ]
}

/// Width below which Ask Dibby / Export move into the overflow menu. The centred toolbar is
/// ~600pt wide and the left island ends ~260pt from the edge, so anything under ~1140 makes the
/// toolbar card paint over the widened menu island. Expanded only well clear of that point.
const MENU_BAR_EXPAND_WIDTH: f32 = 1160.0;

const BOARD_PRESETS: [(&str, [u8; 4], [u8; 4], bool); 6] = [
    ("Paper", [250, 248, 243, 255], [174, 168, 158, 255], true),
    ("Snow", [255, 255, 255, 255], [203, 207, 214, 255], true),
    ("Sand", [244, 235, 214, 255], [174, 158, 127, 255], true),
    ("Mist", [232, 239, 244, 255], [148, 164, 176, 255], true),
    ("Slate", [43, 45, 51, 255], [94, 99, 112, 255], false),
    ("Ink", [24, 23, 28, 255], [205, 201, 210, 255], false),
];

/// Any press on the chrome dismisses an open popover, matching what the scrim does.
fn dismiss_popovers(s: &mut DoodleApp) {
    s.menu_open = false;
    s.overflow_open = false;
}

pub fn view(s: &DoodleApp, _cx: &mut Cx<DoodleApp>) -> El {
    let pal = theme::palette(s.light_theme);
    let mut root = col()
        .key("root")
        .w_full()
        .h_full()
        .fill(pal.bg)
        .on_drawn_layout(|s: &mut DoodleApp, rect: [f32; 4]| {
            s.window_size = [rect[2], rect[3]];
        });

    root = root.child(crate::canvas::canvas_layer(s, &pal));
    for label in crate::canvas::statement_texts(s, &pal) {
        root = root.child(label);
    }

    if s.projection.objects.is_empty() && s.hint_visible {
        root = root.child(empty_hint(s, &pal));
    }

    for card in crate::cards::card_views(s, &pal) {
        root = root.child(card);
    }

    if s.menu_open {
        root = root.child(menu_scrim());
    }

    if s.overflow_open {
        root = root.child(overflow_scrim());
    }

    if s.zen {
        root = root.child(zen_hint(&pal));
    } else {
        root = root.child(menu_island(s, &pal));
        root = root.child(toolbar_island(s, &pal));
        root = root.child(overflow_island(s, &pal));
        root = root.child(status_island(s, &pal));
        root = root.child(zoom_island(s, &pal));
    }

    // Popovers sit above the islands so a menu opened near one is never clipped by it.
    if s.menu_open {
        root = root.child(menu_popover(&pal));
    }
    if s.overflow_open {
        root = root.child(overflow_popover(s, &pal));
    }

    if s.show_settings {
        root = root.child(settings_panel(s, &pal));
    }
    if s.show_help {
        root = root.child(help_panel(&pal));
    }
    if s.show_close_confirm {
        root = root.child(centered(&pal, 420.0, close_body(&pal)));
    }
    if s.pin_preview.is_some() {
        root = root.child(centered(&pal, 440.0, pin_body(s, &pal)));
    }
    root
}

fn empty_hint(s: &DoodleApp, pal: &Palette) -> El {
    let top = (s.window_size[1] * 0.42).max(80.0);
    col()
        .absolute(0.0, top)
        .w_full()
        .items_center()
        .gap(6.0)
        .hit()
        .child(
            text("Draw anywhere")
                .font(theme::heading(26.0))
                .color(pal.muted),
        )
        .child(
            text("B brush · V select · H hand · wheel pans · Ctrl+wheel zooms")
                .font(theme::body())
                .color(pal.muted),
        )
        .child(
            text("Ask Dibby for ideas")
                .font(theme::caption())
                .color(pal.muted),
        )
}

fn island(pal: &Palette) -> El {
    theme::island(pal)
}

fn menu_island(s: &DoodleApp, pal: &Palette) -> El {
    let mut menu_button =
        theme::flat_button("menu-toggle", "Doodleverse", pal).on_press(|s: &mut DoodleApp| {
            s.overflow_open = false;
            s.menu_open = !s.menu_open;
        });
    if s.menu_open {
        menu_button = menu_button.fill(pal.hover);
    }
    let mut content = row().gap(6.0).items_center().child(menu_button);
    // Narrow windows leave the centred toolbar room to pass between the two corner islands.
    if s.window_size[0] >= MENU_BAR_EXPAND_WIDTH {
        content = content
            .child(theme::flat_button("ask-dibby", "Ask Dibby", pal).on_press(
                |s: &mut DoodleApp| {
                    dismiss_popovers(s);
                    s.create_ai_card();
                },
            ))
            .child(theme::flat_button("export", "Export", pal).on_press_tx(
                |s: &mut DoodleApp, tx| {
                    dismiss_popovers(s);
                    s.export_file(tx);
                },
            ));
    }
    island(pal).absolute(12.0, 10.0).child(content)
}

fn menu_scrim() -> El {
    el().absolute(0.0, 0.0)
        .w_full()
        .h_full()
        .on_press(|s: &mut DoodleApp| {
            s.menu_open = false;
            s.overflow_open = false;
        })
}

fn overflow_scrim() -> El {
    el().absolute(0.0, 0.0)
        .w_full()
        .h_full()
        .on_press(|s: &mut DoodleApp| {
            s.menu_open = false;
            s.overflow_open = false;
        })
}

fn menu_popover(pal: &Palette) -> El {
    let mut items = col().gap(2.0).w_full();
    for (label, action) in [("Open board…", 0u8), ("Export board…", 1)] {
        items = items.child(
            theme::flat_button(format!("menu-{action}"), label, pal)
                .w_full()
                .on_press_tx(move |s: &mut DoodleApp, tx| match action {
                    0 => s.open_file(tx),
                    _ => s.export_file(tx),
                }),
        );
    }
    items = items.child(theme::divider_h(pal));
    items = items.child(
        theme::flat_button("menu-settings", "Canvas settings…", pal)
            .w_full()
            .on_press(|s: &mut DoodleApp| {
                s.show_settings = true;
                s.menu_open = false;
            }),
    );
    island(pal)
        .absolute(12.0, 64.0)
        .w(228.0)
        .padding(9.0)
        .child(items)
}

fn toolbar_island(s: &DoodleApp, pal: &Palette) -> El {
    let mut bar = row().gap(6.0).items_center();
    for (tool, label) in [
        (Tool::Draw, "Brush"),
        (Tool::Select, "Select"),
        (Tool::Hand, "Hand"),
    ] {
        bar = bar.child(
            theme::active_button(format!("tool-{:?}", tool), label, s.tool == tool, pal)
                .w(62.0)
                .on_press(move |s: &mut DoodleApp| {
                    dismiss_popovers(s);
                    s.tool = tool;
                    if tool != Tool::Draw {
                        s.stroke = None;
                    }
                }),
        );
    }
    bar = bar.child(theme::divider_v(pal));
    bar = bar.child(
        text(format!("{:.0} px", s.brush.base_width))
            .font(theme::caption())
            .color(pal.muted),
    );
    bar = bar.child(slider(
        pal,
        "brush-size",
        "",
        60.0,
        s.slider_fraction(Target::BrushSize),
        Target::BrushSize,
    ));
    bar = bar.child(theme::divider_v(pal));
    let mut colors = row().gap(4.0).items_center();
    for (name, rgba) in swatches(s.light_theme) {
        let selected = s.color == rgba;
        colors = colors.child(
            theme::swatch(format!("swatch-{name}"), rgba, selected, pal).on_press(
                move |s: &mut DoodleApp| {
                    dismiss_popovers(s);
                    s.color = rgba;
                    s.tool = Tool::Draw;
                },
            ),
        );
    }
    bar = bar.child(colors);
    bar = bar.child(theme::divider_v(pal));
    bar = bar.child(history_row(s, pal));
    el().absolute(0.0, 10.0)
        .w_full()
        .justify_center()
        .child(island(pal).child(bar))
}

fn history_row(s: &DoodleApp, pal: &Palette) -> El {
    let undo_depth = s.doc.undo_depth();
    let redo_depth = s.doc.redo_depth();
    let mut undo = theme::flat_button("undo", "Undo", pal);
    let mut redo = theme::flat_button("redo", "Redo", pal);
    if undo_depth > 0 {
        undo = undo.on_press(|s: &mut DoodleApp| {
            dismiss_popovers(s);
            s.undo();
        });
    } else {
        undo = undo.opacity(0.4);
    }
    if redo_depth > 0 {
        redo = redo.on_press(|s: &mut DoodleApp| {
            dismiss_popovers(s);
            s.redo();
        });
    } else {
        redo = redo.opacity(0.4);
    }
    row().gap(4.0).items_center().child(undo).child(redo)
}

fn overflow_island(s: &DoodleApp, pal: &Palette) -> El {
    let mut button =
        theme::flat_button("overflow-toggle", "\u{22EF}", pal).on_press(|s: &mut DoodleApp| {
            s.menu_open = false;
            s.overflow_open = !s.overflow_open;
        });
    if s.overflow_open {
        button = button.fill(pal.hover);
    }
    island(pal)
        .pin(None, Some(12.0), Some(10.0), None)
        .child(button)
}

fn overflow_popover(s: &DoodleApp, pal: &Palette) -> El {
    let mut items = col().gap(2.0).w_full();
    for (label, key) in [
        ("Group", "group"),
        ("Ungroup", "ungroup"),
        ("Fit to content", "fit"),
    ] {
        items = items.child(theme::flat_button(key, label, pal).w_full().on_press(
            move |s: &mut DoodleApp| {
                match key {
                    "group" => s.group_selection(),
                    "ungroup" => s.ungroup_selection(),
                    _ => s.fit_to_content(),
                }
                s.overflow_open = false;
            },
        ));
    }
    items = items.child(theme::divider_h(pal));
    items = items.child(
        theme::flat_button("settings", "Canvas settings", pal)
            .w_full()
            .on_press(|s: &mut DoodleApp| {
                s.show_settings = !s.show_settings;
                s.overflow_open = false;
            }),
    );
    // Below the expand width Ask Dibby moves here so it stays reachable at every size.
    if s.window_size[0] < MENU_BAR_EXPAND_WIDTH {
        items = items.child(
            theme::flat_button("ask-dibby", "Ask Dibby", pal)
                .w_full()
                .on_press(|s: &mut DoodleApp| {
                    s.overflow_open = false;
                    s.menu_open = false;
                    s.create_ai_card();
                }),
        );
    }
    items = items.child(
        theme::flat_button("menu-help", "Shortcuts & about", pal)
            .w_full()
            .on_press(|s: &mut DoodleApp| {
                s.show_help = true;
                s.overflow_open = false;
            }),
    );
    items = items.child(theme::divider_h(pal));
    items = items.child(
        theme::flat_button("menu-zen", "Hide UI (Alt+Z)", pal)
            .w_full()
            .on_press(|s: &mut DoodleApp| {
                s.toggle_zen();
                s.overflow_open = false;
            }),
    );
    island(pal)
        .pin(None, Some(64.0), Some(10.0), None)
        .w(200.0)
        .padding(9.0)
        .child(items)
}

fn status_island(s: &DoodleApp, pal: &Palette) -> El {
    let mut content = row().gap(8.0).items_center();
    content = content.child(text("Local only").font(theme::caption()).color(pal.muted));
    content = content.child(
        text("· in memory until export")
            .font(theme::caption())
            .color(pal.muted),
    );
    if s.dirty() {
        content = content.child(text("· unsaved").font(theme::caption()).color(pal.accent));
    }
    content = content.child(
        text(format!(
            "· {} objects · {} points",
            s.projection.objects.len(),
            s.projection.total_stroke_points
        ))
        .font(theme::caption())
        .color(pal.muted),
    );
    if let Some((message, kind)) = s.notice_text() {
        content = content.child(text(message).font(theme::caption()).color(
            if kind == NoticeKind::Error {
                pal.danger
            } else {
                pal.text
            },
        ));
    }
    island(pal)
        .pin(Some(12.0), None, None, Some(10.0))
        .child(content)
}

fn zoom_island(s: &DoodleApp, pal: &Palette) -> El {
    let content = row()
        .gap(6.0)
        .items_center()
        .child(
            theme::flat_button("zoom-out", "−", pal).on_press(|s: &mut DoodleApp| {
                dismiss_popovers(s);
                s.zoom_from_center(1.0 / 1.2);
            }),
        )
        .child(
            text(format!("{:.0}%", s.zoom() * 100.0))
                .font(theme::caption())
                .color(pal.muted)
                .w(46.0),
        )
        .child(
            theme::flat_button("zoom-in", "+", pal).on_press(|s: &mut DoodleApp| {
                dismiss_popovers(s);
                s.zoom_from_center(1.2);
            }),
        )
        .child(
            theme::flat_button("zoom-fit", "Fit", pal).on_press(|s: &mut DoodleApp| {
                dismiss_popovers(s);
                s.fit_to_content();
            }),
        );
    island(pal)
        .pin(None, None, Some(12.0), Some(10.0))
        .child(content)
}

fn zen_hint(pal: &Palette) -> El {
    island(pal).pin(Some(12.0), None, None, Some(12.0)).child(
        theme::flat_button("zen-exit", "Zen mode · Alt+Z to show the UI", pal)
            .on_press(|s: &mut DoodleApp| s.zen = false),
    )
}

fn slider(pal: &Palette, key: &str, _label: &str, width: f32, fraction: f32, target: Target) -> El {
    let knob_left = (width - 12.0) * fraction;
    el().key(format!("slider-{key}"))
        .w(width)
        .h(18.0)
        .radius(9.0)
        .fill(pal.press)
        .border(1.0, pal.border)
        .on_drag(move |s: &mut DoodleApp, drag, _| {
            if drag.w > 0.0 {
                s.apply_slider(target, f64::from(drag.x / drag.w));
            }
        })
        .child(
            el().absolute(knob_left, 3.0)
                .size(12.0, 12.0)
                .radius(6.0)
                .fill(pal.accent)
                .hit(),
        )
}

fn slider_row(pal: &Palette, label: &str, key: &str, target: Target, fraction: f32) -> El {
    row()
        .gap(8.0)
        .items_center()
        .w_full()
        .child(text(label).font(theme::caption()).color(pal.muted).w(104.0))
        .child(slider(pal, key, label, 168.0, fraction, target))
}

fn settings_panel(s: &DoodleApp, pal: &Palette) -> El {
    let height = (s.window_size[1] - 150.0).clamp(280.0, 680.0);
    let mut body = col().gap(8.0).w_full();

    body = body.child(theme::section("Appearance", pal));
    let mut themes = row().gap(6.0);
    themes = themes.child(
        theme::active_button("theme-light", "Paper light", s.light_theme, pal).on_press(
            |s: &mut DoodleApp| {
                if !s.light_theme {
                    s.toggle_theme();
                }
            },
        ),
    );
    themes = themes.child(
        theme::active_button("theme-dark", "Ink dark", !s.light_theme, pal).on_press(
            |s: &mut DoodleApp| {
                if s.light_theme {
                    s.toggle_theme();
                }
            },
        ),
    );
    body = body.child(themes);

    let mut kinds = row().gap(4.0).flex_wrap();
    for kind in [
        GridKind::None,
        GridKind::Dots,
        GridKind::Pluses,
        GridKind::Lines,
    ] {
        let label = match kind {
            GridKind::None => "None",
            GridKind::Dots => "Dots",
            GridKind::Pluses => "Pluses",
            GridKind::Lines => "Lines",
        };
        kinds = kinds.child(
            theme::active_button(format!("grid-{label}"), label, s.grid.kind == kind, pal)
                .on_press(move |s: &mut DoodleApp| s.grid.kind = kind),
        );
    }
    body = body.child(theme::section("Grid", pal));
    body = body.child(kinds);

    body = body.child(theme::section("Board color", pal));
    let mut presets = row().gap(4.0).flex_wrap();
    for (name, background, lines, light) in BOARD_PRESETS {
        presets = presets.child(
            theme::active_button(
                format!("board-{name}"),
                name,
                s.grid.background_rgba == background,
                pal,
            )
            .on_press(move |s: &mut DoodleApp| s.apply_board_preset(background, lines, light)),
        );
    }
    body = body.child(presets);

    body = body.child(slider_row(
        pal,
        "Spacing",
        "grid-spacing",
        Target::GridSpacing,
        s.slider_fraction(Target::GridSpacing),
    ));
    body = body.child(slider_row(
        pal,
        "Grid opacity",
        "grid-opacity",
        Target::GridOpacity,
        s.slider_fraction(Target::GridOpacity),
    ));
    body = body.child(slider_row(
        pal,
        "Dot / plus size",
        "grid-mark",
        Target::GridMarkSize,
        s.slider_fraction(Target::GridMarkSize),
    ));
    body = body.child(slider_row(
        pal,
        "Line width",
        "grid-line",
        Target::GridLineWidth,
        s.slider_fraction(Target::GridLineWidth),
    ));

    body = body.child(theme::section("Custom board color", pal));
    for (label, target) in [
        ("Red", Target::CustomBoardRed),
        ("Green", Target::CustomBoardGreen),
        ("Blue", Target::CustomBoardBlue),
    ] {
        body = body.child(slider_row(
            pal,
            label,
            &format!("board-{label}"),
            target,
            s.slider_fraction(target),
        ));
    }
    body = body.child(theme::section("Custom grid color", pal));
    for (label, target) in [
        ("Red", Target::CustomGridRed),
        ("Green", Target::CustomGridGreen),
        ("Blue", Target::CustomGridBlue),
    ] {
        body = body.child(slider_row(
            pal,
            label,
            &format!("grid-{label}"),
            target,
            s.slider_fraction(target),
        ));
    }
    body = body.child(
        text("Grid and theme are session-only.")
            .font(theme::caption())
            .color(pal.muted),
    );

    body = body.child(theme::divider_h(pal));
    body = body.child(theme::section("Brush", pal));
    body = body.child(slider_row(
        pal,
        "Size",
        "brush-panel-size",
        Target::BrushSize,
        s.slider_fraction(Target::BrushSize),
    ));
    body = body.child(slider_row(
        pal,
        "Pressure",
        "brush-pressure",
        Target::Pressure,
        s.slider_fraction(Target::Pressure),
    ));
    body = body.child(slider_row(
        pal,
        "Smoothing",
        "brush-smoothing",
        Target::Smoothing,
        s.slider_fraction(Target::Smoothing),
    ));
    body = body.child(slider_row(
        pal,
        "Start taper",
        "brush-taper-start",
        Target::TaperStart,
        s.slider_fraction(Target::TaperStart),
    ));
    body = body.child(slider_row(
        pal,
        "End taper",
        "brush-taper-end",
        Target::TaperEnd,
        s.slider_fraction(Target::TaperEnd),
    ));
    body = body.child(
        text(
            "A tablet pen's real pressure is read on Windows through the pointer stack. Mouse \
             strokes simulate pressure from speed and stroke progress.",
        )
        .font(theme::caption())
        .color(pal.muted)
        .w_full()
        .wrap(),
    );

    body = body.child(theme::divider_h(pal));
    body = body.child(
        theme::active_button("reduced-motion", "Reduced motion", s.reduced_motion, pal)
            .on_press(|s: &mut DoodleApp| s.reduced_motion = !s.reduced_motion),
    );
    body = body.child(
        row()
            .gap(6.0)
            .child(
                theme::button("settings-fit", "Fit to content", pal)
                    .on_press(|s: &mut DoodleApp| s.fit_to_content()),
            )
            .child(
                theme::button("settings-reset", "Reset view", pal)
                    .on_press(|s: &mut DoodleApp| s.camera.reset()),
            ),
    );

    let header = row()
        .w_full()
        .gap(8.0)
        .items_center()
        .sticky_top(0.0)
        .fill(pal.card)
        .child(
            text("Canvas settings")
                .font(theme::heading(17.0))
                .color(pal.text)
                .grow(1.0),
        )
        .child(
            theme::flat_button("settings-close", "×", pal)
                .on_press(|s: &mut DoodleApp| s.show_settings = false),
        );
    body = col()
        .w_full()
        .gap(8.0)
        .child(header)
        .child(theme::divider_h(pal))
        .child(body);

    theme::island_col(pal)
        .absolute(140.0, 84.0)
        .w(320.0)
        .h(height)
        .clip_rounded(14.0)
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .child(body)
}

fn help_panel(pal: &Palette) -> El {
    let mut body = col().gap(6.0).w_full();
    body = body.child(
        text("An infinite shared doodle space")
            .font(theme::heading(19.0))
            .color(pal.text),
    );
    body = body.child(
        text(
            "Boards exist only in memory. Export writes a portable file; import always creates \
             independent objects with fresh identities.",
        )
        .font(theme::body())
        .color(pal.muted)
        .wrap(),
    );
    body = body.child(theme::section("Shortcuts", pal));
    for (key, action) in [
        ("B / V / H", "Brush / Select / Hand"),
        ("Wheel", "Pan the board"),
        ("Ctrl+wheel or pinch", "Zoom at the pointer"),
        ("Shift+click", "Toggle selection"),
        ("Ctrl+A", "Select all"),
        ("Delete", "Delete selection"),
        ("Ctrl+G", "Group selection"),
        ("Ctrl+Shift+G", "Ungroup"),
        ("Ctrl+Z / Ctrl+Y", "Undo / redo"),
        ("F / 0", "Fit content / reset view"),
        ("Alt+Z", "Zen mode"),
        ("Ctrl+O / Ctrl+S", "Open / export board"),
        ("Esc", "Cancel stroke, drag, or import"),
    ] {
        body = body.child(
            row()
                .gap(10.0)
                .w_full()
                .child(text(key).font(theme::caption()).color(pal.muted).w(150.0))
                .child(
                    text(action)
                        .font(theme::caption())
                        .color(pal.text)
                        .grow(1.0),
                ),
        );
    }
    body = body.child(theme::divider_h(pal));
    body = body.child(
        text("This build")
            .font(theme::section_style())
            .color(pal.muted),
    );
    body = body.child(
        text(
            "Native local editor: drawing, camera, grids, selection, groups, undo, portable files, \
             and local AI cards with an offline demo stream. Peer-to-peer rooms, BYOK/ChatGPT \
             connections, and ACP agents are not included yet; no credentials are stored and \
             nothing phones home.",
        )
        .font(theme::body())
        .color(pal.muted)
        .wrap(),
    );
    body = body.child(
        text(format!("Doodleverse {}", doodle_types::APP_VERSION))
            .font(theme::caption())
            .color(pal.muted),
    );
    body = body.child(
        theme::button("help-close", "Close", pal).on_press(|s: &mut DoodleApp| s.show_help = false),
    );
    island(pal).absolute(220.0, 120.0).w(420.0).child(body)
}

fn close_body(pal: &Palette) -> El {
    let mut body = col().gap(10.0).w_full();
    body = body.child(
        text("This board is not exported")
            .font(theme::heading(19.0))
            .color(pal.text),
    );
    body = body.child(
        text(
            "Doodleverse never autosaves. Closing now discards every unexported stroke, pinned \
             answer, and group in this session.",
        )
        .font(theme::body())
        .color(pal.muted)
        .wrap(),
    );
    let buttons = row()
        .gap(8.0)
        .child(
            theme::button("close-export", "Export, then close", pal).on_press_tx(
                |s: &mut DoodleApp, tx| {
                    s.show_close_confirm = false;
                    s.export_after_close = true;
                    s.export_file(tx);
                },
            ),
        )
        .child(
            theme::button("close-discard", "Discard and close", pal).on_press_tx(
                |s: &mut DoodleApp, tx| {
                    s.closing = true;
                    tx.quit();
                },
            ),
        )
        .child(
            theme::button("close-cancel", "Cancel", pal)
                .on_press(|s: &mut DoodleApp| s.show_close_confirm = false),
        );
    body = body.child(buttons);
    body
}

fn pin_body(s: &DoodleApp, pal: &Palette) -> El {
    let Some(preview) = &s.pin_preview else {
        return el();
    };
    let card_id = preview.card_id;
    let mut body = col().gap(10.0).w_full();
    body = body.child(
        text("Pin this answer?")
            .font(theme::heading(19.0))
            .color(pal.text),
    );
    body = body.child(
        text(
            "Only the selected answer below becomes shared board content. Your question and \
             other alternatives stay local to this session.",
        )
        .font(theme::body())
        .color(pal.muted)
        .wrap(),
    );
    body = body.child(
        el().w_full()
            .padding(10.0)
            .radius(8.0)
            .fill(theme::color([255, 241, 186, 255]))
            .border(1.0, theme::color([216, 198, 132, 255]))
            .child(
                el().w_full()
                    .max_h(230.0)
                    .scroll(ScrollSpec {
                        stick_bottom: false,
                    })
                    .child(
                        text(preview.text.clone())
                            .font(theme::body())
                            .color(theme::color([45, 40, 28, 255]))
                            .w_full()
                            .wrap(),
                    ),
            ),
    );
    body = body.child(
        row()
            .gap(8.0)
            .child(
                theme::button("pin-confirm", "Pin answer", pal)
                    .on_press(move |s: &mut DoodleApp| s.confirm_pin(card_id)),
            )
            .child(
                theme::button("pin-cancel", "Cancel", pal)
                    .on_press(|s: &mut DoodleApp| s.pin_preview = None),
            ),
    );
    body
}

/// A modal: a full-window scrim with the card centred inside it.
fn centered(pal: &Palette, width: f32, body: El) -> El {
    el().absolute(0.0, 0.0)
        .w_full()
        .h_full()
        .fill(theme::color([10, 8, 12, 96]))
        .items_center()
        .justify_center()
        .on_press(|_: &mut DoodleApp| {})
        .child(
            el().w(width)
                .on_press(|_: &mut DoodleApp| {})
                .child(theme::island(pal).w(width).child(body)),
        )
}
