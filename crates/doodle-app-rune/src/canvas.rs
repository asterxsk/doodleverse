use doodle_render::grid_frame;
use doodle_types::{
    FillStyle, ObjectKind, Point, Pose, Rect, ShapeKind, ShapeObject, StatementKind, StrokeGeometry,
};
use rune::{Color, Cursor, CursorImage, El, LineCap, Path, PointerButton, Shape, canvas};

use crate::app::DoodleApp;
use crate::theme::{self, Palette};

use doodle_canvas::{
    arrow_head_segments, diamond_points, ellipse_points, line_endpoints, rectangle_points,
    shape_bounds,
};

const MAX_SEGMENTS: usize = 200_000;

/// Hard cap on hatch strokes per shape. Spacing widens to stay under it, so a huge shape at high
/// zoom keeps full coverage instead of drawing a partial pattern.
const MAX_HATCH_LINES: usize = 240;

/// Screen-point cap on grid marks drawn per frame. `grid_frame` also keeps marks at least 16
/// screen points apart, so this only guards against a sudden viewport or settings change.
const GRID_MARK_CAP: usize = 4_000;

/// Below this zoom the grid fades out entirely; the fade starts at [`GRID_FADE_END`]. At such a
/// distance the grid is noise, and dropping it removes thousands of marks from the frame.
const GRID_HIDE_BELOW: f64 = 0.02;
const GRID_FADE_END: f64 = 0.05;

/// The pen and sticky-note cursors: 64x64 PNGs at 2x, rendered from the same Material Symbols
/// glyphs as the toolbar (`icons::regenerate_cursor_pngs`), dark ink with a white halo. The
/// hotspot sits on the glyph's writing tip and the note's top-left corner.
const PEN_CURSOR: CursorImage =
    CursorImage::png(include_bytes!("../assets/cursors/pen.png"), [7.5, 24.5]);
const STICKY_CURSOR: CursorImage = CursorImage::png(
    include_bytes!("../assets/cursors/sticky-note.png"),
    [6.3, 6.3],
);

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

    draw_grid(s, &mut shape, viewport);
    let offsets = s.drag_offsets();

    for projected in &s.projection.objects {
        // An eraser drag hides what it has touched before the batch delete commits.
        if s.pending_erase(projected.top_id) {
            continue;
        }
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
                    stroke.fill_rgba,
                    &mut budget,
                );
            }
            ObjectKind::Statement(statement) => {
                if statement.kind != StatementKind::Sticky {
                    continue;
                }
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
            ObjectKind::Shape(shape_object) => {
                draw_shape(
                    s,
                    &mut shape,
                    shape_object,
                    Pose::new(
                        projected.object.pose.x + offset.0,
                        projected.object.pose.y + offset.1,
                    ),
                    s_color(shape_object.style.rgba),
                    true,
                    &mut budget,
                );
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
            s.fill_rgba,
            &mut budget,
        );
    }

    // The shape a drag is drawing lives here, not in the document, until the release.
    if let Some((object, pose)) = s.shape_preview() {
        draw_shape(
            s,
            &mut shape,
            &object,
            pose,
            s_color(object.style.rgba),
            true,
            &mut budget,
        );
    }

    draw_selection(s, &mut shape, pal, width, height);
    draw_marquee(s, &mut shape, pal);
    draw_eraser_feedback(s, &mut shape, pal);
    draw_pending_import(s, &mut shape, width, height);
    shape
}

fn draw_grid(s: &DoodleApp, shape: &mut Shape, viewport: Rect) {
    let settings = s.grid.sanitized();
    if settings.kind == doodle_render::GridKind::None {
        return;
    }
    let zoom = s.zoom();
    if zoom <= GRID_HIDE_BELOW {
        return;
    }
    let grid = grid_frame(&s.camera, viewport, settings);
    let fade =
        ((zoom - GRID_HIDE_BELOW) / (GRID_FADE_END - GRID_HIDE_BELOW)).clamp(0.0, 1.0) as f32;
    let alpha = settings.opacity.clamp(0.0, 1.0) * fade;
    if alpha <= 0.0 {
        return;
    }
    let color = s_color(settings.line_rgba).alpha(alpha);
    let width = settings.line_width.max(0.1);
    let (w, h) = (viewport.width() as f32, viewport.height() as f32);

    // One path of independent line subpaths and one `stroke_path` call, so the cost is linear
    // in the number of lines instead of a full `Shape` copy per line.
    let mut lines = Path::new();
    let mut has_lines = false;
    for line in &grid.vertical_lines {
        lines = lines.move_to([*line, 0.0]).line_to([*line, h]);
        has_lines = true;
    }
    for line in &grid.horizontal_lines {
        lines = lines.move_to([0.0, *line]).line_to([w, *line]);
        has_lines = true;
    }
    if has_lines {
        *shape = std::mem::take(shape).stroke_path(&lines, width, LineCap::Butt, color);
    }

    if !grid.dots.is_empty() {
        let mut path = Path::new();
        for dot in grid.dots.iter().take(GRID_MARK_CAP) {
            for index in 0..10 {
                let angle = std::f32::consts::TAU * index as f32 / 10.0;
                let at = [
                    dot[0] + angle.cos() * settings.mark_size,
                    dot[1] + angle.sin() * settings.mark_size,
                ];
                path = if index == 0 {
                    path.move_to(at)
                } else {
                    path.line_to(at)
                };
            }
            path = path.close();
        }
        *shape = std::mem::take(shape).fill_path(&path, color);
    }

    if !grid.pluses.is_empty() {
        let arm = settings.mark_size * 2.0;
        let mut path = Path::new();
        for plus in grid.pluses.iter().take(GRID_MARK_CAP) {
            let center = [plus[0], plus[1]];
            path = path
                .move_to([center[0] - arm, center[1]])
                .line_to([center[0] + arm, center[1]])
                .move_to([center[0], center[1] - arm])
                .line_to([center[0], center[1] + arm]);
        }
        *shape = std::mem::take(shape).stroke_path(&path, width, LineCap::Round, color);
    }
}

