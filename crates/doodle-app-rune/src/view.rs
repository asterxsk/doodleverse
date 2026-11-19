use doodle_render::GridKind;
use doodle_types::{FillStyle, StatementKind};
use rune::tok::motion;
use rune::{
    Color, Cx, El, Key, KeyEvent, ScrollSpec, Transition, Tx, canvas, col, edit_access, el, row,
    text, text_input,
};

use crate::app::{DoodleApp, NoticeKind, Target, Tool};
use crate::icons;
use crate::theme::{self, Palette};

/// Default ink follows the board so a new stroke stays readable on a dark canvas.
fn ink_rgba(light: bool) -> [u8; 4] {
    if light {
        [32, 30, 35, 255]
    } else {
        [238, 234, 227, 255]
    }
}

/// Muted coss-style background fills for closed shapes. `None` is transparent and is drawn with
/// a small cross so it reads as "no fill" rather than a white swatch.
fn background_swatches() -> [(&'static str, &'static str, [u8; 4]); 6] {
    [
        ("None", "No fill", [0, 0, 0, 0]),
        ("Sand", "Sand", [232, 226, 214, 255]),
        ("Sage", "Sage", [214, 228, 218, 255]),
        ("Rose", "Rose", [232, 218, 222, 255]),
        ("Sky", "Sky", [214, 226, 238, 255]),
        ("Stone", "Stone", [226, 226, 226, 255]),
    ]
}

fn note_swatches() -> [(&'static str, &'static str, [u8; 4]); 6] {
    [
        ("Yellow", "Yellow", [255, 241, 186, 255]),
        ("Sand", "Sand", [240, 230, 210, 255]),
        ("Sage", "Sage", [220, 236, 224, 255]),
        ("Rose", "Rose", [245, 226, 230, 255]),
        ("Sky", "Sky", [222, 234, 248, 255]),
        ("Stone", "Stone", [232, 232, 232, 255]),
    ]
}

