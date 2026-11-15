use doodle_render::GridKind;
use eframe::egui;

use super::{DoodleApp, to_color32};

const BOARD_PRESETS: [(&str, [u8; 4], [u8; 4], bool); 6] = [
    ("Paper", [250, 248, 243, 255], [174, 168, 158, 255], true),
    ("Snow", [255, 255, 255, 255], [203, 207, 214, 255], true),
    ("Sand", [244, 235, 214, 255], [174, 158, 127, 255], true),
    ("Mist", [232, 239, 244, 255], [148, 164, 176, 255], true),
    ("Slate", [43, 45, 51, 255], [94, 99, 112, 255], false),
    ("Ink", [24, 23, 28, 255], [92, 89, 101, 255], false),
];

impl DoodleApp {
    pub(super) fn settings_window(&mut self, ctx: &egui::Context) {
        let pal = self.palette();
        let mut open = self.show_settings;
        egui::Window::new("Canvas settings")
            .open(&mut open)
            .collapsible(true)
            .resizable(false)
            .default_pos([140.0, 100.0])
            .show(ctx, |ui| {
                ui.set_width(280.0);
                ui.label(egui::RichText::new("Appearance").strong());
                ui.horizontal(|ui| {
                    if ui
                        .selectable_label(self.light_theme, "Paper light")
                        .clicked()
                        && !self.light_theme
                    {
                        self.light_theme = true;
                        ctx.set_visuals(egui::Visuals::light());
                        self.grid.background_rgba = [250, 248, 243, 255];
                        self.grid.line_rgba = [174, 168, 158, 255];
                    }
                    if ui.selectable_label(!self.light_theme, "Ink dark").clicked()
                        && self.light_theme
                    {
                        self.light_theme = false;
                        ctx.set_visuals(egui::Visuals::dark());
                        self.grid.background_rgba = [24, 23, 28, 255];
                        self.grid.line_rgba = [92, 89, 101, 255];
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Grid");
                    egui::ComboBox::from_id_salt("grid-kind")
                        .selected_text(match self.grid.kind {
                            GridKind::None => "None",
                            GridKind::Dots => "Dots",
                            GridKind::Pluses => "Pluses",
                            GridKind::Lines => "Lines",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.grid.kind, GridKind::None, "None");
                            ui.selectable_value(&mut self.grid.kind, GridKind::Dots, "Dots");
                            ui.selectable_value(&mut self.grid.kind, GridKind::Pluses, "Pluses");
                            ui.selectable_value(&mut self.grid.kind, GridKind::Lines, "Lines");
                        });
                });
                ui.label(egui::RichText::new("Board color").size(11.5));
                ui.horizontal_wrapped(|ui| {
                    for (name, background, lines, light) in BOARD_PRESETS {
                        if ui
                            .add(
                                egui::Button::new(name)
                                    .selected(self.grid.background_rgba == background),
                            )
                            .on_hover_text("Changes the board and grid colors")
                            .clicked()
                        {
                            self.grid.background_rgba = background;
                            self.grid.line_rgba = lines;
                            if self.light_theme != light {
                                self.light_theme = light;
                                ctx.set_visuals(if light {
                                    egui::Visuals::light()
                                } else {
                                    egui::Visuals::dark()
                                });
                            }
                        }
                    }
                });
                ui.add(
                    egui::Slider::new(&mut self.grid.spacing, 8.0..=200.0)
                        .text("Spacing")
                        .logarithmic(true),
                );
                ui.add(egui::Slider::new(&mut self.grid.opacity, 0.0..=1.0).text("Grid opacity"));
                ui.add(
                    egui::Slider::new(&mut self.grid.mark_size, 0.25..=8.0).text("Dot / plus size"),
                );
                ui.add(egui::Slider::new(&mut self.grid.line_width, 0.25..=8.0).text("Line width"));
                ui.horizontal(|ui| {
                    ui.label("Grid color");
                    let mut grid_color = to_color32(self.grid.line_rgba);
                    if ui.color_edit_button_srgba(&mut grid_color).changed() {
                        self.grid.line_rgba = grid_color.to_array();
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Board background");
                    let mut background = to_color32(self.grid.background_rgba);
                    if ui.color_edit_button_srgba(&mut background).changed() {
                        self.grid.background_rgba = background.to_array();
                    }
                });
                ui.label(
                    egui::RichText::new("Grid and theme are session-only.")
                        .size(11.0)
                        .color(pal.muted),
                );
                ui.separator();
                ui.label(egui::RichText::new("Brush").strong());
                ui.add(
                    egui::Slider::new(&mut self.brush.base_width, 1.0..=32.0)
                        .text("Size")
                        .logarithmic(true),
                );
                ui.add(
                    egui::Slider::new(&mut self.brush.pressure_sensitivity, 0.0..=1.0)
                        .text("Pressure"),
                );
                ui.add(egui::Slider::new(&mut self.brush.smoothing, 0.0..=1.0).text("Smoothing"));
                ui.add(
                    egui::Slider::new(&mut self.brush.taper_start, 0.0..=0.45).text("Start taper"),
                );
                ui.add(egui::Slider::new(&mut self.brush.taper_end, 0.0..=0.45).text("End taper"));
                ui.label(
                    egui::RichText::new(
                        "A tablet pen's real pressure is read on Windows through the pointer \
                         stack. Mouse strokes simulate pressure from speed and stroke progress.",
                    )
                    .size(11.0)
                    .color(pal.muted),
                );
                ui.separator();
                ui.checkbox(&mut self.reduced_motion, "Reduced motion")
                    .on_hover_text("Shows Dibby's neutral frame instead of animations");
                ui.horizontal(|ui| {
                    if ui.button("Fit to content").clicked() {
                        self.fit_to_content();
                    }
                    if ui.button("Reset view").clicked() {
                        self.camera.reset();
                    }
                });
            });
        self.show_settings = open;
    }

    pub(super) fn help_window(&mut self, ctx: &egui::Context) {
        let pal = self.palette();
        let mut open = self.show_help;
        egui::Window::new("Doodleverse")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_pos([220.0, 140.0])
            .show(ctx, |ui| {
                ui.set_width(380.0);
                ui.heading("An infinite shared doodle space");
                ui.label(
                    egui::RichText::new(
                        "Boards exist only in memory. Export writes a portable file; import always \
                         creates independent objects with fresh identities.",
                    )
                    .size(12.0)
                    .color(pal.muted),
                );
                ui.add_space(6.0);
                ui.label(egui::RichText::new("Shortcuts").strong());
                egui::Grid::new("shortcuts").num_columns(2).show(ui, |ui| {
                    for (key, action) in [
                        ("B / V", "Brush / Select"),
                        ("Space-drag or middle-drag", "Pan"),
                        ("Wheel", "Zoom at cursor"),
                        ("Shift+click", "Toggle selection"),
                        ("Ctrl+A", "Select all"),
                        ("Delete", "Delete selection"),
                        ("Ctrl+G", "Group selection"),
                        ("Ctrl+Shift+G", "Ungroup"),
                        ("Ctrl+Z / Ctrl+Y", "Undo / redo"),
                        ("F / 0", "Fit content / reset view"),
                        ("Esc", "Cancel stroke, drag, or import"),
                    ] {
                        ui.label(egui::RichText::new(key).monospace().size(11.5));
                        ui.label(egui::RichText::new(action).size(11.5));
                        ui.end_row();
                    }
                });
                ui.add_space(8.0);
                ui.label(egui::RichText::new("This build").strong());
                ui.label(
                    egui::RichText::new(
                        "Native local editor: drawing, camera, grids, selection, groups, undo, \
                         portable files, and local AI cards with an offline demo stream. \
                         Peer-to-peer rooms, BYOK/ChatGPT connections, and ACP agents are not \
                         included yet; no credentials are stored and nothing phones home.",
                    )
                    .size(11.5)
                    .color(pal.muted),
                );
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new(format!(
                        "Dibby asset bundle loaded from the approved motion manifest. \
                         Version {}",
                        doodle_types::APP_VERSION
                    ))
                    .size(11.0)
                    .color(pal.muted),
                );
            });
        self.show_help = open;
    }

    pub(super) fn close_modal(&mut self, ctx: &egui::Context) {
        if !self.show_close_confirm {
            return;
        }
        let pal = self.palette();
        let mut export_and_close = false;
        let mut discard = false;
        let mut cancel = false;
        egui::Modal::new(egui::Id::unique("unsaved-board-modal")).show(ctx, |ui| {
            ui.set_width(400.0);
            ui.heading("This board is not exported");
            ui.label(
                egui::RichText::new(
                    "Doodleverse never autosaves. Closing now discards every unexported stroke, \
                     pinned answer, and group in this session. A crash cannot be prevented either.",
                )
                .size(12.5)
                .color(pal.muted),
            );
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui.button("Export, then close").clicked() {
                    export_and_close = true;
                }
                if ui
                    .button(egui::RichText::new("Discard and close").color(pal.danger))
                    .clicked()
                {
                    discard = true;
                }
                if ui.button("Cancel").clicked() {
                    cancel = true;
                }
            });
        });
        if export_and_close {
            self.export_file();
            if !self.dirty() {
                self.request_close(ctx);
            }
        } else if discard {
            self.request_close(ctx);
        } else if cancel {
            self.show_close_confirm = false;
        }
    }
}
