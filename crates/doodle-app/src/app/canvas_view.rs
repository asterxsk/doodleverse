use std::collections::HashMap;

use doodle_canvas::geometry::stroke_hit_clearance;
use doodle_canvas::{RawSample, StrokeBuilder, stroke_mesh};
use doodle_render::grid_frame;
use doodle_types::{ObjectId, ObjectKind, Point, Pose, Rect};
use eframe::egui;

use super::{
    DoodleApp, Marquee, SelectDrag, StrokeGesture, Tool, placement_rect, placement_translation,
    screen_rect, to_color32,
};

impl DoodleApp {
    pub(super) fn canvas_ui(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let rect = ui.available_rect_before_wrap();
        self.canvas_rect = rect;
        let viewport = screen_rect(rect);
        let response = ui.allocate_rect(rect, egui::Sense::click_and_drag());
        self.handle_shortcuts(&ctx, viewport);
        self.handle_pointer(&ctx, &response, viewport);
        self.draw_scene(ui, &self.palette(), viewport);
        self.draw_overlays(ui, &self.palette(), viewport);
        let _ = response;
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context, viewport: Rect) {
        let typing = ctx.memory(|memory| memory.focused().is_some());
        let (b, v, delete, escape, fit, reset, zen_toggle) = ctx.input(|i| {
            (
                i.key_pressed(egui::Key::B),
                i.key_pressed(egui::Key::V),
                i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace),
                i.key_pressed(egui::Key::Escape),
                i.key_pressed(egui::Key::F),
                i.key_pressed(egui::Key::Num0),
                i.modifiers.alt && i.key_pressed(egui::Key::Z),
            )
        });
        self.space_down = !typing && ctx.input(|i| i.key_down(egui::Key::Space));
        if typing {
            return;
        }
        if zen_toggle {
            self.zen = !self.zen;
        }
        if b {
            self.tool = Tool::Draw;
        }
        if v {
            self.tool = Tool::Select;
            self.stroke = None;
        }
        if delete {
            self.delete_selection();
        }
        if escape {
            self.stroke = None;
            self.drag = None;
            self.marquee = None;
            self.pin_preview = None;
            self.cancel_pending_import();
        }
        if fit {
            self.fit_to_content();
        }
        if reset {
            self.camera.reset();
        }
        let (command, shift, z, y, a, g, plus, minus) = ctx.input(|i| {
            (
                i.modifiers.command,
                i.modifiers.shift,
                i.key_pressed(egui::Key::Z),
                i.key_pressed(egui::Key::Y),
                i.key_pressed(egui::Key::A),
                i.key_pressed(egui::Key::G),
                i.key_pressed(egui::Key::Plus) || i.key_pressed(egui::Key::Equals),
                i.key_pressed(egui::Key::Minus),
            )
        });
        if command && z {
            if shift {
                self.redo();
            } else {
                self.undo();
            }
        }
        if command && y {
            self.redo();
        }
        if command && a {
            self.select_all();
        }
        if command && g {
            if shift {
                self.ungroup_selection();
            } else {
                self.group_selection();
            }
        }
        if plus || minus {
            let factor = if plus { 1.2 } else { 1.0 / 1.2 };
            let center = self.canvas_rect.center();
            self.camera.zoom_at(
                Point::new(f64::from(center.x), f64::from(center.y)),
                viewport,
                factor,
            );
        }
    }

    fn handle_pointer(&mut self, ctx: &egui::Context, response: &egui::Response, viewport: Rect) {
        let palette = self.palette();
        let pointer = ctx.input(|i| i.pointer.interact_pos());
        let world = pointer
            .map(|pos| {
                self.camera
                    .screen_to_world(Point::new(f64::from(pos.x), f64::from(pos.y)), viewport)
            })
            .unwrap_or(self.last_pointer_world);
        self.last_pointer_world = world;

        // Zoom at the cursor. Wheel/trackpad scrolling is camera zoom in this canvas.
        if response.hovered() {
            let scroll = ctx.input(|i| i.smooth_scroll_delta.y);
            if scroll.abs() > 0.05
                && let Some(pos) = pointer
            {
                let factor = f64::from(scroll) * 0.0022;
                self.camera.zoom_at(
                    Point::new(f64::from(pos.x), f64::from(pos.y)),
                    viewport,
                    factor.exp(),
                );
            }
            let icon = if self.space_down {
                egui::CursorIcon::Grab
            } else {
                match self.tool {
                    Tool::Draw => egui::CursorIcon::Crosshair,
                    Tool::Select => egui::CursorIcon::Default,
                }
            };
            ctx.set_cursor_icon(icon);
        }

        // Pan: middle-drag, or Space + primary-drag.
        if response.dragged_by(egui::PointerButton::Middle) {
            let delta = response.drag_delta();
            self.camera
                .pan_by_screen_delta(f64::from(delta.x), f64::from(delta.y));
            ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
        }

        let primary_pressed = ctx.input(|i| i.pointer.primary_pressed());
        let primary_down = ctx.input(|i| i.pointer.primary_down());
        let primary_released = ctx.input(|i| i.pointer.primary_released());
        let shift = ctx.input(|i| i.modifiers.shift);
        let inside = response.hovered() || response.contains_pointer();

        if self.space_down && primary_down && inside {
            self.pan_active = true;
        }
        if self.pan_active {
            let delta = ctx.input(|i| i.pointer.delta());
            if primary_down {
                self.camera
                    .pan_by_screen_delta(f64::from(delta.x), f64::from(delta.y));
                self.drag = None;
                self.marquee = None;
                self.stroke = None;
                ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
                return;
            }
            self.pan_active = false;
        }

        self.update_pending_import(world);

        if self.pending_import.is_some() {
            ctx.set_cursor_icon(egui::CursorIcon::Crosshair);
            if primary_pressed && inside {
                self.commit_pending_import();
            }
            return;
        }

        if primary_pressed && inside {
            match self.tool {
                Tool::Draw => {
                    let time = ctx.input(|i| i.time);
                    let start = RawSample::new(world, doodle_platform::pen_pressure(), time);
                    let builder =
                        StrokeBuilder::new(self.brush, start, 0.7 / self.camera.zoom.max(0.05));
                    self.stroke = Some(StrokeGesture {
                        builder,
                        style: self.brush.style(self.color),
                    });
                    self.hint_visible = false;
                    self.drag = None;
                    self.marquee = None;
                }
                Tool::Select => {
                    if let Some(top) = self.hit_top(world) {
                        if shift {
                            if !self.selection.insert(top) {
                                self.selection.remove(&top);
                            }
                        } else if !self.selection.contains(&top) {
                            self.selection.clear();
                            self.selection.insert(top);
                        }
                        self.drag = Some(SelectDrag {
                            start_screen: pointer.unwrap_or_default(),
                            last_screen: pointer.unwrap_or_default(),
                            moved: false,
                            top_ids: self.selected_top_ids(),
                        });
                        self.marquee = None;
                    } else {
                        if !shift {
                            self.selection.clear();
                        }
                        self.marquee = Some(Marquee {
                            start_screen: pointer.unwrap_or_default(),
                            current_screen: pointer.unwrap_or_default(),
                        });
                        self.drag = None;
                    }
                }
            }
        }

        if let Some(gesture) = self.stroke.as_mut()
            && primary_down
        {
            let events = ctx.input(|i| {
                i.events
                    .iter()
                    .filter_map(|event| match event {
                        egui::Event::PointerMoved(pos) => Some(*pos),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
            });
            let time = ctx.input(|i| i.time);
            let pressure = doodle_platform::pen_pressure();
            if events.is_empty() {
                if let Some(pos) = pointer {
                    let sample_world = self
                        .camera
                        .screen_to_world(Point::new(f64::from(pos.x), f64::from(pos.y)), viewport);
                    gesture
                        .builder
                        .push(RawSample::new(sample_world, pressure, time));
                }
            } else {
                let count = events.len();
                for (index, pos) in events.into_iter().enumerate() {
                    let sample_world = self
                        .camera
                        .screen_to_world(Point::new(f64::from(pos.x), f64::from(pos.y)), viewport);
                    let stamp = time - (count - 1 - index) as f64 * 0.003;
                    gesture
                        .builder
                        .push(RawSample::new(sample_world, pressure, stamp));
                }
            }
            ctx.request_repaint();
        }

        if let Some(drag) = self.drag.as_mut() {
            if let Some(pos) = pointer {
                drag.last_screen = pos;
                let delta = pos - drag.start_screen;
                if delta.length() > 2.5 {
                    drag.moved = true;
                }
            }
            if !primary_down {
                let drag = self.drag.take().expect("drag exists");
                if drag.moved {
                    let dx = f64::from(drag.last_screen.x - drag.start_screen.x)
                        / self.camera.zoom.max(0.05);
                    let dy = f64::from(drag.last_screen.y - drag.start_screen.y)
                        / self.camera.zoom.max(0.05);
                    if dx.abs() > 0.01 || dy.abs() > 0.01 {
                        self.move_selection(dx, dy);
                    }
                }
            }
        }

        if let Some(marquee) = self.marquee.as_mut() {
            if let Some(pos) = pointer {
                marquee.current_screen = pos;
            }
            if !primary_down {
                let marquee = self.marquee.take().expect("marquee exists");
                let start = self.camera.screen_to_world(
                    Point::new(
                        f64::from(marquee.start_screen.x),
                        f64::from(marquee.start_screen.y),
                    ),
                    viewport,
                );
                let end = self.camera.screen_to_world(
                    Point::new(
                        f64::from(marquee.current_screen.x),
                        f64::from(marquee.current_screen.y),
                    ),
                    viewport,
                );
                let selection_rect = Rect::new(
                    start.x.min(end.x),
                    start.y.min(end.y),
                    start.x.max(end.x),
                    start.y.max(end.y),
                );
                for object in self.projection.top_level() {
                    if object
                        .bounds
                        .is_some_and(|bounds| bounds.intersects_rect(selection_rect))
                    {
                        self.selection.insert(object.object.id);
                    }
                }
            }
        }

        let should_finalize = primary_released
            || (self.stroke.is_some()
                && !primary_down
                && !ctx.input(|i| i.viewport().focused.unwrap_or(true)));
        if should_finalize {
            if let Some(mut gesture) = self.stroke.take() {
                if let Some(pos) = pointer {
                    let sample_world = self
                        .camera
                        .screen_to_world(Point::new(f64::from(pos.x), f64::from(pos.y)), viewport);
                    let time = ctx.input(|i| i.time);
                    gesture.builder.push(RawSample::new(
                        sample_world,
                        doodle_platform::pen_pressure(),
                        time,
                    ));
                }
                let (pose, geometry) = gesture.builder.finish();
                if geometry.is_valid() {
                    self.create_stroke(pose, gesture.style, geometry);
                }
            }
            self.marquee = None;
        }

        if response.hovered() {
            let _ = palette;
        }
    }

    fn hit_top(&self, world: Point) -> Option<ObjectId> {
        let tolerance = 5.0 / self.camera.zoom.max(0.05);
        for projected in self.projection.objects.iter().rev() {
            match &projected.object.kind {
                ObjectKind::Stroke(stroke) => {
                    if let Some(clearance) =
                        stroke_hit_clearance(&stroke.geometry, projected.object.pose, world)
                        && clearance <= tolerance
                    {
                        return Some(projected.top_id);
                    }
                }
                ObjectKind::Statement(_) => {
                    if projected
                        .bounds
                        .is_some_and(|bounds| bounds.contains(world))
                    {
                        return Some(projected.top_id);
                    }
                }
                ObjectKind::Group => {}
            }
        }
        None
    }

    fn drag_offsets(&self) -> HashMap<ObjectId, (f64, f64)> {
        let mut offsets = HashMap::new();
        if let Some(drag) = &self.drag
            && drag.moved
        {
            let dx =
                f64::from(drag.last_screen.x - drag.start_screen.x) / self.camera.zoom.max(0.05);
            let dy =
                f64::from(drag.last_screen.y - drag.start_screen.y) / self.camera.zoom.max(0.05);
            for id in &drag.top_ids {
                offsets.insert(*id, (dx, dy));
            }
        }
        offsets
    }

    fn draw_scene(&mut self, ui: &mut egui::Ui, pal: &super::Palette, viewport: Rect) {
        let painter = ui.painter().clone();
        painter.rect_filled(ui.max_rect(), 0.0, to_color32(self.grid.background_rgba));

        let grid_settings = self.grid.sanitized();
        let grid = grid_frame(&self.camera, viewport, grid_settings);
        let base_color = to_color32(grid_settings.line_rgba);
        let alpha = grid_settings.opacity.clamp(0.0, 1.0);
        let grid_color = egui::Color32::from_rgba_unmultiplied(
            base_color.r(),
            base_color.g(),
            base_color.b(),
            (f32::from(base_color.a()) * alpha) as u8,
        );
        let line_stroke = egui::Stroke::new(grid_settings.line_width, grid_color);
        for line in &grid.vertical_lines {
            painter.line_segment(
                [
                    egui::pos2(*line, viewport.min_y as f32),
                    egui::pos2(*line, viewport.max_y as f32),
                ],
                line_stroke,
            );
        }
        for line in &grid.horizontal_lines {
            painter.line_segment(
                [
                    egui::pos2(viewport.min_x as f32, *line),
                    egui::pos2(viewport.max_x as f32, *line),
                ],
                line_stroke,
            );
        }
        for dot in &grid.dots {
            painter.circle_filled(
                egui::pos2(dot[0], dot[1]),
                grid_settings.mark_size,
                grid_color,
            );
        }
        for plus in &grid.pluses {
            let pos = egui::pos2(plus[0], plus[1]);
            let arm = grid_settings.mark_size * 2.0;
            painter.line_segment(
                [pos - egui::vec2(arm, 0.0), pos + egui::vec2(arm, 0.0)],
                line_stroke,
            );
            painter.line_segment(
                [pos - egui::vec2(0.0, arm), pos + egui::vec2(0.0, arm)],
                line_stroke,
            );
        }

        let offsets = self.drag_offsets();
        let viewport_rect = self.canvas_rect;
        let mesh_entries = {
            let projection = &self.projection;
            let camera = &self.camera;
            self.renderer
                .frame_with_offsets(projection, camera, viewport, &offsets)
        };
        let mut meshes: HashMap<ObjectId, egui::Mesh> = HashMap::with_capacity(mesh_entries.len());
        for (id, buffer) in mesh_entries {
            let mut mesh = egui::Mesh::default();
            for vertex in buffer.vertices {
                mesh.colored_vertex(
                    egui::pos2(vertex.pos[0], vertex.pos[1]),
                    egui::Color32::from_rgba_unmultiplied(
                        vertex.color[0],
                        vertex.color[1],
                        vertex.color[2],
                        vertex.color[3],
                    ),
                );
            }
            mesh.indices = buffer.indices;
            meshes.insert(id, mesh);
        }

        for projected in &self.projection.objects {
            let id = projected.object.id;
            if let Some(mesh) = meshes.remove(&id) {
                painter.add(egui::Shape::mesh(mesh));
                continue;
            }
            let ObjectKind::Statement(statement) = &projected.object.kind else {
                continue;
            };
            let offset = offsets
                .get(&projected.top_id)
                .copied()
                .unwrap_or((0.0, 0.0));
            let pose = Pose::new(
                projected.object.pose.x + offset.0,
                projected.object.pose.y + offset.1,
            );
            let rect =
                self.statement_screen_rect(statement.width, statement.height, pose, viewport);
            if !rect.intersects(viewport_rect) {
                continue;
            }
            let fill = to_color32(statement.style.rgba);
            painter.rect(
                rect,
                10.0,
                fill,
                egui::Stroke::new(1.0, pal.border),
                egui::StrokeKind::Inside,
            );
            let luminance = 0.299 * f32::from(fill.r())
                + 0.587 * f32::from(fill.g())
                + 0.114 * f32::from(fill.b());
            let text_color = if luminance > 140.0 {
                egui::Color32::from_rgb(45, 40, 28)
            } else {
                egui::Color32::from_rgb(248, 244, 235)
            };
            let text_rect = rect.shrink(12.0);
            let galley = painter.layout(
                statement.text.clone(),
                egui::FontId::proportional(14.0),
                text_color,
                text_rect.width(),
            );
            painter
                .with_clip_rect(text_rect)
                .galley(text_rect.min, galley, text_color);
            if let Some(mascot) = &self.mascot {
                let mark = egui::Rect::from_min_size(
                    rect.right_top() + egui::vec2(-24.0, 4.0),
                    egui::vec2(20.0, 20.0),
                );
                mascot.paint(&painter, mark, doodle_ai::MascotMood::Neutral, 0);
            }
        }

        if let Some(gesture) = &self.stroke {
            let geometry = gesture.builder.current_geometry();
            let pose = gesture.builder.current_pose();
            let buffer = stroke_mesh(&geometry, pose, gesture.style.rgba, |point| {
                let screen = self.camera.world_to_screen(point, viewport);
                [screen.x as f32, screen.y as f32]
            });
            let mut mesh = egui::Mesh::default();
            for vertex in buffer.vertices {
                mesh.colored_vertex(
                    egui::pos2(vertex.pos[0], vertex.pos[1]),
                    egui::Color32::from_rgba_unmultiplied(
                        vertex.color[0],
                        vertex.color[1],
                        vertex.color[2],
                        vertex.color[3],
                    ),
                );
            }
            mesh.indices = buffer.indices;
            painter.add(egui::Shape::mesh(mesh));
        }

        if self.projection.objects.is_empty() && self.hint_visible {
            let center = ui.max_rect().center();
            painter.text(
                center - egui::vec2(0.0, 10.0),
                egui::Align2::CENTER_CENTER,
                "Draw anywhere",
                egui::FontId::proportional(24.0),
                pal.muted,
            );
            painter.text(
                center + egui::vec2(0.0, 20.0),
                egui::Align2::CENTER_CENTER,
                "B brush · V select · wheel zoom · space-drag pan · Ask Dibby for ideas",
                egui::FontId::proportional(12.0),
                pal.muted,
            );
        }
    }

    fn statement_screen_rect(
        &self,
        width: f64,
        height: f64,
        pose: Pose,
        viewport: Rect,
    ) -> egui::Rect {
        let top_left = self
            .camera
            .world_to_screen(Point::new(pose.x, pose.y), viewport);
        let bottom_right = self
            .camera
            .world_to_screen(Point::new(pose.x + width, pose.y + height), viewport);
        egui::Rect::from_min_max(
            egui::pos2(top_left.x as f32, top_left.y as f32),
            egui::pos2(bottom_right.x as f32, bottom_right.y as f32),
        )
    }

    fn draw_overlays(&self, ui: &mut egui::Ui, pal: &super::Palette, viewport: Rect) {
        let painter = ui.painter().clone();
        let offsets = self.drag_offsets();
        for id in &self.selection {
            let Some(object) = self.projection.get(*id) else {
                continue;
            };
            let Some(bounds) = object.bounds else {
                continue;
            };
            let offset = offsets.get(id).copied().unwrap_or((0.0, 0.0));
            let bounds = bounds.translate(offset.0, offset.1);
            let rect = self.world_rect_to_screen(bounds, viewport);
            painter.rect_stroke(
                rect.expand(2.0),
                4.0,
                egui::Stroke::new(1.6, pal.selection),
                egui::StrokeKind::Outside,
            );
            for corner in [
                rect.left_top(),
                rect.right_top(),
                rect.left_bottom(),
                rect.right_bottom(),
            ] {
                painter.rect_filled(
                    egui::Rect::from_center_size(corner, egui::vec2(5.0, 5.0)),
                    1.5,
                    pal.selection,
                );
            }
        }

        if let Some(marquee) = &self.marquee {
            let rect = egui::Rect::from_two_pos(marquee.start_screen, marquee.current_screen);
            painter.rect(
                rect,
                2.0,
                egui::Color32::from_rgba_unmultiplied(55, 145, 141, 24),
                egui::Stroke::new(1.0, pal.selection),
                egui::StrokeKind::Inside,
            );
        }

        if let Some(pending) = &self.pending_import {
            let translation = placement_translation(&pending.payload, pending.cursor);
            let placement_rect = placement_rect(&pending.payload, translation);
            let screen_rect = self.world_rect_to_screen(placement_rect, viewport);
            let color = if pending.valid {
                egui::Color32::from_rgba_unmultiplied(55, 145, 141, 90)
            } else {
                egui::Color32::from_rgba_unmultiplied(190, 70, 60, 90)
            };
            for object in &pending.payload.objects {
                let ghost_pose =
                    Pose::new(object.pose.x + translation.0, object.pose.y + translation.1);
                match &object.kind {
                    ObjectKind::Stroke(stroke) => {
                        let buffer = stroke_mesh(
                            &stroke.geometry,
                            ghost_pose,
                            [color.r(), color.g(), color.b(), 130],
                            |point| {
                                let screen = self.camera.world_to_screen(point, viewport);
                                [screen.x as f32, screen.y as f32]
                            },
                        );
                        let mut mesh = egui::Mesh::default();
                        for vertex in buffer.vertices {
                            mesh.colored_vertex(
                                egui::pos2(vertex.pos[0], vertex.pos[1]),
                                egui::Color32::from_rgba_unmultiplied(
                                    vertex.color[0],
                                    vertex.color[1],
                                    vertex.color[2],
                                    vertex.color[3],
                                ),
                            );
                        }
                        mesh.indices = buffer.indices;
                        painter.add(egui::Shape::mesh(mesh));
                    }
                    ObjectKind::Statement(statement) => {
                        let rect = self.statement_screen_rect(
                            statement.width,
                            statement.height,
                            ghost_pose,
                            viewport,
                        );
                        painter.rect(
                            rect,
                            8.0,
                            color,
                            egui::Stroke::new(1.0, color),
                            egui::StrokeKind::Inside,
                        );
                    }
                    ObjectKind::Group => {}
                }
            }
            painter.rect_stroke(
                screen_rect,
                6.0,
                egui::Stroke::new(1.5, color),
                egui::StrokeKind::Outside,
            );
            let label = if pending.valid {
                "Click to place · Esc to cancel"
            } else {
                "Area occupied · move the import · Esc to cancel"
            };
            painter.text(
                screen_rect.left_top() + egui::vec2(0.0, -18.0),
                egui::Align2::LEFT_BOTTOM,
                label,
                egui::FontId::proportional(12.0),
                color,
            );
        }
    }

    fn world_rect_to_screen(&self, rect: Rect, viewport: Rect) -> egui::Rect {
        let min = self
            .camera
            .world_to_screen(Point::new(rect.min_x, rect.min_y), viewport);
        let max = self
            .camera
            .world_to_screen(Point::new(rect.max_x, rect.max_y), viewport);
        egui::Rect::from_min_max(
            egui::pos2(min.x as f32, min.y as f32),
            egui::pos2(max.x as f32, max.y as f32),
        )
    }
}