fn draw_stroke(
    s: &DoodleApp,
    shape: &mut Shape,
    geometry: &StrokeGeometry,
    pose: Pose,
    color: Color,
    fill_rgba: [u8; 4],
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
        fill_disc(shape, points[0].0, points[0].1, color);
        return;
    }
    // A pen stroke that comes back to its own start encloses a region, and the pen's Background
    // swatch paints it, the way Excalidraw's freedraw does. The centreline is closed implicitly
    // by the fill, so only a stroke whose ends actually meet gets one; an open scribble keeps no
    // background at all.
    if fill_rgba[3] > 0 && *budget > 0 {
        let (start, start_width) = points[0];
        let (end, end_width) = points[points.len() - 1];
        let gap = ((end[0] - start[0]).powi(2) + (end[1] - start[1]).powi(2)).sqrt();
        if gap <= (start_width.max(end_width)).max(4.0) * 1.25 {
            let polygon: Vec<[f32; 2]> = points.iter().map(|(at, _)| *at).collect();
            *budget -= 1;
            *shape = std::mem::take(shape).fill(polygon, s_color(fill_rgba));
        }
    }
    // Zoomed far out every measured width lands under one screen point. The ribbon outline then
    // paints a sub-pixel band whose edge staircase reads as pixelation. At that size the taper
    // is invisible anyway, so the stroke drops to one antialiased capsule polyline, the same
    // treatment a vector editor gives a scaled-down line; it fades smoothly instead of dithering.
    // The threshold also catches a thin fast stroke: below a couple of points the ribbon's
    // antialiased chunk edges read as a dashed fringe along the line, while a plain stroked
    // polyline stays a clean one-or-two-point line.
    if points.iter().all(|(_, width)| *width < 1.6) {
        if *budget == 0 {
            return;
        }
        let mut center: Vec<[f32; 2]> = Vec::with_capacity(points.len());
        for (at, _) in &points {
            let keep = center.last().is_none_or(|last| {
                let dx = at[0] - last[0];
                let dy = at[1] - last[1];
                dx * dx + dy * dy >= 0.12
            });
            if keep {
                center.push(*at);
            }
        }
        let last = points[points.len() - 1].0;
        if center.last() != Some(&last) {
            center.push(last);
        }
        if center.len() < 2 {
            return;
        }
        let line =
            (points.iter().map(|(_, width)| *width).sum::<f32>() / points.len() as f32).max(1.0);
        *budget -= 1;
        *shape = std::mem::take(shape).stroke(center, line, color);
        return;
    }
    // Each run is one filled outline whose edges interpolate the measured widths. Stroking
    // per-segment capsules quantized the width into half-point steps, and the overlapping round
    // caps showed those steps as a dithered ladder along a speed taper. A single outline keeps
    // the edge continuous. Sharp corners split the run so the outline never folds on itself.
    let mut run_start = 0;
    let mut previous_dir: Option<[f32; 2]> = None;
    for index in 1..points.len() {
        let (a, _) = points[index - 1];
        let (b, _) = points[index];
        let Some(dir) = normalized_direction(a, b) else {
            continue;
        };
        let sharp = previous_dir.is_some_and(|prev| prev[0] * dir[0] + prev[1] * dir[1] < 0.25);
        if sharp {
            emit_stroke_run(
                shape,
                &points[run_start..index],
                color,
                width,
                height,
                budget,
            );
            // The split leaves two flat ends at the joint; one disc covers both.
            if *budget > 0 {
                *budget -= 1;
                fill_disc(shape, points[index - 1].0, points[index - 1].1, color);
            }
            run_start = index - 1;
        }
        previous_dir = Some(dir);
        if *budget == 0 {
            return;
        }
    }
    emit_stroke_run(shape, &points[run_start..], color, width, height, budget);
}

fn normalized_direction(a: [f32; 2], b: [f32; 2]) -> Option<[f32; 2]> {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let length = (dx * dx + dy * dy).sqrt();
    (length > 0.000_1).then(|| [dx / length, dy / length])
}

fn fill_disc(shape: &mut Shape, center: [f32; 2], width: f32, color: Color) {
    let radius = width.max(0.5) * 0.5;
    let mut path = Path::new();
    for index in 0..14 {
        let angle = std::f32::consts::TAU * index as f32 / 14.0;
        let at = [
            center[0] + angle.cos() * radius,
            center[1] + angle.sin() * radius,
        ];
        path = if index == 0 {
            path.move_to(at)
        } else {
            path.line_to(at)
        };
    }
    *shape = std::mem::take(shape).fill_path(&path.close(), color);
}