fn swatches(light: bool) -> [(&'static str, &'static str, [u8; 4]); 6] {
    [
        ("Ink", "Ink", ink_rgba(light)),
        ("Coral", "Coral", [222, 108, 85, 255]),
        ("Teal", "Teal", [55, 145, 141, 255]),
        ("Gold", "Amber", [214, 164, 52, 255]),
        ("Sky", "Blue", [86, 132, 196, 255]),
        ("Plum", "Violet", [136, 86, 150, 255]),
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
    ("Slate", [38, 38, 38, 255], [115, 115, 115, 255], false),
    ("Ink", [22, 22, 22, 255], [82, 82, 82, 255], false),
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

    if let Some(editor) = inline_editor(s, &pal) {
        root = root.child(editor);
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
        // Every top-level child carries a key. Identity is the key, not the position, so a card
        // opening or closing cannot shift the islands to new paths and remount (and re-animate)
        // the whole chrome.
        root = root.child(menu_island(s, &pal));
        root = root.child(toolbar_island(s, &pal));
        root = root.child(overflow_island(s, &pal));
    }
    // The bottom-left cluster is Excalidraw's zoom/undo group: it stays in zen mode.
    root = root.child(bottom_cluster(s, &pal));
    if s.minimap_open {
        root = root.child(minimap_panel(s, &pal));
    }
    if let Some(notice) = notice_island(s, &pal) {
        root = root.child(notice);
    }

    // Panels first, then popovers: a menu or overflow menu opened over a panel must paint above
    // it and take presses on the rows the two overlap, while the modal dialogs below still sit
    // over everything.
    if s.tool_panel_visible() {
        root = root.child(tool_settings_panel(s, &pal));
    }
    if s.show_settings {
        root = root.child(settings_panel(s, &pal));
    }
    if s.show_help {
        root = root.child(help_panel(s, &pal));
    }

    // Popovers sit above the islands and the panels so a menu opened near one is never clipped.
    if s.menu_open {
        root = root.child(menu_popover(&pal));
    }
    if s.overflow_open {
        root = root.child(overflow_popover(s, &pal));
    }
    if s.show_close_confirm {
        root = root.child(
            centered(&pal, 420.0, "Unsaved board", close_body(&pal), |s, _| {
                s.show_close_confirm = false;
            })
            .key("close-modal")
            // Cancel, Escape and the scrim all remove the dialog; focus goes back to the menu
            // trigger the window close was started from.
            .focus_return("menu-toggle"),
        );
    }
    if let Some(preview) = &s.pin_preview {
        let card_id = preview.card_id;
        root = root.child(
            centered(
                &pal,
                440.0,
                "Pin answer",
                pin_body(s, &pal),
                move |s, tx| {
                    s.pin_preview = None;
                    // Escape is the dialog's own dismissal and Rune does not count it as focus
                    // navigation, so its restore hands focus back with the pointer modality the
                    // dialog opened with. Ask for the Pin button here with the key's own
                    // modality: a key dismissal (Escape) keeps the ring the keyboard expects,
                    // while a pointer Cancel closes without a `Tx` and stays ring-free. The
                    // button itself, not the Card, since the Card's first focusable child is
                    // its question field.
                    let ring = tx.by_keys() || tx.focus_visible();
                    tx.focus(format!("card-pin-{card_id}"), ring);
                },
            )
            .key("pin-modal")
            // Return focus to the card that opened the dialog. The pin button is marked
            // selected, so a cancel returns there; once a pin disables it, the restore falls
            // to the card's other focusable action.
            .focus_return(format!("card-{card_id}")),
        );
    }
    root
}

/// The inline editor a Text or Sticky click opens: a single-line field at the click point.
/// Return commits, Escape cancels, and a press anywhere on it stays off the board.
fn inline_editor(s: &DoodleApp, pal: &Palette) -> Option<El> {
    let editor = s.editor.as_ref()?;
    let zoom = s.zoom();
    let screen = s.camera.world_to_screen(editor.world, s.viewport());
    let sticky = editor.kind == StatementKind::Sticky;
    let width = if sticky {
        (240.0 * zoom).max(150.0) as f32
    } else {
        (320.0 * zoom).max(160.0) as f32
    };
    let x = (screen.x as f32).clamp(12.0, (s.window_size[0] - width - 12.0).max(12.0));
    let box_height = if sticky {
        (160.0 * zoom).max(80.0) as f32
    } else {
        44.0
    };
    let y = (screen.y as f32).clamp(12.0, (s.window_size[1] - box_height - 12.0).max(12.0));
    let (label, placeholder) = if sticky {
        ("Sticky note", "Sticky note…")
    } else {
        ("Text", "Text…")
    };
    let sticky_fill = s.note_rgba;
    let sticky_luma = 0.2126 * f32::from(sticky_fill[0])
        + 0.7152 * f32::from(sticky_fill[1])
        + 0.0722 * f32::from(sticky_fill[2]);
    let ink = if sticky {
        if sticky_luma < 120.0 {
            theme::color([250, 250, 250, 255])
        } else {
            pal.statement_ink
        }
    } else {
        theme::color(s.color)
    };
    let field = text_input(&s.editor_edit, theme::body())
        .key("inline-editor-field")
        .label(label)
        .on_edit(edit_access(|s: &mut DoodleApp| &mut s.editor_edit))
        .on_submit(|s: &mut DoodleApp, _| s.commit_editor())
        // The field's own key step runs before its editing step, so Escape reaches here rather
        // than being taken as an end-editing key that leaves the editor mounted.
        .on_input_key(|s: &mut DoodleApp, event, _| {
            if event.key == Key::Escape {
                s.cancel_editor();
                true
            } else {
                false
            }
        })
        .single_line()
        .placeholder(placeholder, pal.muted)
        .color(ink)
        .caret(ink, pal.selection)
        .w_full()
        .autofocus();
    let mut boxed = el()
        .w(width)
        .padding(8.0)
        .radius(8.0)
        .border(1.0, pal.selection)
        .fill(if sticky {
            theme::color(sticky_fill)
        } else {
            pal.card
        })
        .shadow(7.0, 14.0, pal.shadow)
        // The editor is a press target in its own right: clicking it must never reach the
        // canvas and start a stroke or another object.
        .hit()
        .on_press(|_: &mut DoodleApp| {})
        .child(field);
    if sticky {
        boxed = boxed.h((160.0 * zoom).max(80.0) as f32);
    }
    Some(el().key("inline-editor").absolute(x, y).child(boxed))
}

fn empty_hint(s: &DoodleApp, pal: &Palette) -> El {
    let top = (s.window_size[1] * 0.42).max(80.0);
    col()
        .key("empty-hint")
        .absolute(0.0, top)
        .w_full()
        .items_center()
        .gap(6.0)
        .hit()
        // The hint is a full-width hit layer above the canvas, so it hears wheels before the
        // canvas does. Hand them to the same board routine; returning its `true` stops the
        // canvas handler below from zooming a second time.
        .on_wheel(|s: &mut DoodleApp, event, _| s.wheel(&event))
        .child(
            text("Draw anywhere")
                .font(theme::heading(26.0))
                .color(pal.muted),
        )
        .child(
            text("P pen · V select · R rectangle · T text · N sticky · E eraser · scroll zooms")
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
    let mut menu_button = theme::flat_button("menu-toggle", "Doodleverse", pal)
        // The button opens a popover, so announce whether it is open (ARIA aria-expanded).
        .expanded(s.menu_open)
        // A dialog's close returns focus here (`focus_return` on the close modal), so the
        // trigger is a focusable control in its own right.
        .focusable(true)
        .on_press(|s: &mut DoodleApp| {
            s.overflow_open = false;
            s.menu_open = !s.menu_open;
        })
        .on_drag(|s: &mut DoodleApp, drag, _| s.popover_trigger_drag(drag));
    if s.menu_open {
        // A subtle pressed fill keeps the label readable; the primary fill is reserved for the
        // selected tool, whose icon and key badge swap to primary-foreground ink.
        menu_button = menu_button.fill(pal.press);
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
    island(pal)
        .key("menu-island")
        .absolute(12.0, 10.0)
        .child(content)
}

fn menu_scrim() -> El {
    el().key("menu-scrim")
        .absolute(0.0, 0.0)
        .w_full()
        .h_full()
        .hit()
        // A press target, not a control: keep the full-window layer out of the accessibility
        // tree so it is never announced as an unnamed button.
        .role("none")
        // Land the full-window hit area in the frame it mounts. Otherwise the scrim's first
        // frames animate in from a smaller box, and a press there reaches the canvas below.
        .instant_changes()
        .on_press(|s: &mut DoodleApp| s.dismiss_popovers_by_pointer())
}

fn overflow_scrim() -> El {
    el().key("overflow-scrim")
        .absolute(0.0, 0.0)
        .w_full()
        .h_full()
        .hit()
        // See `menu_scrim`: a pointer layer, not an unnamed full-window button.
        .role("none")
        // See `menu_scrim`: the press that dismisses the popover must never fall through.
        .instant_changes()
        .on_press(|s: &mut DoodleApp| s.dismiss_popovers_by_pointer())
}

fn menu_popover(pal: &Palette) -> El {
    let mut items = col().gap(2.0).w_full();
    for (label, action) in [("Open board…", 0u8), ("Export board…", 1)] {
        items = items.child(
            theme::menu_row(format!("menu-{action}"), label, pal)
                .w_full()
                .on_press_tx(move |s: &mut DoodleApp, tx| match action {
                    0 => s.open_file(tx),
                    _ => s.export_file(tx),
                }),
        );
    }
    items = items.child(theme::divider_h(pal));
    items = items.child(
        theme::menu_row("menu-settings", "Canvas settings…", pal)
            .w_full()
            .on_press(|s: &mut DoodleApp| {
                s.open_settings();
                s.menu_open = false;
            }),
    );
    island(pal)
        .key("menu-popover")
        // coss menu popup: scale from 96 % with a fade, on the default spring.
        .transition(Transition::scale(0.96))
        .absolute(12.0, 64.0)
        .w(228.0)
        .padding(8.0)
        .child(items)
}

fn tool_icon(tool: Tool, ink: Color) -> El {
    let name = match tool {
        Tool::Draw => "pen",
        Tool::Select => "pointer",
        Tool::Hand => "hand",
        Tool::Eraser => "eraser",
        Tool::Rectangle => "rectangle",
        Tool::Diamond => "diamond",
        Tool::Ellipse => "circle",
        Tool::Arrow => "arrow",
        Tool::Line => "line",
        Tool::Sticky => "sticky",
        Tool::Text => "text",
    };
    icons::icon(name, 16.0, ink)
}

/// One drawing tool in the toolbar: name, key badge and tooltip.
fn tool_item(s: &DoodleApp, pal: &Palette, tool: Tool, name: &str, key: &str) -> El {
    let selected = s.tool == tool;
    let ink = if selected { pal.selected_ink } else { pal.text };
    theme::tool_button(format!("tool-{tool:?}"), selected, pal)
        .label(name)
        .help(name)
        .help_keys([key])
        .child(tool_icon(tool, ink))
        .child(theme::key_badge(key, selected, pal))
        .on_press(move |s: &mut DoodleApp| {
            dismiss_popovers(s);
            s.set_tool(tool);
        })
}

fn toolbar_island(s: &DoodleApp, pal: &Palette) -> El {
    let mut bar = row().gap(4.0).items_center();
    let locked = s.toolbar_locked;
    let lock_ink = if locked { pal.selected_ink } else { pal.text };
    bar = bar.child(
        theme::tool_button("tool-Lock", locked, pal)
            .label("Lock toolbar")
            .help("Lock toolbar")
            .help_keys(["Q"])
            .child(icons::icon("lock", 16.0, lock_ink))
            .child(theme::key_badge("Q", locked, pal))
            .on_press(|s: &mut DoodleApp| {
                dismiss_popovers(s);
                s.toolbar_locked = !s.toolbar_locked;
            }),
    );
    bar = bar.child(theme::divider_v(pal));
    for (tool, name, key) in [
        (Tool::Hand, "Hand", "H"),
        (Tool::Select, "Select", "V"),
        (Tool::Rectangle, "Rectangle", "R"),
        (Tool::Diamond, "Diamond", "D"),
        (Tool::Ellipse, "Ellipse", "O"),
        (Tool::Arrow, "Arrow", "A"),
        (Tool::Line, "Line", "L"),
        // "P" is the pen's shown binding; "B" still switches to it.
        (Tool::Draw, "Pen", "P"),
        (Tool::Text, "Text", "T"),
        (Tool::Sticky, "Sticky", "N"),
        (Tool::Eraser, "Eraser", "E"),
    ] {
        bar = bar.child(tool_item(s, pal, tool, name, key));
    }
    // coss Toolbar: `rounded-xl border bg-card p-1`, groups at `gap-1`, separators between.
    let card = el()
        .hit()
        .padding(4.0)
        .radius(theme::RADIUS_TOOLBAR)
        .fill(pal.card)
        .border(1.0, pal.border)
        .instant_changes()
        .transition(Transition::none());
    el().key("toolbar-island")
        .absolute(0.0, 10.0)
        .w_full()
        .justify_center()
        .child(card.child(bar))
}

/// The left tool-settings panel, modelled on Excalidraw's panel but in coss tokens. It is the
/// primary surface for the next drawing's style; the top bar only selects tools.
fn tool_settings_panel(s: &DoodleApp, pal: &Palette) -> El {
    let closed = s.tool_panel_closed_shape();
    let mut body = theme::panel(pal);

    // Sticky notes carry a background colour, not an ink; text uses the ink swatches as its
    // glyph colour.
    if s.tool != Tool::Sticky {
        let mut stroke_row = row().gap(6.0).items_center().w_full();
        for (key, name, rgba) in swatches(s.light_theme) {
            let selected = s.color == rgba;
            stroke_row = stroke_row.child(
                theme::swatch(format!("tool-swatch-{key}"), rgba, selected, pal)
                    .label(name)
                    .on_press(move |s: &mut DoodleApp| s.color = rgba),
            );
        }
        body = body.child(panel_section(pal, "Stroke", stroke_row));
    }

    if s.tool == Tool::Sticky {
        let mut note_row = row().gap(6.0).items_center().w_full();
        for (key, name, rgba) in note_swatches() {
            let selected = s.note_rgba == rgba;
            note_row = note_row.child(
                theme::swatch(format!("tool-note-{key}"), rgba, selected, pal)
                    .label(name)
                    .on_press(move |s: &mut DoodleApp| s.note_rgba = rgba),
            );
        }
        body = body.child(panel_section(pal, "Background", note_row));
    }

    if closed {
        let mut background_row = row().gap(6.0).items_center().w_full();
        for (key, name, rgba) in background_swatches() {
            let selected = s.fill_rgba == rgba;
            let mut chip = theme::swatch(format!("tool-background-{key}"), rgba, selected, pal)
                .label(name)
                .on_press(move |s: &mut DoodleApp| s.fill_rgba = rgba);
            if rgba[3] == 0 {
                chip = chip
                    .items_center()
                    .justify_center()
                    .child(icons::icon("close", 10.0, pal.muted));
            }
            background_row = background_row.child(chip);
        }
        body = body.child(panel_section(pal, "Background", background_row));

        let mut fills = row().gap(4.0).w_full();
        for (style, label) in [
            (FillStyle::Hachure, "Hachure"),
            (FillStyle::CrossHatch, "Cross"),
            (FillStyle::Solid, "Solid"),
        ] {
            fills = fills.child(
                theme::choice_button(
                    format!("tool-fill-{label}"),
                    label,
                    s.fill_style == style,
                    pal,
                )
                .grow(1.0)
                .on_press(move |s: &mut DoodleApp| s.fill_style = style),
            );
        }
        body = body.child(panel_section(pal, "Fill", fills));
    }

    if s.tool_panel_stroke_width() {
        let mut widths = row().gap(4.0).w_full();
        for (value, label) in [(2.0_f32, "Thin"), (4.0, "Bold"), (8.0, "Extra")] {
            widths = widths.child(
                theme::choice_button(
                    format!("tool-width-{label}"),
                    label,
                    (s.brush.base_width - value).abs() < 0.5,
                    pal,
                )
                .grow(1.0)
                .on_press(move |s: &mut DoodleApp| s.brush.base_width = value),
            );
        }
        body = body.child(panel_section(pal, "Stroke width", widths));
    }

    if s.tool_panel_pen() {
        // coss Field + Switch: the label on the left, the 30×18 switch on the right.
        let pressure = theme::switch_row("tool-pressure", "Pressure", s.pressure_enabled, pal)
            .on_press(|s: &mut DoodleApp| s.pressure_enabled = !s.pressure_enabled);
        body = body.child(pressure);
    }

    let mut opacity = col().gap(6.0).w_full();
    opacity = opacity.child(
        row()
            .w_full()
            .items_center()
            .child(
                text("Opacity")
                    .font(theme::field_label_style())
                    .color(pal.text)
                    .grow(1.0),
            )
            .child(
                text(format!("{:.0}%", s.opacity * 100.0))
                    .font(rune::tok::TextStyle::new(14.0, 400.0))
                    .color(pal.muted),
            ),
    );
    opacity = opacity.child(slider(
        pal,
        "opacity",
        "Opacity",
        184.0,
        s.slider_fraction(Target::Opacity),
        Target::Opacity,
        s.slider_value_text(Target::Opacity),
    ));
    body = body.child(opacity);

    // The panel floats at the left edge, vertically centred on the window rather than pinned
    // under the toolbar, so the tools sit at eye level. The anchor spans the full height and is
    // not hit-testable; only the panel itself takes pointer input.
    col()
        .key("tool-settings-anchor")
        .absolute(crate::app::TOOL_PANEL_X, 0.0)
        .w(crate::app::TOOL_PANEL_WIDTH)
        .h(s.window_size[1])
        .justify_center()
        .child(
            body.key("tool-settings-panel")
                .w(crate::app::TOOL_PANEL_WIDTH)
                // coss popup motion: a small scale-up with the fade as the panel appears for a
                // drawing tool and leaves for a navigation tool.
                .transition(Transition::scale(0.98))
                .on_drawn_layout(|s: &mut DoodleApp, rect: [f32; 4]| {
                    s.tool_panel_layout = rect;
                }),
        )
}

fn panel_section(pal: &Palette, title: &str, control: El) -> El {
    // coss `Field`: a 14/500 label in the primary ink above its control.
    col()
        .w_full()
        .gap(6.0)
        .child(text(title).font(theme::field_label_style()).color(pal.text))
        .child(control)
}

fn history_row(s: &DoodleApp, pal: &Palette) -> El {
    let undo_depth = s.doc.undo_depth();
    let redo_depth = s.doc.redo_depth();
    // Rune keeps a disabled element's role and name when both are set explicitly, so the
    // undo/redo buttons read as disabled buttons rather than anonymous groups.
    let mut undo = theme::flat_icon_button("undo", pal)
        .label("Undo")
        .child(icons::icon("undo", 16.0, pal.text));
    let mut redo = theme::flat_icon_button("redo", pal)
        .label("Redo")
        .child(icons::icon("redo", 16.0, pal.text));
    if undo_depth > 0 {
        undo = undo.on_press(|s: &mut DoodleApp| {
            dismiss_popovers(s);
            s.undo();
        });
    } else {
        // `disabled` stops focus but leaves the box without a press target, so a press would
        // fall through to the board. `hit` keeps the press and consumes it.
        undo = undo.opacity(0.4).role("button").disabled(true).hit();
    }
    if redo_depth > 0 {
        redo = redo.on_press(|s: &mut DoodleApp| {
            dismiss_popovers(s);
            s.redo();
        });
    } else {
        redo = redo.opacity(0.4).role("button").disabled(true).hit();
    }
    row().gap(4.0).items_center().child(undo).child(redo)
}

/// A 28×28 icon button for the bottom-left cluster.
fn cluster_button(key: &str, label: &str, selected: bool, pal: &Palette, icon: &str) -> El {
    let ink = if selected { pal.selected_ink } else { pal.text };
    theme::tool_button(key, selected, pal)
        .size(28.0, 28.0)
        .label(label)
        .help(label)
        .child(icons::icon(icon, 15.0, ink))
}

fn bottom_cluster(s: &DoodleApp, pal: &Palette) -> El {
    let mut bar = row().gap(4.0).items_center();
    bar = bar.child(
        text(format!("{:.0}%", s.zoom() * 100.0))
            .font(theme::caption())
            .color(pal.muted)
            .w(42.0),
    );
    bar = bar.child(
        cluster_button(
            "board-settings",
            "Board settings",
            s.show_settings,
            pal,
            "settings",
        )
        .on_press(|s: &mut DoodleApp| {
            dismiss_popovers(s);
            s.toggle_settings();
        }),
    );
    bar = bar.child(
        cluster_button("minimap-toggle", "Minimap", s.minimap_open, pal, "map").on_press(
            |s: &mut DoodleApp| {
                dismiss_popovers(s);
                s.toggle_minimap();
            },
        ),
    );
    bar = bar.child(theme::divider_v(pal));
    bar = bar.child(history_row(s, pal));
    el().key("bottom-cluster")
        .pin(Some(12.0), None, None, Some(10.0))
        .h(36.0)
        .padding(4.0)
        .radius(theme::RADIUS_SURFACE)
        .fill(pal.card)
        .border(1.0, pal.border)
        .shadow(7.0, 14.0, pal.shadow)
        .hit()
        .child(bar)
}

/// The minimap panel above the bottom-left cluster. It is a persistent panel, not a popover:
/// the toggle button opens and closes it, and Escape leaves it alone.
///
/// Wheel over the panel is ignored (the panel consumes it) rather than zooming the board; the
/// board still zooms normally everywhere outside the panel.
fn minimap_panel(s: &DoodleApp, pal: &Palette) -> El {
    let [x, y, width, height] = s.minimap_rect();
    theme::island(pal)
        .key("minimap-panel")
        // coss popup motion; the 0.98 scale keeps a quick open from reading as a flash.
        .transition(Transition::scale(0.98))
        .absolute(x, y)
        .padding(0.0)
        .w(width)
        .h(height)
        .clip_rounded(theme::RADIUS_SURFACE)
        .child(
            canvas(crate::canvas::minimap_scene(s, pal))
                .size(width, height)
                .radius(theme::RADIUS_SURFACE)
                .label("Minimap")
                .cursor(rune::Cursor::Pointer)
                .on_drag(|s: &mut DoodleApp, drag, _| s.minimap_drag(drag))
                // Documented choice: wheel over the minimap is ignored.
                .on_wheel(|_: &mut DoodleApp, _, _| true),
        )
}

/// Transient messages float above the bottom-left cluster (and above the minimap when it is
/// open), replacing the removed status pill's notice slot.
fn notice_island(s: &DoodleApp, pal: &Palette) -> Option<El> {
    let (message, kind) = s.notice_text()?;
    let bottom = if s.minimap_open { 206.0 } else { 54.0 };
    Some(
        island(pal)
            .key("notice-island")
            .pin(Some(12.0), None, None, Some(bottom))
            .child(
                text(message)
                    .font(theme::caption())
                    .color(if kind == NoticeKind::Error {
                        pal.danger
                    } else {
                        pal.text
                    }),
            ),
    )
}

fn overflow_island(s: &DoodleApp, pal: &Palette) -> El {
    let open = s.overflow_open;
    let ink = if open { pal.selected_ink } else { pal.text };
    let button = el()
        .key("overflow-toggle")
        .size(32.0, 32.0)
        .radius(theme::RADIUS_SURFACE)
        .items_center()
        .justify_center()
        .fill(if open {
            pal.selected_bg
        } else {
            Color::TRANSPARENT
        })
        .press(pal.press)
        .label("More tools")
        .expanded(open)
        .on_press(|s: &mut DoodleApp| {
            s.menu_open = false;
            s.overflow_open = !s.overflow_open;
        })
        .on_drag(|s: &mut DoodleApp, drag, _| s.popover_trigger_drag(drag))
        .child(icons::icon("more", 16.0, ink));
    island(pal)
        .key("overflow-island")
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
        items = items.child(theme::menu_row(key, label, pal).w_full().on_press(
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
        theme::menu_row("settings", "Canvas settings", pal)
            .w_full()
            .on_press(|s: &mut DoodleApp| {
                s.toggle_settings();
                s.overflow_open = false;
            }),
    );
    // Below the expand width Ask Dibby and Export move here so both stay reachable at every size.
    if s.window_size[0] < MENU_BAR_EXPAND_WIDTH {
        items = items.child(
            theme::menu_row("ask-dibby", "Ask Dibby", pal)
                .w_full()
                .on_press(|s: &mut DoodleApp| {
                    s.overflow_open = false;
                    s.menu_open = false;
                    s.create_ai_card();
                }),
        );
        items = items.child(
            theme::menu_row("export", "Export board", pal)
                .w_full()
                .on_press_tx(|s: &mut DoodleApp, tx| {
                    dismiss_popovers(s);
                    s.export_file(tx);
                }),
        );
    }
    items = items.child(
        theme::menu_row_icon("menu-help", "Shortcuts & about", "help", pal)
            .w_full()
            .on_press(|s: &mut DoodleApp| {
                s.open_help();
                s.overflow_open = false;
            }),
    );
    items = items.child(theme::divider_h(pal));
    items = items.child(
        theme::menu_row("menu-zen", "Hide UI (Alt+Z)", pal)
            .w_full()
            .on_press(|s: &mut DoodleApp| {
                s.toggle_zen();
                s.overflow_open = false;
            }),
    );
    island(pal)
        .key("overflow-popover")
        // coss menu popup: scale from 96 % with a fade, on the default spring.
        .transition(Transition::scale(0.96))
        .pin(None, Some(64.0), Some(10.0), None)
        .w(200.0)
        .padding(8.0)
        .child(items)
}

fn zen_hint(pal: &Palette) -> El {
    // Top-left: the bottom-left corner belongs to the cluster in zen mode too.
    island(pal)
        .key("zen-hint")
        .pin(Some(12.0), Some(12.0), None, None)
        .child(
            theme::flat_button("zen-exit", "Zen mode · Alt+Z to show the UI", pal)
                .on_press(|s: &mut DoodleApp| s.zen = false),
        )
}

fn slider(
    pal: &Palette,
    key: &str,
    label: &str,
    width: f32,
    fraction: f32,
    target: Target,
    value_text: String,
) -> El {
    // coss slider: a 4pt `bg-input` track inset 2pt each side, a `bg-primary` indicator that
    // runs to the thumb centre, and a 16pt white thumb with an `input` (light) or `background`
    // (dark) border that scales to 1.2 while dragged.
    const THUMB: f32 = 16.0;
    const TRACK: f32 = 4.0;
    let thumb_left = 2.0 + (width - 4.0 - THUMB).max(0.0) * fraction;
    let filled = (thumb_left + THUMB * 0.5 - 2.0).max(0.0);
    el().key(format!("slider-{key}"))
        .label(label)
        // Rune has an accessible slider role and value, so advertise it to assistive
        // technology instead of leaving the track as an unnamed group.
        .role("slider")
        .a11y_value(value_text)
        .range_value(f64::from(fraction), 0.0, 1.0)
        .range_step(0.05)
        .on_step(move |s: &mut DoodleApp, delta, _| {
            let next = f64::from(s.slider_fraction(target)) + f64::from(delta) * 0.05;
            s.apply_slider(target, next.clamp(0.0, 1.0));
        })
        .w(width)
        .h(20.0)
        .hit()
        .on_drag(move |s: &mut DoodleApp, drag, _| {
            if drag.w > 0.0 {
                s.apply_slider(target, f64::from(drag.x / drag.w));
            }
        })
        // A thin recessed track with a filled progress run and a raised knob; the old pill
        // with a flat 12pt dot read as a disabled control.
        .child(
            el().absolute(2.0, (20.0 - TRACK) * 0.5)
                .w(width - 4.0)
                .h(TRACK)
                .radius(TRACK * 0.5)
                .fill(pal.input),
        )
        .child(
            el().absolute(2.0, (20.0 - TRACK) * 0.5)
                .w(filled)
                .h(TRACK)
                .radius(TRACK * 0.5)
                .fill(pal.accent)
                .motion(motion::SNAPPY),
        )
        .child(
            el().absolute(thumb_left, (20.0 - THUMB) * 0.5)
                .size(THUMB, THUMB)
                .radius(THUMB * 0.5)
                .fill(theme::color([255, 255, 255, 255]))
                .border(1.0, if pal.dark { pal.bg } else { pal.input })
                .shadow(2.0, 5.0, pal.shadow)
                .press_scale(1.2)
                .motion(motion::SNAPPY)
                .hit(),
        )
}

fn slider_row(s: &DoodleApp, pal: &Palette, label: &str, key: &str, target: Target) -> El {
    let width = 304.0;
    col()
        .gap(6.0)
        .w_full()
        .child(
            row()
                .w_full()
                .items_center()
                .child(
                    text(label)
                        .font(rune::tok::TextStyle::new(14.0, 500.0))
                        .color(pal.text)
                        .grow(1.0),
                )
                // coss `SliderValue`: `text-sm`, muted, right aligned under the label row.
                .child(
                    text(s.slider_value_text(target))
                        .font(rune::tok::TextStyle::new(14.0, 400.0))
                        .color(pal.muted),
                ),
        )
        .child(slider(
            pal,
            key,
            label,
            width,
            s.slider_fraction(target),
            target,
            s.slider_value_text(target),
        ))
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
        s,
        pal,
        "Spacing",
        "grid-spacing",
        Target::GridSpacing,
    ));
    body = body.child(slider_row(
        s,
        pal,
        "Grid opacity",
        "grid-opacity",
        Target::GridOpacity,
    ));
    body = body.child(slider_row(
        s,
        pal,
        "Dot / plus size",
        "grid-mark",
        Target::GridMarkSize,
    ));
    body = body.child(slider_row(
        s,
        pal,
        "Line width",
        "grid-line",
        Target::GridLineWidth,
    ));

    body = body.child(theme::section("Custom board color", pal));
    for (label, target) in [
        ("Red", Target::CustomBoardRed),
        ("Green", Target::CustomBoardGreen),
        ("Blue", Target::CustomBoardBlue),
    ] {
        body = body.child(slider_row(s, pal, label, &format!("board-{label}"), target));
    }
    body = body.child(theme::section("Custom grid color", pal));
    for (label, target) in [
        ("Red", Target::CustomGridRed),
        ("Green", Target::CustomGridGreen),
        ("Blue", Target::CustomGridBlue),
    ] {
        body = body.child(slider_row(s, pal, label, &format!("grid-{label}"), target));
    }
    body = body.child(
        text("Grid and theme are session-only.")
            .font(theme::caption())
            .color(pal.muted),
    );

    body = body.child(theme::divider_h(pal));
    body = body.child(theme::section("Brush", pal));
    body = body.child(slider_row(
        s,
        pal,
        "Smoothing",
        "brush-smoothing",
        Target::Smoothing,
    ));
    body = body.child(slider_row(
        s,
        pal,
        "Start taper",
        "brush-taper-start",
        Target::TaperStart,
    ));
    body = body.child(slider_row(
        s,
        pal,
        "End taper",
        "brush-taper-end",
        Target::TaperEnd,
    ));
    body = body.child(
        text(
            "A tablet pen's real pressure is read on Windows through the pointer stack. Mouse \
             strokes simulate pressure from speed: slow strokes are thick, fast ones are thin.",
        )
        .font(theme::caption())
        .color(pal.muted)
        .w_full()
        .wrap(),
    );

    body = body.child(theme::divider_h(pal));
    body = body.child(
        theme::switch_row("reduced-motion", "Reduced motion", s.reduced_motion, pal)
            .on_press(|s: &mut DoodleApp| s.reduced_motion = !s.reduced_motion),
    );
    body = body.child(
        row()
            .gap(6.0)
            .child(
                theme::outlined_button("settings-fit", "Fit to content", pal)
                    .on_press(|s: &mut DoodleApp| s.fit_to_content()),
            )
            .child(
                theme::outlined_button("settings-reset", "Reset view", pal)
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
            theme::flat_icon_button("settings-close", pal)
                .label("Close settings")
                .child(icons::icon("x", 16.0, pal.text))
                .on_press(|s: &mut DoodleApp| s.show_settings = false),
        );
    body = col()
        .w_full()
        .gap(8.0)
        .child(header)
        .child(theme::divider_h(pal))
        .child(body);

    // Step past the tool-settings panel instead of painting under it.
    let x = if s.tool_panel_visible() { 240.0 } else { 140.0 };
    theme::island_col(pal)
        .key("settings-panel")
        // coss popup motion, so the panel settles in instead of appearing whole.
        .transition(Transition::scale(0.98))
        .absolute(x, 84.0)
        .w(320.0)
        .h(height)
        .clip_rounded(theme::RADIUS_SURFACE)
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .child(body)
}

fn help_panel(s: &DoodleApp, pal: &Palette) -> El {
    // The same rect `DoodleApp::help_rect` guards the wheel with, so the panel's scroll view
    // always wins over the board zoom.
    let [x, y, width, height] = s.help_rect().unwrap_or([220.0, 120.0, 420.0, 480.0]);
    // Sticky header, as on the settings panel, so Close stays reachable however short the
    // window is and however far the body scrolls.
    let header = row()
        .w_full()
        .gap(8.0)
        .items_center()
        .sticky_top(0.0)
        .fill(pal.card)
        .child(
            text("Shortcuts & about")
                .font(theme::heading(17.0))
                .color(pal.text)
                .grow(1.0),
        )
        .child(
            theme::flat_icon_button("help-close", pal)
                .label("Close")
                .child(icons::icon("x", 16.0, pal.text))
                .on_press(|s: &mut DoodleApp| s.show_help = false),
        );
    let mut body = col()
        .w_full()
        .gap(6.0)
        .child(header)
        .child(theme::divider_h(pal));
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
        ("P (or B) / V / H / E", "Pen / Select / Hand / Eraser"),
        (
            "R / D / O / A / L",
            "Rectangle / Diamond / Ellipse / Arrow / Line",
        ),
        ("T / N", "Text / Sticky note"),
        ("Q", "Lock the toolbar in place"),
        ("Wheel", "Zoom at the pointer"),
        ("Ctrl+wheel or pinch", "Zoom at the pointer"),
        ("Shift+wheel", "Pan horizontally"),
        ("Hand tool or middle drag", "Pan the board"),
        ("Shift+click", "Toggle selection"),
        ("Shift+drag a shape", "Constrain to square, circle or 45°"),
        ("Ctrl+A", "Select all"),
        ("Delete", "Delete selection"),
        ("Ctrl+G", "Group selection"),
        ("Ctrl+Shift+G", "Ungroup"),
        ("Ctrl+Z / Ctrl+Y", "Undo / redo"),
        ("F / 0", "Fit content / reset view"),
        ("Alt+Z", "Zen mode"),
        ("Ctrl+O / Ctrl+S", "Open / export board"),
        ("Esc", "Cancel editor, stroke, drag, or import"),
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
                        .grow(1.0)
                        .wrap(),
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
    theme::island_col(pal)
        .key("help-panel")
        // coss popup motion, so the panel settles in instead of appearing whole.
        .transition(Transition::scale(0.98))
        .absolute(x, y)
        .w(width)
        .h(height)
        .radius(theme::RADIUS_SURFACE)
        .padding(16.0)
        .clip_rounded(theme::RADIUS_SURFACE)
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .child(body)
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
            theme::outlined_button("close-export", "Export, then close", pal)
                .focusable(true)
                .autofocus()
                .on_key(enter_activates_tx(|s: &mut DoodleApp, tx| {
                    s.show_close_confirm = false;
                    s.export_after_close = true;
                    s.export_file(tx);
                }))
                .on_press_tx(|s: &mut DoodleApp, tx| {
                    s.show_close_confirm = false;
                    s.export_after_close = true;
                    s.export_file(tx);
                }),
        )
        .child(
            theme::outlined_button("close-discard", "Discard and close", pal)
                .focusable(true)
                .on_key(enter_activates_tx(|s: &mut DoodleApp, tx| {
                    s.closing = true;
                    tx.quit();
                }))
                .on_press_tx(|s: &mut DoodleApp, tx| {
                    s.closing = true;
                    tx.quit();
                }),
        )
        .child(
            theme::outlined_button("close-cancel", "Cancel", pal)
                .focusable(true)
                .on_key(enter_activates(|s: &mut DoodleApp| {
                    s.show_close_confirm = false;
                }))
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
                            .color(pal.statement_ink)
                            .w_full()
                            .wrap(),
                    ),
            ),
    );
    body = body.child(
        row()
            .gap(8.0)
            .child(
                theme::outlined_button("pin-confirm", "Pin answer", pal)
                    .focusable(true)
                    .autofocus()
                    .on_key(enter_activates_tx(move |s: &mut DoodleApp, tx| {
                        s.confirm_pin(card_id);
                        // Pinning disables the card's Pin button, so focus lands on the card's
                        // generate action instead of falling back to the question field. Keep
                        // the keyboard ring: it was the keyboard that confirmed, and the Cancel
                        // path restores with one. Enter is not focus navigation, so on the
                        // autofocused button `focus_visible` still reports the pointer that
                        // opened the dialog; `by_keys` says this change came from a key.
                        let ring = tx.by_keys() || tx.focus_visible();
                        tx.focus(format!("card-go-{card_id}"), ring);
                    }))
                    .on_press_tx(move |s: &mut DoodleApp, tx| {
                        s.confirm_pin(card_id);
                        // A pointer click came from no key and keeps no ring; a keyboard press
                        // (Space on the focused button) does.
                        let ring = tx.by_keys() || tx.focus_visible();
                        tx.focus(format!("card-go-{card_id}"), ring);
                    }),
            )
            .child(
                theme::outlined_button("pin-cancel", "Cancel", pal)
                    .focusable(true)
                    .on_key(enter_activates(|s: &mut DoodleApp| s.pin_preview = None))
                    .on_press(|s: &mut DoodleApp| s.pin_preview = None),
            ),
    );
    body
}

