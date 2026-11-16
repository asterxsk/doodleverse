use doodle_render::GridKind;
use rune::{
    Color, Cx, El, Key, KeyEvent, LineCap, ScrollSpec, Shape, Transition, Tx, canvas, col, el, row,
    text,
};

use crate::app::{DoodleApp, NoticeKind, Target, Tool};
use crate::theme::{self, Palette};

fn circle_points(center: [f32; 2], radius: f32, segments: usize) -> Vec<[f32; 2]> {
    (0..segments)
        .map(|index| {
            let angle = std::f32::consts::TAU * index as f32 / segments as f32;
            [
                center[0] + angle.cos() * radius,
                center[1] + angle.sin() * radius,
            ]
        })
        .collect()
}

/// Pencil barrel outline, nib lines at the lower tip, and the ferrule crossbar.
fn pencil_icon(ink: Color) -> Shape {
    Shape::new()
        .stroke_with(
            vec![[4.8, 12.6], [6.2, 14.0], [12.6, 7.6], [11.2, 6.2]],
            true,
            1.3,
            LineCap::Round,
            ink,
        )
        .line([4.8, 12.6], [3.0, 14.4], 1.3, LineCap::Round, ink)
        .line([6.2, 14.0], [3.0, 14.4], 1.3, LineCap::Round, ink)
        .line([9.8, 7.6], [11.2, 9.0], 1.3, LineCap::Round, ink)
}

/// Cursor outline (a concave-sided pointer) plus its short tail.
fn cursor_icon(ink: Color) -> Shape {
    Shape::new()
        .stroke_with(
            vec![[4.4, 4.4], [7.4, 12.9], [9.0, 9.5], [12.4, 8.0]],
            true,
            1.4,
            LineCap::Round,
            ink,
        )
        .line([9.8, 9.8], [13.1, 13.1], 1.4, LineCap::Round, ink)
}

/// Four finger lines over a simplified palm curve.
fn hand_icon(ink: Color) -> Shape {
    Shape::new()
        .line([5.2, 3.9], [5.2, 8.6], 1.4, LineCap::Round, ink)
        .line([7.0, 3.3], [7.0, 8.6], 1.4, LineCap::Round, ink)
        .line([8.8, 3.9], [8.8, 8.9], 1.4, LineCap::Round, ink)
        .line([10.6, 4.9], [10.6, 9.4], 1.4, LineCap::Round, ink)
        .stroke(
            vec![
                [4.3, 8.4],
                [4.4, 10.6],
                [5.6, 12.3],
                [7.6, 13.3],
                [9.6, 13.2],
                [11.3, 12.0],
                [12.3, 10.2],
            ],
            1.4,
            ink,
        )
}

/// Three vertical dots for the overflow trigger.
fn dots_icon(ink: Color) -> Shape {
    let mut shape = Shape::new();
    for y in [3.5, 8.0, 12.5] {
        shape = shape.fill(circle_points([8.0, y], 1.2, 12), ink);
    }
    shape
}

/// Default ink follows the board so a new stroke stays readable on a dark canvas.
fn ink_rgba(light: bool) -> [u8; 4] {
    if light {
        [32, 30, 35, 255]
    } else {
        [238, 234, 227, 255]
    }
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
        // Every top-level child carries a key. Identity is the key, not the position, so a card
        // opening or closing cannot shift the islands to new paths and remount (and re-animate)
        // the whole chrome.
        root = root.child(menu_island(s, &pal));
        root = root.child(toolbar_island(s, &pal));
        root = root.child(overflow_island(s, &pal));
        root = root.child(status_island(s, &pal));
        root = root.child(zoom_island(s, &pal));
    }

    // Panels first, then popovers: a menu or overflow menu opened over a panel must paint above
    // it and take presses on the rows the two overlap, while the modal dialogs below still sit
    // over everything.
    if s.show_settings {
        root = root.child(settings_panel(s, &pal));
    }
    if s.show_help {
        root = root.child(help_panel(&pal, s.window_size[1]));
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
        // canvas handler below from panning a second time.
        .on_wheel(|s: &mut DoodleApp, event, _| s.wheel(&event))
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
        menu_button = menu_button.fill(pal.selected_bg);
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
                s.show_settings = true;
                s.menu_open = false;
            }),
    );
    island(pal)
        .key("menu-popover")
        // Menus mount and unmount instantly: no fade or scale, open or close, and no child
        // entrance either.
        .instant_changes()
        .transition(Transition::none())
        .absolute(12.0, 64.0)
        .w(228.0)
        .padding(8.0)
        .child(items)
}