fn emit_stroke_run(
    shape: &mut Shape,
    points: &[([f32; 2], f32)],
    color: Color,
    width: f32,
    height: f32,
    budget: &mut usize,
) {
    if points.len() < 2 || *budget < 3 {
        return;
    }
    let margin = points.iter().map(|(_, w)| *w).fold(0.0, f32::max) + 64.0;
    if !points
        .iter()
        .any(|(at, _)| visible(*at, width, height, margin))
    {
        return;
    }
    let last = points.len() - 1;
    let (left, right, band) = offset_chains(points);
    // The band is filled as short chained runs rather than one outline polygon. A single outline
    // over a long stroke is one ear-clipped polygon with a thousand nearly collinear vertices,
    // and the triangulation there leaves visible lobes along the edge. A 32-point run is small
    // enough to triangulate cleanly. Neighbouring runs overlap by one point so the antialiased
    // end cap of one run sits under the opaque interior of the next instead of showing as a hair
    // line; a full disc at the joint would instead poke out wherever a raw sample width exceeds
    // the smoothed band.
    const RUN: usize = 32;
    const OVERLAP: usize = 1;
    let mut run_start = 0;
    while run_start < last {
        if *budget == 0 {
            return;
        }
        let run_end = (run_start + RUN).min(last);
        let mut polygon: Vec<[f32; 2]> = Vec::with_capacity((run_end - run_start + 1) * 2);
        polygon.extend_from_slice(&left[run_start..=run_end]);
        for index in (run_start..=run_end).rev() {
            polygon.push(right[index]);
        }
        *budget -= 1;
        *shape = std::mem::take(shape).fill(polygon, color);
        run_start = if run_end == last {
            last
        } else {
            run_end - OVERLAP
        };
    }
    // The start gets a round cap from the smoothed band width. The end is left as the band's own
    // cut: a disc there read as an ink blot oozing past the last sample, and the short end taper
    // already closes the stroke instead of chopping it square.
    fill_disc(shape, points[0].0, band[0], color);
}

/// The two offset chains of a stroke run: each point pushed out along the smoothed normal by
/// half the smoothed width. Raw accepted samples carry speed noise, and a width change of half a
/// point every two screen points put a visible notch in the edge of a wide stroke; the same holds
/// for the normals where central differences meet. Triangular taps keep the taper and the curve
/// without the high-frequency wobble.
fn offset_chains(points: &[([f32; 2], f32)]) -> (Vec<[f32; 2]>, Vec<[f32; 2]>, Vec<f32>) {
    let last = points.len() - 1;
    let smoothed_width = |index: usize| -> f32 {
        let mut sum = 0.0;
        let mut weight = 0.0;
        for (offset, tap) in [(-2i32, 1.0_f32), (-1, 2.0), (0, 3.0), (1, 2.0), (2, 1.0)] {
            let at = index as i32 + offset;
            if (0..=last as i32).contains(&at) {
                sum += points[at as usize].1 * tap;
                weight += tap;
            }
        }
        sum / weight
    };
    let raw_tangent = |index: usize| -> Option<[f32; 2]> {
        if index == 0 {
            normalized_direction(points[0].0, points[1].0)
        } else if index == last {
            normalized_direction(points[last - 1].0, points[last].0)
        } else {
            normalized_direction(points[index - 1].0, points[index + 1].0)
                .or_else(|| normalized_direction(points[index - 1].0, points[index].0))
        }
    };
    let mut tangents: Vec<[f32; 2]> = Vec::with_capacity(points.len());
    for index in 0..points.len() {
        tangents.push(raw_tangent(index).unwrap_or([1.0, 0.0]));
    }
    let smoothed: Vec<[f32; 2]> = (0..tangents.len())
        .map(|index| {
            let mut x = 0.0;
            let mut y = 0.0;
            for (offset, tap) in [(-1i32, 1.0_f32), (0, 2.0), (1, 1.0)] {
                let at = index as i32 + offset;
                if (0..tangents.len() as i32).contains(&at) {
                    x += tangents[at as usize][0] * tap;
                    y += tangents[at as usize][1] * tap;
                }
            }
            let length = (x * x + y * y).sqrt();
            if length > 0.000_1 {
                [x / length, y / length]
            } else {
                tangents[index]
            }
        })
        .collect();
    let mut left: Vec<[f32; 2]> = Vec::with_capacity(points.len());
    let mut right: Vec<[f32; 2]> = Vec::with_capacity(points.len());
    let mut band: Vec<f32> = Vec::with_capacity(points.len());
    let mut direction = smoothed[0];
    for (index, (at, _)) in points.iter().enumerate() {
        if index != 0 {
            direction = smoothed[index];
        }
        let normal = [-direction[1], direction[0]];
        let width = smoothed_width(index).max(0.5);
        let half = width * 0.5;
        band.push(width);
        left.push([at[0] + normal[0] * half, at[1] + normal[1] * half]);
        right.push([at[0] - normal[0] * half, at[1] - normal[1] * half]);
    }
    (left, right, band)
}

