use doodle_render::grid_frame;
use doodle_types::{ObjectKind, Point, Pose, Rect, StrokeGeometry};
use rune::{Color, Cursor, El, LineCap, Shape, canvas};

use crate::app::DoodleApp;
use crate::theme::{self, Palette};

const MAX_SEGMENTS: usize = 200_000;

fn s_color(rgba: [u8; 4]) -> Color {
    theme::color(rgba)
}

fn to_screen(s: &DoodleApp, x: f64, y: f64) -> [f32; 2] {
    let screen = s.camera.world_to_screen(Point::new(x, y), s.viewport());
    [screen.x as f32, screen.y as f32]
}

/// A rounded rectangle as a polygon, so a statement's card has the same corner radius the
/// element-based UI uses. `segments` is the arc resolution per corner.
fn rounded_rect_polygon(
    min: [f32; 2],
    max: [f32; 2],
    radius: f32,
    segments: usize,
) -> Vec<[f32; 2]> {
    let (x0, y0, x1, y1) = (min[0], min[1], max[0], max[1]);
    let r = radius.min((x1 - x0).min(y1 - y0) * 0.5).max(0.0);
    let mut points = Vec::with_capacity((segments + 1) * 4);
    let corners = [
        ([x1 - r, y1 - r], 0.0f32),
        ([x0 + r, y1 - r], std::f32::consts::FRAC_PI_2),
        ([x0 + r, y0 + r], std::f32::consts::PI),
        ([x1 - r, y0 + r], std::f32::consts::PI * 1.5),
    ];
    for (center, start) in corners {
        for index in 0..=segments {
            let angle = start + std::f32::consts::FRAC_PI_2 * index as f32 / segments as f32;
            points.push([center[0] + angle.cos() * r, center[1] + angle.sin() * r]);
        }
    }
    points
}

fn circle(points: &mut Vec<[f32; 2]>, center: [f32; 2], radius: f32, segments: usize) {
    let radius = radius.max(0.35);
    for index in 0..segments {
        let angle = std::f32::consts::TAU * index as f32 / segments as f32;
        points.push([
            center[0] + angle.cos() * radius,
            center[1] + angle.sin() * radius,
        ]);
    }
}

fn visible(screen: [f32; 2], width: f32, height: f32, margin: f32) -> bool {
    screen[0] >= -margin
        && screen[1] >= -margin
        && screen[0] <= width + margin
        && screen[1] <= height + margin
}

/// The whole board as one shape: grid, strokes, statements, preview and overlays.
pub fn scene(s: &DoodleApp, pal: &Palette) -> Shape {
    let viewport = s.viewport();
    let (width, height) = (viewport.width() as f32, viewport.height() as f32);
    let zoom = s.zoom() as f32;
    let mut shape = Shape::new();
    let mut budget = MAX_SEGMENTS;

    draw_grid(s, &mut shape, viewport, &mut budget);
    let offsets = s.drag_offsets();

    for projected in &s.projection.objects {
        let offset = offsets
            .get(&projected.top_id)
            .copied()
            .unwrap_or((0.0, 0.0));
        match &projected.object.kind {
            ObjectKind::Stroke(stroke) => {
                draw_stroke(
                    s,
                    &mut shape,
                    &stroke.geometry,
                    Pose::new(
                        projected.object.pose.x + offset.0,
                        projected.object.pose.y + offset.1,
                    ),
                    s_color(stroke.style.rgba),
                    &mut budget,
                );
            }
            ObjectKind::Statement(statement) => {
                let pose = Pose::new(
                    projected.object.pose.x + offset.0,
                    projected.object.pose.y + offset.1,
                );
                let min = to_screen(s, pose.x, pose.y);
                let max = to_screen(s, pose.x + statement.width, pose.y + statement.height);
                if !visible(min, width, height, 64.0) && !visible(max, width, height, 64.0) {
                    continue;
                }
                let radius = 10.0 * zoom;
                let corners = rounded_rect_polygon(min, max, radius, 4);
                shape = shape
                    .fill(corners.clone(), s_color(statement.style.rgba))
                    .stroke_with(corners, true, 1.0, LineCap::Round, pal.border);
            }
            ObjectKind::Group => {}
        }
    }

    if let Some(gesture) = &s.stroke {
        draw_stroke(
            s,
            &mut shape,
            &gesture.builder.current_geometry(),
            gesture.builder.current_pose(),
            s_color(gesture.style.rgba),
            &mut budget,
        );
    }

    draw_selection(s, &mut shape, pal, width, height);
    draw_marquee(s, &mut shape, pal);
    draw_pending_import(s, &mut shape, width, height);
    shape
}