fn toolbar_island(s: &DoodleApp, pal: &Palette) -> El {
    let mut bar = row().gap(4.0).items_center();
    for tool in [Tool::Draw, Tool::Select, Tool::Hand] {
        let selected = s.tool == tool;
        let ink = if selected { pal.selected_ink } else { pal.text };
        let (name, icon) = match tool {
            Tool::Draw => ("Draw", pencil_icon(ink)),
            Tool::Select => ("Select", cursor_icon(ink)),
            Tool::Hand => ("Hand", hand_icon(ink)),
        };
        bar = bar.child(
            theme::tool_button(format!("tool-{:?}", tool), selected, pal)
                .label(name)
                .child(canvas(icon).size(16.0, 16.0))
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
        "Brush size",
        60.0,
        s.slider_fraction(Target::BrushSize),
        Target::BrushSize,
        s.slider_value_text(Target::BrushSize),
    ));
    bar = bar.child(theme::divider_v(pal));
    let mut colors = row().gap(4.0).items_center();
    for (key, name, rgba) in swatches(s.light_theme) {
        let selected = s.color == rgba;
        colors = colors.child(
            theme::swatch(format!("swatch-{key}"), rgba, selected, pal)
                .label(name)
                .on_press(move |s: &mut DoodleApp| {
                    dismiss_popovers(s);
                    s.color = rgba;
                    s.tool = Tool::Draw;
                }),
        );
    }
    bar = bar.child(colors);
    bar = bar.child(theme::divider_v(pal));
    bar = bar.child(history_row(s, pal));
    el().key("toolbar-island")
        .absolute(0.0, 10.0)
        .w_full()
        .justify_center()
        .child(island(pal).child(bar))
}

fn history_row(s: &DoodleApp, pal: &Palette) -> El {
    let undo_depth = s.doc.undo_depth();
    let redo_depth = s.doc.redo_depth();
    // Rune keeps a disabled element's role and name when both are set explicitly, so the
    // undo/redo buttons read as disabled buttons rather than anonymous groups.
    let mut undo = theme::flat_button("undo", "Undo", pal)
        .h(36.0)
        .label("Undo");
    let mut redo = theme::flat_button("redo", "Redo", pal)
        .h(36.0)
        .label("Redo");
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

fn overflow_island(s: &DoodleApp, pal: &Palette) -> El {
    let open = s.overflow_open;
    let ink = if open { pal.selected_ink } else { pal.text };
    let button = el()
        .key("overflow-toggle")
        .size(36.0, 36.0)
        .radius(8.0)
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
        .child(canvas(dots_icon(ink)).size(16.0, 16.0));
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
                s.show_settings = !s.show_settings;
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
        theme::menu_row("menu-help", "Shortcuts & about", pal)
            .w_full()
            .on_press(|s: &mut DoodleApp| {
                s.show_help = true;
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
        // Menus mount and unmount instantly: no fade or scale, open or close, and no child
        // entrance either.
        .instant_changes()
        .transition(Transition::none())
        .pin(None, Some(64.0), Some(10.0), None)
        .w(200.0)
        .padding(8.0)
        .child(items)
}

fn plural(count: usize, singular: &str) -> String {
    if count == 1 {
        format!("{count} {singular}")
    } else {
        format!("{count} {singular}s")
    }
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
            "· {} · {}",
            plural(s.projection.objects.len(), "object"),
            plural(s.projection.total_stroke_points, "point")
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
        .key("status-island")
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
        .key("zoom-island")
        .pin(None, None, Some(12.0), Some(10.0))
        .child(content)
}

fn zen_hint(pal: &Palette) -> El {
    island(pal)
        .key("zen-hint")
        .pin(Some(12.0), None, None, Some(12.0))
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
    let knob_left = (width - 12.0) * fraction;
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

fn slider_row(s: &DoodleApp, pal: &Palette, label: &str, key: &str, target: Target) -> El {
    row()
        .gap(8.0)
        .items_center()
        .w_full()
        .child(text(label).font(theme::caption()).color(pal.muted).w(104.0))
        .child(slider(
            pal,
            key,
            label,
            168.0,
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
        "Size",
        "brush-panel-size",
        Target::BrushSize,
    ));
    body = body.child(slider_row(
        s,
        pal,
        "Pressure",
        "brush-pressure",
        Target::Pressure,
    ));
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
        .key("settings-panel")
        .absolute(140.0, 84.0)
        .w(320.0)
        .h(height)
        .clip_rounded(8.0)
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .child(body)
}

fn help_panel(pal: &Palette, window_height: f32) -> El {
    let height = (window_height - 140.0).clamp(240.0, 620.0);
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
            theme::flat_button("help-close", "Close", pal)
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
    theme::island_col(pal)
        .key("help-panel")
        .absolute(220.0, 120.0)
        .w(420.0)
        .h(height)
        .radius(12.0)
        .padding(16.0)
        .clip_rounded(12.0)
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
                        .radius(12.0)
                        .padding(16.0)
                        .child(body),
                ),
        )
}