fn draw_shape(
    s: &DoodleApp,
    shape: &mut Shape,
    object: &ShapeObject,
    pose: Pose,
    color: Color,
    with_fill: bool,
    budget: &mut usize,
) {
    let zoom = s.zoom();
    let viewport = s.viewport();
    let (width, height) = (viewport.width() as f32, viewport.height() as f32);
    let Some(bounds) = shape_bounds(object, pose) else {
        return;
    };
    let min = to_screen(s, bounds.min_x, bounds.min_y);
    let max = to_screen(s, bounds.max_x, bounds.max_y);
    if max[0] < -64.0 || min[0] > width + 64.0 || max[1] < -64.0 || min[1] > height + 64.0 {
        return;
    }
    if with_fill {
        draw_shape_fill(s, shape, object, pose, budget);
    }
    let line_width = (object.style.base_width * zoom as f32).max(0.5);
    let square = Pen {
        color,
        width: line_width,
        cap: LineCap::Butt,
    };
    let round = Pen {
        color,
        width: line_width,
        cap: LineCap::Round,
    };
    match object.shape {
        ShapeKind::Rectangle => {
            draw_polyline(
                s,
                shape,
                &rectangle_points(object, pose),
                true,
                square,
                budget,
            );
        }
        ShapeKind::Diamond => {
            draw_polyline(
                s,
                shape,
                &diamond_points(object, pose),
                true,
                square,
                budget,
            );
        }
        ShapeKind::Ellipse => {
            let segments = ellipse_segments(object, zoom);
            let points = ellipse_points(object, pose, segments);
            draw_polyline(s, shape, &points, true, round, budget);
        }
        ShapeKind::Arrow => {
            let (start, end) = line_endpoints(object, pose);
            draw_polyline(s, shape, &[start, end], false, round, budget);
            for (a, b) in arrow_head_segments(object, pose) {
                draw_polyline(s, shape, &[a, b], false, round, budget);
            }
        }
        ShapeKind::Line => {
            let (start, end) = line_endpoints(object, pose);
            draw_polyline(s, shape, &[start, end], false, round, budget);
        }
    }
}

/// Paints a closed shape's interior before its stroke: a solid polygon, or diagonal hatch lines
/// clipped analytically to the shape. Arrow and line ignore fill, as does any transparent fill.
fn draw_shape_fill(
    s: &DoodleApp,
    shape: &mut Shape,
    object: &ShapeObject,
    pose: Pose,
    budget: &mut usize,
) {
    if object.shape.is_open() || object.fill_rgba[3] == 0 {
        return;
    }
    let zoom = s.zoom();
    let color = s_color(object.fill_rgba);
    let line_width = (object.style.base_width * zoom as f32).max(0.5);
    let spacing = (line_width * 3.0).max(6.0);
    let hatch_width = (line_width * 0.5).clamp(1.0, 3.0);
    match object.shape {
        ShapeKind::Rectangle => {
            let polygon = screen_points(s, &rectangle_points(object, pose));
            match object.fill_style {
                FillStyle::Solid => fill_polygon(shape, &polygon, color),
                FillStyle::Hachure => {
                    hatch_polygon(shape, &polygon, color, spacing, hatch_width, false, budget)
                }
                FillStyle::CrossHatch => {
                    hatch_polygon(shape, &polygon, color, spacing, hatch_width, true, budget)
                }
            }
        }
        ShapeKind::Diamond => {
            let polygon = screen_points(s, &diamond_points(object, pose));
            match object.fill_style {
                FillStyle::Solid => fill_polygon(shape, &polygon, color),
                FillStyle::Hachure => {
                    hatch_polygon(shape, &polygon, color, spacing, hatch_width, false, budget)
                }
                FillStyle::CrossHatch => {
                    hatch_polygon(shape, &polygon, color, spacing, hatch_width, true, budget)
                }
            }
        }
        ShapeKind::Ellipse => {
            let segments = ellipse_segments(object, zoom);
            let polygon = screen_points(s, &ellipse_points(object, pose, segments));
            let min = to_screen(s, pose.x, pose.y);
            let max = to_screen(s, pose.x + object.width, pose.y + object.height);
            let ellipse = [
                (min[0] + max[0]) * 0.5,
                (min[1] + max[1]) * 0.5,
                (max[0] - min[0]).abs() * 0.5,
                (max[1] - min[1]).abs() * 0.5,
            ];
            match object.fill_style {
                FillStyle::Solid => fill_polygon(shape, &polygon, color),
                FillStyle::Hachure => {
                    hatch_ellipse(shape, ellipse, color, spacing, hatch_width, false, budget)
                }
                FillStyle::CrossHatch => {
                    hatch_ellipse(shape, ellipse, color, spacing, hatch_width, true, budget)
                }
            }
        }
        ShapeKind::Arrow | ShapeKind::Line => {}
    }
}

fn screen_points(s: &DoodleApp, points: &[Point]) -> Vec<[f32; 2]> {
    points
        .iter()
        .map(|point| to_screen(s, point.x, point.y))
        .collect()
}

fn fill_polygon(shape: &mut Shape, points: &[[f32; 2]], color: Color) {
    if points.len() < 3 {
        return;
    }
    let mut path = Path::new().move_to(points[0]);
    for point in &points[1..] {
        path = path.line_to(*point);
    }
    *shape = std::mem::take(shape).fill_path(&path.close(), color);
}

/// The diagonal families used by hachure (one) and cross-hatch (both).
fn hatch_directions(cross: bool) -> &'static [[f32; 2]] {
    const DIAGONAL: f32 = std::f32::consts::FRAC_1_SQRT_2;
    if cross {
        &[[DIAGONAL, DIAGONAL], [DIAGONAL, -DIAGONAL]]
    } else {
        &[[DIAGONAL, DIAGONAL]]
    }
}

fn cross(a: [f32; 2], b: [f32; 2]) -> f32 {
    a[0] * b[1] - a[1] * b[0]
}

