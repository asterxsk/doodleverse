use eframe::egui;

use super::{DoodleApp, NoticeKind, Palette, Tool, to_color32};

const SWATCHES: [(&str, [u8; 4]); 6] = [
    ("Ink", [32, 30, 35, 255]),
    ("Coral", [222, 108, 85, 255]),
    ("Teal", [55, 145, 141, 255]),
    ("Gold", [214, 164, 52, 255]),
    ("Sky", [86, 132, 196, 255]),
    ("Plum", [136, 86, 150, 255]),
];

fn island(pal: &Palette) -> egui::Frame {
    egui::Frame::new()
        .fill(pal.card)
        .stroke(egui::Stroke::new(1.0, pal.border))
        .corner_radius(14)
        .inner_margin(egui::Margin::same(7))
        .shadow(egui::epaint::Shadow {
            offset: [0, 2],
            blur: 18,
            spread: 0,
            color: egui::Color32::from_black_alpha(34),
        })
}

impl DoodleApp {
    /// Excalidraw-style floating islands. Hidden in zen mode (Alt+Z).
    pub(super) fn toolbox_ui(&mut self, ctx: &egui::Context) {
        if self.zen {
            return;
        }
        let pal = self.palette();
        let rect = ctx.content_rect();

        egui::Area::new(egui::Id::unique("doodle-menu-island"))
            .order(egui::Order::Foreground)
            .fixed_pos(rect.min + egui::vec2(12.0, 10.0))
            .show(ctx, |ui| {
                island(&pal).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.menu_button(
                            egui::RichText::new("Doodleverse").strong().size(15.0),
                            |ui| {
                                if ui.button("Open board…").clicked() {
                                    self.open_file();
                                    ui.close();
                                }
                                if ui.button("Export board…").clicked() {
                                    self.export_file();
                                    ui.close();
                                }
                                ui.separator();
                                if ui.button("Canvas settings…").clicked() {
                                    self.show_settings = true;
                                    ui.close();
                                }
                                if ui.button("Shortcuts & about").clicked() {
                                    self.show_help = true;
                                    ui.close();
                                }
                                ui.separator();
                                if ui.button("Hide UI (Alt+Z)").clicked() {
                                    self.zen = true;
                                    ui.close();
                                }
                            },
                        );
                        if ui
                            .button("Ask Dibby")
                            .on_hover_text("Open a local AI brainstorm card")
                            .clicked()
                        {
                            self.create_ai_card();
                        }
                        if ui
                            .button("Export")
                            .on_hover_text("Export the visible board")
                            .clicked()
                        {
                            self.export_file();
                        }
                    });
                });
            });

        egui::Area::new(egui::Id::unique("doodle-tool-island"))
            .order(egui::Order::Foreground)
            .fixed_pos(rect.min + egui::vec2(12.0, 58.0))
            .show(ctx, |ui| {
                island(&pal).show(ui, |ui| {
                    ui.set_width(64.0);
                    ui.vertical(|ui| {
                        for (tool, label, hint) in [
                            (Tool::Draw, "Brush", "Draw freehand (B)"),
                            (Tool::Select, "Select", "Select and move objects (V)"),
                        ] {
                            if ui
                                .add_sized(
                                    [64.0, 26.0],
                                    egui::Button::new(label).selected(self.tool == tool),
                                )
                                .on_hover_text(hint)
                                .clicked()
                            {
                                self.tool = tool;
                                if tool == Tool::Select {
                                    self.stroke = None;
                                }
                            }
                        }
                        ui.separator();
                        egui::Grid::new("ink-swatches")
                            .num_columns(2)
                            .spacing([4.0, 4.0])
                            .show(ui, |ui| {
                                for (index, (name, rgba)) in SWATCHES.iter().enumerate() {
                                    let (swatch, response) = ui.allocate_exact_size(
                                        egui::vec2(28.0, 22.0),
                                        egui::Sense::click(),
                                    );
                                    ui.painter().rect_filled(swatch, 5.0, to_color32(*rgba));
                                    if self.color == *rgba {
                                        ui.painter().rect_stroke(
                                            swatch.expand(1.5),
                                            6.0,
                                            egui::Stroke::new(2.0, pal.selection),
                                            egui::StrokeKind::Outside,
                                        );
                                    }
                                    let response = response.on_hover_text(format!("{name} ink"));
                                    if response.clicked() {
                                        self.color = *rgba;
                                        self.tool = Tool::Draw;
                                    }
                                    if index % 2 == 1 {
                                        ui.end_row();
                                    }
                                }
                            });
                        ui.separator();
                        ui.label(
                            egui::RichText::new(format!("{:.0} px", self.brush.base_width))
                                .size(10.0)
                                .color(pal.muted),
                        );
                        ui.add(
                            egui::Slider::new(&mut self.brush.base_width, 1.0..=32.0)
                                .show_value(false),
                        );
                        ui.separator();
                        if ui
                            .add_enabled(self.doc.undo_depth() > 0, egui::Button::new("Undo"))
                            .on_hover_text(match self.doc.undo_label() {
                                Some(label) => format!("Undo {label} (Ctrl+Z)"),
                                None => "Ctrl+Z".to_owned(),
                            })
                            .clicked()
                        {
                            self.undo();
                        }
                        if ui
                            .add_enabled(self.doc.redo_depth() > 0, egui::Button::new("Redo"))
                            .on_hover_text(match self.doc.redo_label() {
                                Some(label) => format!("Redo {label} (Ctrl+Y)"),
                                None => "Ctrl+Y or Ctrl+Shift+Z".to_owned(),
                            })
                            .clicked()
                        {
                            self.redo();
                        }
                    });
                });
            });

        egui::Area::new(egui::Id::unique("doodle-actions-island"))
            .order(egui::Order::Foreground)
            .anchor(egui::Align2::RIGHT_TOP, [-12.0, 10.0])
            .show(ctx, |ui| {
                island(&pal).show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.set_width(76.0);
                        for (label, hint) in [
                            ("Group", "Group selection (Ctrl+G)"),
                            ("Ungroup", "Ungroup selection (Ctrl+Shift+G)"),
                            ("Fit", "Fit content to view (F)"),
                            ("Settings", "Canvas, grid, brush, theme"),
                        ] {
                            if ui.button(label).on_hover_text(hint).clicked() {
                                match label {
                                    "Group" => self.group_selection(),
                                    "Ungroup" => self.ungroup_selection(),
                                    "Fit" => self.fit_to_content(),
                                    _ => self.show_settings = !self.show_settings,
                                }
                            }
                        }
                    });
                });
            });
    }

    /// Bottom status strip and zoom controls as floating pills.
    pub(super) fn status_islands(&mut self, ctx: &egui::Context) {
        if self.zen {
            egui::Area::new(egui::Id::unique("doodle-zen-hint"))
                .order(egui::Order::Foreground)
                .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -14.0])
                .show(ctx, |ui| {
                    island(&self.palette()).show(ui, |ui| {
                        if ui
                            .button("Zen mode · Alt+Z to show the UI")
                            .on_hover_text("Excalidraw uses Alt+Z for zen mode")
                            .clicked()
                        {
                            self.zen = false;
                        }
                    });
                });
            return;
        }

        let pal = self.palette();
        egui::Area::new(egui::Id::unique("doodle-status-island"))
            .order(egui::Order::Foreground)
            .anchor(egui::Align2::LEFT_BOTTOM, [12.0, -10.0])
            .show(ctx, |ui| {
                island(&pal).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("Local only")
                                .strong()
                                .size(11.0)
                                .color(pal.muted),
                        )
                        .on_hover_text(
                            "Multiplayer, AI sign-in, and agents are not in this build. \
                             Boards are never saved automatically.",
                        );
                        ui.label(
                            egui::RichText::new("· in memory until export")
                                .size(11.0)
                                .color(pal.muted),
                        );
                        if self.dirty() {
                            ui.label(
                                egui::RichText::new("· unsaved")
                                    .size(11.0)
                                    .color(pal.accent),
                            );
                        }
                        ui.label(
                            egui::RichText::new(format!(
                                "· {} objects · {} points",
                                self.projection.objects.len(),
                                self.projection.total_stroke_points
                            ))
                            .size(11.0)
                            .color(pal.muted),
                        );
                        if let Some((text, kind)) = self.notice_text() {
                            ui.separator();
                            ui.label(egui::RichText::new(text).size(11.0).color(
                                if kind == NoticeKind::Error {
                                    pal.danger
                                } else {
                                    pal.text
                                },
                            ));
                        }
                    });
                });
            });

        egui::Area::new(egui::Id::unique("doodle-zoom-island"))
            .order(egui::Order::Foreground)
            .anchor(egui::Align2::RIGHT_BOTTOM, [-12.0, -10.0])
            .show(ctx, |ui| {
                island(&pal).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if ui.button("-").on_hover_text("Zoom out").clicked() {
                            self.zoom_from_center(1.0 / 1.2);
                        }
                        ui.label(
                            egui::RichText::new(format!("{:.0}%", self.camera.zoom * 100.0))
                                .size(11.5)
                                .color(pal.muted),
                        );
                        if ui.button("+").on_hover_text("Zoom in").clicked() {
                            self.zoom_from_center(1.2);
                        }
                        if ui
                            .button("Fit")
                            .on_hover_text("Fit content (F). Reset view with 0")
                            .clicked()
                        {
                            self.fit_to_content();
                        }
                    });
                });
            });
    }

    fn zoom_from_center(&mut self, factor: f64) {
        let center = self.canvas_rect.center();
        self.camera.zoom_at(
            doodle_types::Point::new(f64::from(center.x), f64::from(center.y)),
            super::screen_rect(self.canvas_rect),
            factor,
        );
    }
}