fn draw_grid(s: &DoodleApp, shape: &mut Shape, viewport: Rect, budget: &mut usize) {
    let settings = s.grid.sanitized();
    if settings.kind == doodle_render::GridKind::None {
        return;
    }
    let grid = grid_frame(&s.camera, viewport, settings);
    let alpha = settings.opacity.clamp(0.0, 1.0);
    let color = s_color(settings.line_rgba).alpha(alpha);
    let width = settings.line_width.max(0.1);
    let (w, h) = (viewport.width() as f32, viewport.height() as f32);
    for line in &grid.vertical_lines {
        if *budget == 0 {
            return;
        }
        *budget -= 1;
        *shape = shape
            .clone()
            .line([*line, 0.0], [*line, h], width, LineCap::Butt, color);
    }
    for line in &grid.horizontal_lines {
        if *budget == 0 {
            return;
        }
        *budget -= 1;
        *shape = shape
            .clone()
            .line([0.0, *line], [w, *line], width, LineCap::Butt, color);
    }
    for dot in &grid.dots {
        if *budget == 0 {
            return;
        }
        *budget -= 1;
        let mut points = Vec::with_capacity(10);
        circle(&mut points, [dot[0], dot[1]], settings.mark_size, 10);
        *shape = shape.clone().fill(points, color);
    }
    for plus in &grid.pluses {
        if *budget < 2 {
            return;
        }
        *budget -= 2;
        let arm = settings.mark_size * 2.0;
        let center = [plus[0], plus[1]];
        *shape = shape
            .clone()
            .line(
                [center[0] - arm, center[1]],
                [center[0] + arm, center[1]],
                width,
                LineCap::Round,
                color,
            )
            .line(
                [center[0], center[1] - arm],
                [center[0], center[1] + arm],
                width,
                LineCap::Round,
                color,
            );
    }
}

fn draw_stroke(
    s: &DoodleApp,
    shape: &mut Shape,
    geometry: &StrokeGeometry,
    pose: Pose,
    color: Color,
    budget: &mut usize,
) {
    let zoom = s.zoom() as f32;
    let viewport = s.viewport();
    let (width, height) = (viewport.width() as f32, viewport.height() as f32);
    let points: Vec<([f32; 2], f32)> = geometry
        .points
        .iter()
        .map(|point| {
            (
                to_screen(s, point.x + pose.x, point.y + pose.y),
                (point.width * zoom).max(0.5),
            )
        })
        .collect();
    if points.len() == 1 {
        if *budget == 0 {
            return;
        }
        *budget -= 1;
        let mut circle_points = Vec::with_capacity(14);
        circle(&mut circle_points, points[0].0, points[0].1 * 0.5, 14);
        *shape = shape.clone().fill(circle_points, color);
        return;
    }
    for pair in points.windows(2) {
        if *budget == 0 {
            return;
        }
        let (a, wa) = pair[0];
        let (b, wb) = pair[1];
        let width_here = ((wa + wb) * 0.5).max(0.5);
        if !visible(a, width, height, width_here + 64.0)
            && !visible(b, width, height, width_here + 64.0)
        {
            continue;
        }
        *budget -= 1;
        *shape = shape.clone().line(a, b, width_here, LineCap::Round, color);
    }
}

fn rect_screen(s: &DoodleApp, rect: Rect) -> [f32; 2] {
    let min = to_screen(s, rect.min_x, rect.min_y);
    let max = to_screen(s, rect.max_x, rect.max_y);
    [max[0] - min[0], max[1] - min[1]]
}

fn draw_selection(s: &DoodleApp, shape: &mut Shape, pal: &Palette, width: f32, height: f32) {
    let offsets = s.drag_offsets();
    for id in &s.selection {
        let Some(object) = s.projection.get(*id) else {
            continue;
        };
        let Some(bounds) = object.bounds else {
            continue;
        };
        let offset = offsets.get(id).copied().unwrap_or((0.0, 0.0));
        let bounds = bounds.translate(offset.0, offset.1);
        let min = to_screen(s, bounds.min_x, bounds.min_y);
        let size = rect_screen(s, bounds);
        let max = [min[0] + size[0], min[1] + size[1]];
        if !visible(min, width, height, 128.0) && !visible(max, width, height, 128.0) {
            continue;
        }
        let expand = 2.0;
        let outline = vec![
            [min[0] - expand, min[1] - expand],
            [max[0] + expand, min[1] - expand],
            [max[0] + expand, max[1] + expand],
            [min[0] - expand, max[1] + expand],
        ];
        *shape = shape
            .clone()
            .stroke_with(outline, true, 1.6, LineCap::Round, pal.selection);
        for corner in [
            [min[0] - expand, min[1] - expand],
            [max[0] + expand, min[1] - expand],
            [min[0] - expand, max[1] + expand],
            [max[0] + expand, max[1] + expand],
        ] {
            let square = vec![
                [corner[0] - 2.5, corner[1] - 2.5],
                [corner[0] + 2.5, corner[1] - 2.5],
                [corner[0] + 2.5, corner[1] + 2.5],
                [corner[0] - 2.5, corner[1] + 2.5],
            ];
            *shape = shape.clone().fill(square, pal.selection);
        }
    }
}