/// Hatch a convex polygon (rectangle or diamond). Each diagonal is clipped by intersecting it
/// with every edge; the extreme intersections bound the visible segment.
fn hatch_polygon(
    shape: &mut Shape,
    polygon: &[[f32; 2]],
    color: Color,
    spacing: f32,
    hatch_width: f32,
    cross_hatch: bool,
    budget: &mut usize,
) {
    if polygon.len() < 3 {
        return;
    }
    let center = polygon_center(polygon);
    let directions = hatch_directions(cross_hatch);
    let per_family_cap = MAX_HATCH_LINES / directions.len();
    let mut path = Path::new();
    let mut has_lines = false;
    for direction in directions {
        let normal = [-direction[1], direction[0]];
        let radius = polygon
            .iter()
            .map(|point| {
                (normal[0] * (point[0] - center[0]) + normal[1] * (point[1] - center[1])).abs()
            })
            .fold(0.0, f32::max);
        let step = spacing.max((2.0 * radius) / per_family_cap as f32).max(1.0);
        let count = ((2.0 * radius) / step).floor() as usize + 1;
        for index in 0..count.min(per_family_cap + 1) {
            if *budget == 0 {
                break;
            }
            let offset = -radius + index as f32 * step;
            if offset > radius + 1e-3 {
                break;
            }
            let origin = [
                center[0] + normal[0] * offset,
                center[1] + normal[1] * offset,
            ];
            if let Some((start, end)) = polygon_span(polygon, origin, *direction)
                && (end - start).abs() >= 0.25
            {
                let a = [
                    origin[0] + direction[0] * start,
                    origin[1] + direction[1] * start,
                ];
                let b = [
                    origin[0] + direction[0] * end,
                    origin[1] + direction[1] * end,
                ];
                path = path.move_to(a).line_to(b);
                has_lines = true;
                *budget -= 1;
            }
        }
    }
    if has_lines {
        *shape = std::mem::take(shape).stroke_path(&path, hatch_width, LineCap::Butt, color);
    }
}

/// Analytic line/ellipse intersections: `(center_x, center_y, radius_x, radius_y)` in screen
/// space. Offsets run across the ellipse's support in the line normal's direction.
fn hatch_ellipse(
    shape: &mut Shape,
    ellipse: [f32; 4],
    color: Color,
    spacing: f32,
    hatch_width: f32,
    cross_hatch: bool,
    budget: &mut usize,
) {
    let [cx, cy, rx, ry] = ellipse;
    if rx <= 0.0 || ry <= 0.0 {
        return;
    }
    let center = [cx, cy];
    let directions = hatch_directions(cross_hatch);
    let per_family_cap = MAX_HATCH_LINES / directions.len();
    let mut path = Path::new();
    let mut has_lines = false;
    for direction in directions {
        let normal = [-direction[1], direction[0]];
        let radius = ((normal[0] * rx).powi(2) + (normal[1] * ry).powi(2)).sqrt();
        let step = spacing.max((2.0 * radius) / per_family_cap as f32).max(1.0);
        let count = ((2.0 * radius) / step).floor() as usize + 1;
        for index in 0..count.min(per_family_cap + 1) {
            if *budget == 0 {
                break;
            }
            let offset = -radius + index as f32 * step;
            if offset > radius + 1e-3 {
                break;
            }
            let origin = [
                center[0] + normal[0] * offset,
                center[1] + normal[1] * offset,
            ];
            if let Some((start, end)) = ellipse_span(ellipse, origin, *direction)
                && (end - start).abs() >= 0.25
            {
                let a = [
                    origin[0] + direction[0] * start,
                    origin[1] + direction[1] * start,
                ];
                let b = [
                    origin[0] + direction[0] * end,
                    origin[1] + direction[1] * end,
                ];
                path = path.move_to(a).line_to(b);
                has_lines = true;
                *budget -= 1;
            }
        }
    }
    if has_lines {
        *shape = std::mem::take(shape).stroke_path(&path, hatch_width, LineCap::Butt, color);
    }
}

fn polygon_center(polygon: &[[f32; 2]]) -> [f32; 2] {
    let mut x = 0.0;
    let mut y = 0.0;
    for point in polygon {
        x += point[0];
        y += point[1];
    }
    [x / polygon.len() as f32, y / polygon.len() as f32]
}

/// The parameter range of a line (origin + t * direction) inside a convex polygon.
fn polygon_span(polygon: &[[f32; 2]], origin: [f32; 2], direction: [f32; 2]) -> Option<(f32, f32)> {
    let mut min = f32::INFINITY;
    let mut max = f32::NEG_INFINITY;
    for index in 0..polygon.len() {
        let a = polygon[index];
        let b = polygon[(index + 1) % polygon.len()];
        let edge = [b[0] - a[0], b[1] - a[1]];
        let denominator = cross(direction, edge);
        if denominator.abs() < 1e-6 {
            continue;
        }
        let to_a = [a[0] - origin[0], a[1] - origin[1]];
        let t = cross(to_a, edge) / denominator;
        let s = cross(to_a, direction) / denominator;
        if (-1e-4..=1.0 + 1e-4).contains(&s) {
            min = min.min(t);
            max = max.max(t);
        }
    }
    (min <= max).then_some((min, max))
}