/// Return key on a focused dialog button. Rune presses a focused control on Space, and without
/// `Cx::web_focus` Return goes only to a default button, so each dialog action wires Return to
/// the same action. The handler is on the button itself, so Return acts on what is focused.
fn enter_activates(
    action: impl Fn(&mut DoodleApp) + 'static,
) -> impl Fn(&mut DoodleApp, &KeyEvent, &mut Tx) -> bool + 'static {
    move |s, ev, _| {
        if ev.key == Key::Enter {
            action(s);
            true
        } else {
            false
        }
    }
}

/// [`enter_activates`] for an action that needs the change's [`Tx`], such as quitting.
fn enter_activates_tx(
    action: impl Fn(&mut DoodleApp, &mut Tx) + 'static,
) -> impl Fn(&mut DoodleApp, &KeyEvent, &mut Tx) -> bool + 'static {
    move |s, ev, tx| {
        if ev.key == Key::Enter {
            action(s, tx);
            true
        } else {
            false
        }
    }
}

/// A modal: a full-window scrim with the card centred inside it. The root is the dialog; the
/// pressable scrim keeps taking presses but announces nothing, and the card wrapper is a plain
/// presentation box, so its copies and buttons belong to the dialog instead of becoming one large
/// unnamed button. The modal trap holds Tab and Return inside the dialog, focuses its `autofocus`
/// action on open, and makes every layer behind it inert and absent from the accessibility tree;
/// Escape and `dismiss` close it.
fn centered(
    pal: &Palette,
    width: f32,
    label: &str,
    body: El,
    dismiss: impl Fn(&mut DoodleApp, &mut Tx) + 'static,
) -> El {
    el().absolute(0.0, 0.0)
        .w_full()
        .h_full()
        .fill(pal.scrim)
        .backdrop_blur(4.0)
        .items_center()
        .justify_center()
        .role("dialog")
        .label(label)
        .modal_tx(false, dismiss)
        .on_press(|_: &mut DoodleApp| {})
        .child(
            el().w(width)
                .role("none")
                .on_press(|_: &mut DoodleApp| {})
                .child(
                    theme::island(pal)
                        .w(width)
                        .radius(theme::RADIUS_DIALOG)
                        .padding(24.0)
                        // coss dialog popup: 200 ms scale 0.98 → 1 with the fade.
                        .transition(Transition::scale(0.98))
                        .child(body),
                ),
        )
}