fn draw_marquee(s: &DoodleApp, shape: &mut Shape, pal: &Palette) {
    let Some(marquee) = &s.marquee else {
        return;
    };
    let min_x = marquee.start[0].min(marquee.current[0]);
    let min_y = marquee.start[1].min(marquee.current[1]);
    let max_x = marquee.start[0].max(marquee.current[0]);
    let max_y = marquee.start[1].max(marquee.current[1]);
    let corners = vec![
        [min_x, min_y],
        [max_x, min_y],
        [max_x, max_y],
        [min_x, max_y],
    ];
    *shape = shape
        .clone()
        .fill(corners.clone(), pal.selection.alpha(0.10))
        .stroke_with(corners, true, 1.0, LineCap::Round, pal.selection);
}

fn draw_pending_import(s: &DoodleApp, shape: &mut Shape, width: f32, height: f32) {
    let Some(pending) = &s.pending_import else {
        return;
    };
    let translation = crate::placement::placement_translation(&pending.payload, pending.cursor);
    let color = if pending.valid {
        theme::color([55, 145, 141, 255]).alpha(0.35)
    } else {
        theme::color([190, 70, 60, 255]).alpha(0.35)
    };
    let mut budget = 40_000;
    for object in &pending.payload.objects {
        let pose = Pose::new(object.pose.x + translation.0, object.pose.y + translation.1);
        match &object.kind {
            ObjectKind::Stroke(stroke) => {
                draw_stroke(s, shape, &stroke.geometry, pose, color, &mut budget);
            }
            ObjectKind::Statement(statement) => {
                let min = to_screen(s, pose.x, pose.y);
                let max = to_screen(s, pose.x + statement.width, pose.y + statement.height);
                if !visible(min, width, height, 64.0) && !visible(max, width, height, 64.0) {
                    continue;
                }
                *shape = shape.clone().fill(
                    vec![
                        [min[0], min[1]],
                        [max[0], min[1]],
                        [max[0], max[1]],
                        [min[0], max[1]],
                    ],
                    color,
                );
            }
            ObjectKind::Group => {}
        }
    }
    let rect = crate::placement::placement_rect(&pending.payload, translation);
    let min = to_screen(s, rect.min_x, rect.min_y);
    let size = rect_screen(s, rect);
    *shape = shape.clone().stroke_with(
        vec![
            [min[0] - 4.0, min[1] - 4.0],
            [min[0] + size[0] + 4.0, min[1] - 4.0],
            [min[0] + size[0] + 4.0, min[1] + size[1] + 4.0],
            [min[0] - 4.0, min[1] + size[1] + 4.0],
        ],
        true,
        1.5,
        LineCap::Round,
        color,
    );
}

/// The interactive board surface.
pub fn canvas_layer(s: &DoodleApp, pal: &Palette) -> El {
    let cursor = match s.tool {
        crate::app::Tool::Draw => Cursor::Crosshair,
        crate::app::Tool::Select => Cursor::Default,
        crate::app::Tool::Hand => {
            if s.panning {
                Cursor::Grabbing
            } else {
                Cursor::Grab
            }
        }
    };
    let _ = pal;
    canvas(scene(s, pal))
        .key("canvas")
        .absolute(0.0, 0.0)
        .w_full()
        .h_full()
        .fill(s_color(s.grid.background_rgba))
        .cursor(cursor)
        .on_drawn_layout(|s: &mut DoodleApp, rect: [f32; 4]| {
            s.canvas_layout = rect;
        })
        .on_drag(|s: &mut DoodleApp, drag, _| s.canvas_drag(drag))
        .on_pointer_move_local(|s: &mut DoodleApp, event, _| {
            s.pointer_moved(event.x, event.y);
        })
        .on_wheel(|s: &mut DoodleApp, event, _| s.wheel(&event))
        .on_magnify(|s: &mut DoodleApp, event, _| s.magnify(&event))
}

/// Text of every statement, positioned above the canvas.
pub fn statement_texts(s: &DoodleApp, pal: &Palette) -> Vec<El> {
    let viewport = s.viewport();
    let zoom = s.zoom();
    let mut texts = Vec::new();
    for projected in &s.projection.objects {
        let ObjectKind::Statement(statement) = &projected.object.kind else {
            continue;
        };
        let screen = s
            .camera
            .world_to_screen(projected.object.pose.as_point(), viewport);
        let font = (14.0 * zoom).clamp(4.0, 48.0) as f32;
        let width = (statement.width * zoom) as f32;
        let height = (statement.height * zoom) as f32;
        if width < 4.0 || height < 4.0 {
            continue;
        }
        texts.push(
            rune::text(statement.text.clone())
                .key(format!("statement-text-{}", projected.object.id))
                .font(rune::TextStyle::new(font, 500.0))
                .color(pal.statement_ink)
                .absolute(screen.x as f32 + 12.0, screen.y as f32 + 10.0)
                .w(width - 24.0)
                .max_h(height - 20.0)
                .clip_rounded((10.0 * zoom) as f32)
                .wrap()
                .hit(),
        );
    }
    texts
}