/// The parameter range of a line (origin + t * direction) inside an axis-aligned ellipse.
fn ellipse_span(ellipse: [f32; 4], origin: [f32; 2], direction: [f32; 2]) -> Option<(f32, f32)> {
    let [cx, cy, rx, ry] = ellipse;
    let ox = origin[0] - cx;
    let oy = origin[1] - cy;
    let a = direction[0] * direction[0] / (rx * rx) + direction[1] * direction[1] / (ry * ry);
    let b = 2.0 * (ox * direction[0] / (rx * rx) + oy * direction[1] / (ry * ry));
    let c = ox * ox / (rx * rx) + oy * oy / (ry * ry) - 1.0;
    let discriminant = b * b - 4.0 * a * c;
    if discriminant < 0.0 || a <= f32::EPSILON {
        return None;
    }
    let root = discriminant.sqrt();
    Some(((-b - root) / (2.0 * a), (-b + root) / (2.0 * a)))
}

#[derive(Clone, Copy)]
struct Pen {
    color: Color,
    width: f32,
    cap: LineCap,
}

fn draw_polyline(
    s: &DoodleApp,
    shape: &mut Shape,
    points: &[Point],
    closed: bool,
    pen: Pen,
    budget: &mut usize,
) {
    if points.len() < 2 {
        return;
    }
    let viewport = s.viewport();
    let (width, height) = (viewport.width() as f32, viewport.height() as f32);
    let screen: Vec<[f32; 2]> = points
        .iter()
        .map(|point| to_screen(s, point.x, point.y))
        .collect();
    let segments = if closed {
        screen.len()
    } else {
        screen.len() - 1
    };
    let mut path = Path::new();
    let mut has_segments = false;
    for index in 0..segments {
        if *budget == 0 {
            return;
        }
        let a = screen[index];
        let b = screen[(index + 1) % screen.len()];
        if !visible(a, width, height, pen.width + 64.0)
            && !visible(b, width, height, pen.width + 64.0)
        {
            continue;
        }
        *budget -= 1;
        path = path.move_to(a).line_to(b);
        has_segments = true;
    }
    if has_segments {
        *shape = std::mem::take(shape).stroke_path(&path, pen.width, pen.cap, pen.color);
    }
}

fn ellipse_segments(object: &ShapeObject, zoom: f64) -> usize {
    let rx = object.width * 0.5 * zoom;
    let ry = object.height * 0.5 * zoom;
    let perimeter =
        std::f64::consts::PI * (3.0 * (rx + ry) - ((3.0 * rx + ry) * (rx + 3.0 * ry)).sqrt());
    ((perimeter / 8.0).ceil() as usize).clamp(16, 256)
}

fn rect_screen(s: &DoodleApp, rect: Rect) -> [f32; 2] {
    let min = to_screen(s, rect.min_x, rect.min_y);
    let max = to_screen(s, rect.max_x, rect.max_y);
    [max[0] - min[0], max[1] - min[1]]
}

fn draw_selection(s: &DoodleApp, shape: &mut Shape, pal: &Palette, width: f32, height: f32) {
    let offsets = s.drag_offsets();
    for id in &s.selection {
        if s.pending_erase(*id) {
            continue;
        }
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
        *shape =
            std::mem::take(shape).stroke_with(outline, true, 1.6, LineCap::Round, pal.selection);
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
            *shape = std::mem::take(shape).fill(square, pal.selection);
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
    *shape = std::mem::take(shape)
        .fill(corners.clone(), pal.selection.alpha(0.10))
        .stroke_with(corners, true, 1.0, LineCap::Round, pal.selection);
}

/// The eraser's own pointer feedback: a translucent disc at the hit-test radius and a short
/// tail opposite the smoothed direction of travel. Drawn on the canvas, not as chrome, so it
/// stays while the tool is active (including zen mode) and follows idle movement.
fn draw_eraser_feedback(s: &DoodleApp, shape: &mut Shape, pal: &Palette) {
    if s.tool != crate::app::Tool::Eraser || !s.pointer_on_canvas || s.panning {
        return;
    }
    let Some(pointer) = s.pointer_screen else {
        return;
    };
    // The hit test uses a world radius of `ERASER_RADIUS / zoom`, which is this many screen
    // points at any zoom: the visible erase disc.
    let radius = crate::app::ERASER_RADIUS as f32;
    let ink = pal.text;
    if let Some(direction) = s.eraser_trail_dir {
        let end = [
            pointer[0] - direction[0] * 48.0,
            pointer[1] - direction[1] * 48.0,
        ];
        *shape = std::mem::take(shape).stroke_with(
            vec![pointer, end],
            false,
            radius * 2.0,
            LineCap::Round,
            ink.alpha(0.14),
        );
    }
    let mut disc = Vec::with_capacity(24);
    circle(&mut disc, pointer, radius, 24);
    let fill_alpha = if s.erasing.is_some() { 0.38 } else { 0.25 };
    *shape = std::mem::take(shape)
        .fill(disc.clone(), ink.alpha(fill_alpha))
        .stroke_with(disc, true, 1.5, LineCap::Round, ink.alpha(0.5));
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
                draw_stroke(
                    s,
                    shape,
                    &stroke.geometry,
                    pose,
                    color,
                    stroke.fill_rgba,
                    &mut budget,
                );
            }
            ObjectKind::Statement(statement) => {
                let min = to_screen(s, pose.x, pose.y);
                let max = to_screen(s, pose.x + statement.width, pose.y + statement.height);
                if !visible(min, width, height, 64.0) && !visible(max, width, height, 64.0) {
                    continue;
                }
                *shape = std::mem::take(shape).fill(
                    vec![
                        [min[0], min[1]],
                        [max[0], min[1]],
                        [max[0], max[1]],
                        [min[0], max[1]],
                    ],
                    color,
                );
            }
            ObjectKind::Shape(shape_object) => {
                draw_shape(s, shape, shape_object, pose, color, false, &mut budget);
            }
            ObjectKind::Group => {}
        }
    }
    let rect = crate::placement::placement_rect(&pending.payload, translation);
    let min = to_screen(s, rect.min_x, rect.min_y);
    let size = rect_screen(s, rect);
    *shape = std::mem::take(shape).stroke_with(
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
    // A middle-button pan overrides every tool while it is held, like the hand tool's drag.
    let cursor = if s.panning {
        Cursor::Grabbing
    } else {
        match s.tool {
            crate::app::Tool::Select => Cursor::Default,
            crate::app::Tool::Hand => Cursor::Grab,
            crate::app::Tool::Rectangle
            | crate::app::Tool::Diamond
            | crate::app::Tool::Ellipse
            | crate::app::Tool::Arrow
            | crate::app::Tool::Line => Cursor::Crosshair,
            crate::app::Tool::Draw => Cursor::Image(PEN_CURSOR),
            crate::app::Tool::Text => Cursor::Text,
            crate::app::Tool::Sticky => Cursor::Image(STICKY_CURSOR),
            // The canvas paints the eraser disc and tail itself, so the OS pointer is hidden.
            crate::app::Tool::Eraser => Cursor::Hidden,
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
        // The middle button pans like the hand tool whatever tool is active; the canvas's
        // `on_drag` hears it and `Tx::button` says which button drove the gesture.
        .pointer_buttons([PointerButton::Middle])
        .on_drawn_layout(|s: &mut DoodleApp, rect: [f32; 4]| {
            s.canvas_layout = rect;
        })
        .on_hover_change(|s: &mut DoodleApp, inside, _| {
            s.pointer_on_canvas = inside;
            if !inside {
                s.eraser_trail_dir = None;
            }
        })
        .on_drag(|s: &mut DoodleApp, drag, tx| s.canvas_drag(drag, tx.button(), tx))
        .on_pointer_move_local(|s: &mut DoodleApp, event, _| {
            s.pointer_moved(event.x, event.y);
        })
        .on_wheel(|s: &mut DoodleApp, event, _| s.wheel(&event))
        .on_magnify(|s: &mut DoodleApp, event, _| s.magnify(&event))
}

/// The minimap thumbnail: every object as a thin outline or its fill, plus the viewport rect.
/// Coordinates are the minimap panel's local points, from [`DoodleApp::minimap_map`].
pub fn minimap_scene(s: &DoodleApp, pal: &Palette) -> Shape {
    // The panel is a thumbnail, so a bounded sample of the board is all it draws. Both caps
    // keep a frame's work linear in the cap however many objects the board holds.
    const OBJECT_CAP: usize = 400;
    const POINT_CAP: usize = 24_000;
    const POINTS_PER_STROKE: usize = 120;

    let map = s.minimap_map();
    let to = |point: Point| map.to_local(point);
    let mut shape = Shape::new();
    let mut points_left = POINT_CAP;
    for projected in s.projection.objects.iter().take(OBJECT_CAP) {
        if s.pending_erase(projected.top_id) {
            continue;
        }
        let pose = projected.object.pose;
        match &projected.object.kind {
            ObjectKind::Stroke(stroke) => {
                if points_left == 0 {
                    break;
                }
                let color = s_color(stroke.style.rgba);
                let stride = (stroke.geometry.points.len() / POINTS_PER_STROKE).max(1);
                let mut path = Path::new();
                let mut first = true;
                for point in stroke.geometry.points.iter().step_by(stride) {
                    if points_left == 0 {
                        break;
                    }
                    points_left -= 1;
                    let at = to(Point::new(point.x + pose.x, point.y + pose.y));
                    path = if first {
                        first = false;
                        path.move_to(at)
                    } else {
                        path.line_to(at)
                    };
                }
                if !first {
                    shape = shape.stroke_path(&path, 1.0, LineCap::Round, color);
                }
            }
            ObjectKind::Statement(statement) => {
                let min = to(Point::new(pose.x, pose.y));
                let max = to(Point::new(
                    pose.x + statement.width,
                    pose.y + statement.height,
                ));
                let color = if statement.kind == StatementKind::Sticky {
                    s_color(statement.style.rgba)
                } else {
                    pal.muted
                };
                let path = Path::new()
                    .move_to([min[0], min[1]])
                    .line_to([max[0], min[1]])
                    .line_to([max[0], max[1]])
                    .line_to([min[0], max[1]])
                    .close();
                shape = shape.fill_path(&path, color);
            }
            ObjectKind::Shape(object) => {
                let (points, closed): (Vec<[f32; 2]>, bool) = match object.shape {
                    ShapeKind::Rectangle => (
                        rectangle_points(object, pose)
                            .iter()
                            .map(|point| to(*point))
                            .collect(),
                        true,
                    ),
                    ShapeKind::Diamond => (
                        diamond_points(object, pose)
                            .iter()
                            .map(|point| to(*point))
                            .collect(),
                        true,
                    ),
                    ShapeKind::Ellipse => (
                        ellipse_points(object, pose, 24)
                            .iter()
                            .map(|point| to(*point))
                            .collect(),
                        true,
                    ),
                    ShapeKind::Arrow | ShapeKind::Line => {
                        let (start, end) = line_endpoints(object, pose);
                        (vec![to(start), to(end)], false)
                    }
                };
                if points.len() < 2 {
                    continue;
                }
                let mut path = Path::new().move_to(points[0]);
                for point in &points[1..] {
                    path = path.line_to(*point);
                }
                if closed {
                    path = path.close();
                }
                shape = shape.stroke_path(&path, 1.0, LineCap::Round, s_color(object.style.rgba));
            }
            ObjectKind::Group => {}
        }
    }
    let visible = s.camera.visible_world(s.viewport());
    let min = to(Point::new(visible.min_x, visible.min_y));
    let max = to(Point::new(visible.max_x, visible.max_y));
    let path = Path::new()
        .move_to([min[0], min[1]])
        .line_to([max[0], min[1]])
        .line_to([max[0], max[1]])
        .line_to([min[0], max[1]]);
    shape.stroke_path(&path, 1.0, LineCap::Round, pal.selection)
}

/// Text of every statement, positioned above the canvas.
pub fn statement_texts(s: &DoodleApp, pal: &Palette) -> Vec<El> {
    let viewport = s.viewport();
    let zoom = s.zoom();
    let mut texts = Vec::new();
    for projected in &s.projection.objects {
        if s.pending_erase(projected.top_id) {
            continue;
        }
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
        let sticky = statement.kind == StatementKind::Sticky;
        // A sticky note's ink follows its background luminance; a text statement is drawn in
        // the ink it was created with. Boards written before text carried its own colour stored
        // the sticky default on text too, so that exact value keeps the board ink for legibility.
        let ink = if sticky {
            let [r, g, b, _] = statement.style.rgba;
            let luma = 0.2126 * f32::from(r) + 0.7152 * f32::from(g) + 0.0722 * f32::from(b);
            if luma < 120.0 {
                s_color([250, 250, 250, 255])
            } else {
                pal.statement_ink
            }
        } else if statement.style.rgba == [255, 241, 186, 255] {
            pal.statement_ink
        } else {
            s_color(statement.style.rgba)
        };
        let (pad_x, pad_y, radius) = if sticky {
            (12.0, 10.0, (10.0 * zoom) as f32)
        } else {
            (0.0, 0.0, 0.0)
        };
        texts.push(
            rune::text(statement.text.clone())
                .key(format!("statement-text-{}", projected.object.id))
                .font(rune::TextStyle::new(font, 500.0))
                .color(ink)
                .absolute(screen.x as f32 + pad_x, screen.y as f32 + pad_y)
                .w(width - pad_x * 2.0)
                .max_h(height - pad_y * 2.0)
                .clip_rounded(radius)
                .wrap()
                .hit(),
        );
    }
    texts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn polygon_span_clips_a_diagonal_to_the_square() {
        let square = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
        let diagonal = [
            std::f32::consts::FRAC_1_SQRT_2,
            std::f32::consts::FRAC_1_SQRT_2,
        ];
        let (start, end) = polygon_span(&square, [5.0, 5.0], diagonal).expect("diagonal span");
        assert!((start + 7.071_068).abs() < 0.01, "start {start}");
        assert!((end - 7.071_068).abs() < 0.01, "end {end}");
    }

    #[test]
    fn polygon_span_rejects_a_line_outside_the_shape() {
        let square = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
        assert!(polygon_span(&square, [5.0, 50.0], [1.0, 0.0]).is_none());
    }

    #[test]
    fn ellipse_span_solves_the_two_roots() {
        let circle = [0.0, 0.0, 10.0, 10.0];
        let (start, end) = ellipse_span(circle, [0.0, 0.0], [1.0, 0.0]).expect("roots");
        assert!((start + 10.0).abs() < 1e-3);
        assert!((end - 10.0).abs() < 1e-3);
        assert!(ellipse_span(circle, [0.0, 20.0], [1.0, 0.0]).is_none());
    }

    #[test]
    fn a_circle_outline_keeps_a_constant_radius() {
        // A 45pt-radius circle sampled every 3pt with a constant 32pt width: every outline
        // point must sit near radius 45 +/- 16. A notch or lobe here is what reads as a lumpy,
        // mayonnaise-like stroke edge on a wide pen.
        let points: Vec<([f32; 2], f32)> = (0..150)
            .map(|index| {
                let angle = std::f32::consts::TAU * index as f32 / 150.0;
                ([45.0 * angle.cos(), 45.0 * angle.sin()], 32.0)
            })
            .collect();
        let (left, right, band) = offset_chains(&points);
        let radius = |at: [f32; 2]| (at[0] * at[0] + at[1] * at[1]).sqrt();
        // The two chains are the inner and outer edge; which one `left` is depends on the
        // winding, so the test checks the pair of radii rather than a side.
        let outer = left
            .iter()
            .chain(right.iter())
            .map(|at| radius(*at))
            .fold(0.0_f32, f32::max);
        let inner = left
            .iter()
            .chain(right.iter())
            .map(|at| radius(*at))
            .fold(f32::MAX, f32::min);
        assert!(
            (outer - 61.0).abs() < 1.0,
            "outer radius drifted to {outer}"
        );
        assert!(
            (inner - 29.0).abs() < 1.0,
            "inner radius drifted to {inner}"
        );
        for width in band {
            assert!((width - 32.0).abs() < 1.0, "band width drifted to {width}");
        }
    }
}
