use std::collections::HashMap;

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
    // background at all. The same test closes the ink outline, so a loop has no cap bulges where
    // its ends meet. The gap is measured in world units against the world widths: a screen-pixel
    // floor made the same loop close at one zoom and open at another.
    let (first, last) = (
        geometry.points[0],
        geometry.points[geometry.points.len() - 1],
    );
    let closed = stroke_ends_meet(
        [first.x as f32, first.y as f32],
        first.width,
        [last.x as f32, last.y as f32],
        last.width,
    );
    if fill_rgba[3] > 0
        && closed
        && *budget > 0
        && let Some(polygon) = background_polygon(&points)
    {
        *budget -= 1;
        *shape = std::mem::take(shape).fill(polygon, s_color(fill_rgba));
    }
    // Zoomed far out every measured width lands under one screen point. The ribbon outline then
    // paints a sub-pixel band whose edge staircase reads as pixelation. At that size the taper
    // is invisible anyway, so the stroke drops to one antialiased capsule polyline, the same
    // treatment a vector editor gives a scaled-down line; it fades smoothly instead of dithering.
    // The threshold also catches a thin fast stroke: below a couple of points the ribbon's
    // antialiased edges read as a dashed fringe along the line, while a plain stroked polyline
    // stays a clean one-or-two-point line.
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
        let center = simplify_polyline(&center, 0.12);
        if center.len() < 2 {
            return;
        }
        let line =
            (points.iter().map(|(_, width)| *width).sum::<f32>() / points.len() as f32).max(1.0);
        *budget -= 1;
        *shape = std::mem::take(shape).stroke(center, line, color);
        return;
    }
    // One filled outline per stroke, built the way perfect-freehand builds a freedraw outline:
    // a streamline-smoothed centreline, a per-point radius, round caps and corner caps. The
    // outline carries the widths already resolved by the brush, so the slow plateau and the
    // taper survive without a second pressure pass. A very short stroke has no room for the
    // outline's caps, so it stays a capsule.
    //
    // A centreline that crosses itself cannot be outlined as one polygon: the ear clipper sees
    // no ear at the crossing and falls back to a fan, which floods the enclosed lens with ink,
    // while the repeated scan can also leave bead-like holes. Such a stroke is cut at the
    // crossing into simple open pieces, each with its own round caps at the cut. A closure is
    // appended first so a loop that crosses itself still draws its last edge.
    // A centreline that crosses itself cannot be outlined as one polygon: the ear clipper sees
    // no ear at the crossing and falls back to a fan, which floods the enclosed lens with ink.
    // Such a stroke is cut at the crossing into simple open pieces, each with its own round caps
    // at the cut. A closure is appended first so a loop that crosses itself still draws its last
    // edge; a loop without crossings keeps the wrapped outline below.
    let mut closed_scan;
    let scan: &[([f32; 2], f32)] = if closed {
        closed_scan = points.clone();
        closed_scan.push(points[0]);
        &closed_scan
    } else {
        &points
    };
    // A band's outline can cross itself even when its centreline does not: a stroke wider than
    // the turn radius folds its inner edge over the outer one at a sharp corner or a retrace.
    // The ear clipper then fans the polygon and floods the fold with ink. Such an outline falls
    // back to a capsule strip; its round ends overlap the neighbouring segment's body, so an
    // opaque band of one colour shows no seam, and unlike a self-intersecting outline it has no
    // winding region to flood.
    let mut fills: Vec<StrokeFill> = Vec::new();
    let add_outlines = |outlines: Vec<Vec<[f32; 2]>>,
                        fallback: &[([f32; 2], f32)],
                        fills: &mut Vec<StrokeFill>| {
        for outline in outlines {
            if polygon_is_simple(&outline) {
                fills.push(StrokeFill::Polygon(outline));
            } else {
                fills.push(StrokeFill::Capsules(fallback.to_vec()));
            }
        }
    };
    if earliest_crossing(scan).is_none() {
        // A loop keeps its single wrapped outline, so a closed pen ring still has no cap bulges.
        // Its capsule fallback has to bridge the closure explicitly.
        let mut fallback = points.clone();
        if closed && !points.is_empty() {
            fallback.push(points[0]);
        }
        add_outlines(stroke_outline(&points, closed), &fallback, &mut fills);
    } else {
        for piece in split_at_self_intersections(scan) {
            add_outlines(stroke_outline(&piece, false), &piece, &mut fills);
        }
    }
    let vertices: usize = fills
        .iter()
        .map(|fill| match fill {
            StrokeFill::Polygon(polygon) => polygon.len(),
            StrokeFill::Capsules(_) => 0,
        })
        .sum();
    if fills.is_empty()
        || (vertices < 3 && !fills.iter().any(|f| matches!(f, StrokeFill::Capsules(_))))
    {
        if *budget == 0 {
            return;
        }
        let line = points
            .iter()
            .map(|(_, width)| *width)
            .fold(0.0_f32, f32::max)
            .max(1.0);
        let mut center: Vec<[f32; 2]> = Vec::with_capacity(points.len());
        for (at, _) in &points {
            if center.last() != Some(at) {
                center.push(*at);
            }
        }
        if center.len() < 2 {
            return;
        }
        *budget -= 1;
        *shape = std::mem::take(shape).stroke(center, line, color);
        return;
    }
    let margin = points.iter().map(|(_, w)| *w).fold(0.0, f32::max) + 64.0;
    if !points
        .iter()
        .any(|(at, _)| visible(*at, width, height, margin))
    {
        return;
    }
    for fill in fills {
        if *budget == 0 {
            return;
        }
        match fill {
            StrokeFill::Polygon(polygon) => {
                *budget -= 1;
                *shape = std::mem::take(shape).fill(polygon, color);
            }
            StrokeFill::Capsules(path) => {
                // Round-ended capsules overlap far enough that no shared edge shows; a quad strip
                // sharing exact edges left a visible line at every joint.
                for pair in path.windows(2) {
                    if *budget == 0 {
                        return;
                    }
                    let (a, b) = (pair[0].0, pair[1].0);
                    let dx = b[0] - a[0];
                    let dy = b[1] - a[1];
                    if dx * dx + dy * dy < 0.01 {
                        continue;
                    }
                    let line = ((pair[0].1 + pair[1].1) * 0.5).max(0.5);
                    *budget -= 1;
                    *shape = std::mem::take(shape).stroke(vec![a, b], line, color);
                }
            }
        }
    }
}

/// What to paint for one outline: the filled polygon when the clipper can fill it, or the
/// fallback capsule strip when the ribbon folds over itself.
enum StrokeFill {
    Polygon(Vec<[f32; 2]>),
    Capsules(Vec<([f32; 2], f32)>),
}

/// perfect-freehand's constants (`src/constants.ts`), in screen points.
const MIN_RADIUS: f32 = 0.01;
/// Points this close to the end are noise and would wobble the end cap.
const END_NOISE_THRESHOLD: f32 = 3.0;
/// Arc resolution for the two round caps. The start cap matches perfect-freehand's 13; the end
/// uses one more to stay smooth on a wide stroke without tripling the vertex count the way the
/// original 1.5-turn end cap does (that polygon crosses itself, which this renderer's ear
/// clipper cannot fill).
const START_CAP_SEGMENTS: usize = 13;
const END_CAP_SEGMENTS: usize = 14;
/// Arc resolution for the cap drawn at a cusp.
const CORNER_CAP_SEGMENTS: usize = 13;

/// A centreline point in perfect-freehand's shape: `vector` points back along the line toward
/// the previous point, and `running` is the distance travelled so far.
struct OutlineSourcePoint {
    at: [f32; 2],
    vector: [f32; 2],
    running: f32,
    width: f32,
}

fn dot(a: [f32; 2], b: [f32; 2]) -> f32 {
    a[0] * b[0] + a[1] * b[1]
}

/// perfect-freehand's `per`: a quarter turn that leaves the two sides in a consistent order.
fn per(a: [f32; 2]) -> [f32; 2] {
    [a[1], -a[0]]
}

fn lerp(a: [f32; 2], b: [f32; 2], t: f32) -> [f32; 2] {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]
}

fn rotate_around(point: [f32; 2], center: [f32; 2], angle: f32) -> [f32; 2] {
    let (sin, cos) = angle.sin_cos();
    let (x, y) = (point[0] - center[0], point[1] - center[1]);
    [x * cos - y * sin + center[0], x * sin + y * cos + center[1]]
}

/// The centreline points with their back-facing unit vectors and running lengths, as
/// perfect-freehand's `getStrokePoints` produces them. The streamline interpolation itself
/// lives in the brush: the stored geometry already carries the smoothed positions, because it
/// is also where each point's width is resolved.
fn outline_source_points(points: &[([f32; 2], f32)]) -> Vec<OutlineSourcePoint> {
    let mut out: Vec<OutlineSourcePoint> = Vec::with_capacity(points.len());
    let (first, first_width) = points[0];
    out.push(OutlineSourcePoint {
        at: first,
        vector: [1.0, 1.0],
        running: 0.0,
        width: first_width,
    });
    for (at, width) in &points[1..] {
        let previous = out.last().expect("seeded with the first point");
        let dx = at[0] - previous.at[0];
        let dy = at[1] - previous.at[1];
        let distance = (dx * dx + dy * dy).sqrt();
        if distance <= 1e-5 {
            continue;
        }
        out.push(OutlineSourcePoint {
            at: *at,
            // Back toward the previous point, the direction perfect-freehand stores.
            vector: [-dx / distance, -dy / distance],
            running: previous.running + distance,
            width: *width,
        });
    }
    if out.len() > 1 {
        out[0].vector = out[1].vector;
    }
    // At 800% one screen point of hand jitter is a steep direction change relative to the
    // stroke's width, and the two offsets inherit it as a notch in the edge. A 1-2-1 tap over
    // the unit vectors removes that high-frequency wobble; the builder's streamline already
    // handles the low-frequency path, so the outline keeps its shape.
    let raw: Vec<[f32; 2]> = out.iter().map(|point| point.vector).collect();
    for (index, point) in out.iter_mut().enumerate() {
        let mut x = 0.0;
        let mut y = 0.0;
        for (offset, tap) in [(-1i32, 1.0_f32), (0, 2.0), (1, 1.0)] {
            let at = index as i32 + offset;
            if (0..raw.len() as i32).contains(&at) {
                x += raw[at as usize][0] * tap;
                y += raw[at as usize][1] * tap;
            }
        }
        let length = (x * x + y * y).sqrt();
        if length > 1e-4 {
            point.vector = [x / length, y / length];
        }
    }
    // The brush's width follows the speed with a short lag, so the first few points ramp from
    // the starting pressure to the stroke's weight. Offsetting by the raw values puts a 6pt
    // ledge where the round start cap meets the band. A 1-2-3-2-1 tap keeps the taper and the
    // plateau but makes the edges continuous.
    let widths: Vec<f32> = out.iter().map(|point| point.width).collect();
    for (index, point) in out.iter_mut().enumerate() {
        let mut sum = 0.0;
        let mut weight = 0.0;
        for (offset, tap) in [(-2i32, 1.0_f32), (-1, 2.0), (0, 3.0), (1, 2.0), (2, 1.0)] {
            let at = index as i32 + offset;
            if (0..widths.len() as i32).contains(&at) {
                sum += widths[at as usize] * tap;
                weight += tap;
            }
        }
        point.width = sum / weight;
    }
    out
}

/// The outline of a stroke, in perfect-freehand's winding order: left side from start to end,
/// round end cap, right side back, round start cap. A closed stroke (a pen loop) joins the two
/// sides without caps, because two caps meeting at the same point would bulge, and comes back as
/// overlapping plain strips (see `closed_band_sections`).
fn stroke_outline(points: &[([f32; 2], f32)], closed: bool) -> Vec<Vec<[f32; 2]>> {
    let path = outline_source_points(points);
    if path.len() < 2 {
        return Vec::new();
    }
    let total = path[path.len() - 1].running;
    let mut left: Vec<[f32; 2]> = Vec::with_capacity(path.len());
    let mut right: Vec<[f32; 2]> = Vec::with_capacity(path.len());
    let mut previous_vector = path[0].vector;
    let mut previous_left = path[0].at;
    let mut previous_right = path[0].at;
    let mut previous_was_sharp = false;

    for index in 0..path.len() {
        let point = &path[index];
        let is_last = index + 1 == path.len();
        // perfect-freehand's end-noise skip. A closed outline keeps its final points, since the
        // join needs them to meet the start.
        if !is_last && !closed && total - point.running < END_NOISE_THRESHOLD {
            continue;
        }
        let radius = (point.width * 0.5).max(MIN_RADIUS);
        let next_vector = if is_last {
            point.vector
        } else {
            path[index + 1].vector
        };
        let next_dpr = if is_last {
            1.0
        } else {
            dot(point.vector, next_vector)
        };
        let previous_dpr = dot(point.vector, previous_vector);
        let sharp = (previous_dpr < 0.0 && !previous_was_sharp) || (!is_last && next_dpr < 0.0);
        if sharp {
            // A cusp gets its own rounded cap, so the polygon turns there instead of folding the
            // two offset chains through each other.
            let offset = [
                per(previous_vector)[0] * radius,
                per(previous_vector)[1] * radius,
            ];
            let mut cap_left = point.at;
            let mut cap_right = point.at;
            for step in 0..=CORNER_CAP_SEGMENTS {
                let angle = std::f32::consts::PI * step as f32 / CORNER_CAP_SEGMENTS as f32;
                cap_left = rotate_around(
                    [point.at[0] - offset[0], point.at[1] - offset[1]],
                    point.at,
                    angle,
                );
                cap_right = rotate_around(
                    [point.at[0] + offset[0], point.at[1] + offset[1]],
                    point.at,
                    -angle,
                );
                left.push(cap_left);
                right.push(cap_right);
            }
            previous_left = cap_left;
            previous_right = cap_right;
            if !is_last && next_dpr < 0.0 {
                previous_was_sharp = true;
            }
            continue;
        }
        previous_was_sharp = false;
        if is_last {
            let offset = [per(point.vector)[0] * radius, per(point.vector)[1] * radius];
            left.push([point.at[0] - offset[0], point.at[1] - offset[1]]);
            right.push([point.at[0] + offset[0], point.at[1] + offset[1]]);
            continue;
        }
        // The offset direction is the normal of the vector blended between this point and the
        // next one by their dot product, so a bend splits the angle between the two segments.
        // The last band step is the exception: its normal is taken from the final point, so the
        // band arrives at the end cap perpendicular to the cap's radius. Blending there made the
        // band edge and the arc leave each other at a small angle, which showed as a notch where
        // the cap meets the straight side.
        let direction = if !closed && index + 2 == path.len() {
            per(next_vector)
        } else {
            per(lerp(next_vector, point.vector, next_dpr))
        };
        let offset = [direction[0] * radius, direction[1] * radius];
        let next_left = [point.at[0] - offset[0], point.at[1] - offset[1]];
        let next_right = [point.at[0] + offset[0], point.at[1] + offset[1]];
        // perfect-freehand's minimum-distance rule: a vertex is kept only once it has moved far
        // enough from the last saved one (half the stroke width, floored so thin strokes do not
        // explode the polygon, capped so a fat stroke at 800% still follows a loop's curve: with
        // a larger step the chords across a pen circle read as facets). This is what keeps the
        // outline's vertex count proportional to its length instead of to the sample rate. Both
        // sides save together: a closed loop's sections pair left and right by index, and
        // independent decimation let the two sides drift apart, which twisted the section cuts
        // into a wedge.
        let spacing = (point.width * 0.5).clamp(3.0, 12.0);
        let min_distance = spacing * spacing;
        let moved = |a: [f32; 2], b: [f32; 2]| {
            let (dx, dy) = (a[0] - b[0], a[1] - b[1]);
            dx * dx + dy * dy > min_distance
        };
        if index == 0 || moved(previous_left, next_left) || moved(previous_right, next_right) {
            left.push(next_left);
            right.push(next_right);
            previous_left = next_left;
            previous_right = next_right;
        }
        previous_vector = point.vector;
    }

    // A straight edge made of many vertices gets a dotted antialiased fringe from the per-edge
    // rasterisation, and each extra vertex costs the ear clipper. Dropping the vertices that the
    // line between their neighbours already covers leaves the two long edges of a straight
    // stroke as single segments without moving the edge by a tenth of a point. A closed loop
    // keeps both chains whole: its sections pair them by index, and simplifying each side
    // separately would let their vertices diverge.
    if closed {
        return closed_band_sections(&left, &right);
    }
    let left = simplify_polyline(&left, 0.08);
    let right = simplify_polyline(&right, 0.08);
    let mut outline = Vec::with_capacity(left.len() + right.len() + 32);
    let first = path[0].at;
    let last = path[path.len() - 1].at;
    let (Some(&first_left), Some(&first_right)) = (left.first(), right.first()) else {
        return Vec::new();
    };
    let (Some(&last_left), Some(&last_right)) = (left.last(), right.last()) else {
        return Vec::new();
    };
    outline.extend_from_slice(&left);
    // Round end cap, from the last left vertex to the last right vertex around the final
    // centre. perfect-freehand sweeps a fixed 1.5 turns so it stays capped after a sharp end
    // turn, but that boundary crosses itself; this renderer's ear clipper needs a simple
    // polygon. Building the arc between the two real chain ends instead keeps the joins exact,
    // and choosing the sweep that bulges outward gives the same half turn on a normal ending.
    let forward = [
        -path[path.len() - 1].vector[0],
        -path[path.len() - 1].vector[1],
    ];
    outline.extend(cap_arc(
        last_left,
        last_right,
        last,
        forward,
        END_CAP_SEGMENTS,
    ));
    outline.extend(right.iter().rev());
    let backward = path[0].vector;
    outline.extend(cap_arc(
        first_right,
        first_left,
        first,
        backward,
        START_CAP_SEGMENTS,
    ));
    vec![outline]
}

/// Split a closed band into plain overlapping strips. A full turn wrapped into one polygon has to
/// traverse the closing cut twice, which leaves coincident vertices where the ear clipper can see
/// no ear and falls back to a fan that fills the whole loop. Each strip here is an ordinary band;
/// neighbouring strips start several vertices before the previous cut, so every antialiased cut
/// lands on the opaque interior of a later strip and disappears. The last strip runs past the
/// chain's end into the first vertices: that wrap draws the short bridge between the stroke's
/// lagged end and its start, so the loop has no seam.
fn closed_band_sections(left: &[[f32; 2]], right: &[[f32; 2]]) -> Vec<Vec<[f32; 2]>> {
    const SECTION: usize = 36;
    const OVERLAP: usize = 4;
    let length = left.len().min(right.len());
    if length < 3 {
        return Vec::new();
    }
    if length <= 2 * OVERLAP + 2 {
        let mut polygon = Vec::with_capacity(length * 2);
        polygon.extend_from_slice(&left[..length]);
        polygon.extend(right[..length].iter().rev());
        return vec![polygon];
    }
    let mut ranges = Vec::new();
    if length <= SECTION + OVERLAP {
        // Two half turns: the second one wraps past the end, so the seam and both cuts sit
        // inside strips and the bridge is drawn without any repeated vertex.
        let half = length / 2;
        ranges.push((0, half + OVERLAP));
        ranges.push((half, length + OVERLAP));
    } else {
        let mut start = 0;
        loop {
            let end = (start + SECTION).min(length + OVERLAP);
            ranges.push((start, end));
            if end == length + OVERLAP {
                break;
            }
            start = end - OVERLAP;
        }
    }
    ranges
        .into_iter()
        .map(|(start, end)| {
            let mut polygon = Vec::with_capacity((end - start + 1) * 2);
            for position in start..=end {
                polygon.push(left[position % length]);
            }
            for position in (start..=end).rev() {
                polygon.push(right[position % length]);
            }
            polygon
        })
        .collect()
}

/// Ramer-Douglas-Peucker, with the tolerance in screen points. A straight run collapses to its
/// two ends; a curve keeps the vertices needed to stay within the tolerance.
fn simplify_polyline(points: &[[f32; 2]], tolerance: f32) -> Vec<[f32; 2]> {
    if points.len() <= 2 {
        return points.to_vec();
    }
    let last = points.len() - 1;
    let mut keep = vec![false; points.len()];
    keep[0] = true;
    keep[last] = true;
    let tolerance_squared = tolerance * tolerance;
    let distance_squared = |point: [f32; 2], a: [f32; 2], b: [f32; 2]| {
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        let length_squared = dx * dx + dy * dy;
        if length_squared <= 1e-9 {
            let (px, py) = (point[0] - a[0], point[1] - a[1]);
            return px * px + py * py;
        }
        let t =
            (((point[0] - a[0]) * dx + (point[1] - a[1]) * dy) / length_squared).clamp(0.0, 1.0);
        let (px, py) = (point[0] - (a[0] + dx * t), point[1] - (a[1] + dy * t));
        px * px + py * py
    };
    let mut stack = vec![(0usize, last)];
    while let Some((start, end)) = stack.pop() {
        let mut farthest = 0.0_f32;
        let mut split = None;
        for index in start + 1..end {
            let distance = distance_squared(points[index], points[start], points[end]);
            if distance > farthest {
                farthest = distance;
                split = Some(index);
            }
        }
        if let Some(index) = split
            && farthest > tolerance_squared
        {
            keep[index] = true;
            stack.push((start, index));
            stack.push((index, end));
        }
    }
    points
        .iter()
        .zip(keep)
        .filter_map(|(point, keep)| keep.then_some(*point))
        .collect()
}

/// Whether a pen stroke's two ends meet. The test lives in world units: the two round ends
/// touch when the gap between the centreline ends is no more than the two radii added together.
/// Measuring the screen gap with a pixel floor made the same loop closed at one zoom and open at
/// another; comparing world coordinates against the world widths keeps the decision fixed for
/// the stroke. A few-world-point gap therefore stays open, while a loop whose ends truly land on
/// each other closes.
fn stroke_ends_meet(first: [f32; 2], first_width: f32, last: [f32; 2], last_width: f32) -> bool {
    let gap = ((last[0] - first[0]).powi(2) + (last[1] - first[1]).powi(2)).sqrt();
    let reach = (first_width.max(0.0) + last_width.max(0.0)) * 0.5;
    gap <= reach
}

/// The polygon painted by the Background swatch. A loop's centreline arrives with duplicate
/// consecutive samples and, at high zoom, runs of near-coincident points; an ear clipper given
/// those can miss and turn the fill into a fan that spills or vanishes. Dedupe, drop a repeated
/// closing vertex and simplify first; a shape with fewer than three non-collinear points is not
/// a region at all and stays unfilled. The tolerance is in screen points, so the same loop fills
/// at 50%, 100% and 800%.
fn background_polygon(points: &[([f32; 2], f32)]) -> Option<Vec<[f32; 2]>> {
    const MIN_STEP_SQUARED: f32 = 0.01;
    let mut polygon: Vec<[f32; 2]> = Vec::with_capacity(points.len());
    for (at, _) in points {
        let keep = polygon.last().is_none_or(|last| {
            let dx = at[0] - last[0];
            let dy = at[1] - last[1];
            dx * dx + dy * dy > MIN_STEP_SQUARED
        });
        if keep {
            polygon.push(*at);
        }
    }
    if polygon.len() > 1 {
        let first = polygon[0];
        let last = polygon[polygon.len() - 1];
        let dx = last[0] - first[0];
        let dy = last[1] - first[1];
        if dx * dx + dy * dy <= MIN_STEP_SQUARED {
            polygon.pop();
        }
    }
    let polygon = simplify_polyline(&polygon, 0.3);
    if polygon.len() < 3 {
        return None;
    }
    // Shoelace area: zero means every point sits on one line, which has no interior to paint.
    let mut area = 0.0_f32;
    for index in 0..polygon.len() {
        let a = polygon[index];
        let b = polygon[(index + 1) % polygon.len()];
        area += a[0] * b[1] - b[0] * a[1];
    }
    (area.abs() > 0.1).then_some(polygon)
}

/// The earliest self-crossing of the polyline, or `None`. Segments are bucketed in a coarse
/// screen-space grid, so a stroke that never crosses only tests its immediate neighbours and
/// stays linear in the sample count; comparing every pair would cost O(n²) on every frame. Only
/// proper interior crossings count: segments that merely share a vertex, touch end to end or lie
/// on top of each other are left for the outline builder, whose decimation already collapses
/// duplicates.
fn earliest_crossing(points: &[([f32; 2], f32)]) -> Option<(usize, usize, ([f32; 2], f32))> {
    const CELL: f32 = 8.0;
    const MAX_CELLS: i64 = 8_192;
    if points.len() < 4 {
        return None;
    }
    let cell_of = |at: [f32; 2]| ((at[0] / CELL).floor() as i32, (at[1] / CELL).floor() as i32);
    let mut grid: HashMap<(i32, i32), Vec<usize>> = HashMap::new();
    for index in 0..points.len() - 1 {
        let a = points[index].0;
        let b = points[index + 1].0;
        if (b[0] - a[0]).abs() < 1e-6 && (b[1] - a[1]).abs() < 1e-6 {
            continue;
        }
        let (min_x, max_x) = (a[0].min(b[0]), a[0].max(b[0]));
        let (min_y, max_y) = (a[1].min(b[1]), a[1].max(b[1]));
        let (x0, y0) = cell_of([min_x, min_y]);
        let (x1, y1) = cell_of([max_x, max_y]);
        let mut best: Option<(usize, f32, f32)> = None;
        let cells = (i64::from(x1) - i64::from(x0) + 1) * (i64::from(y1) - i64::from(y0) + 1);
        if cells <= MAX_CELLS {
            for x in x0..=x1 {
                for y in y0..=y1 {
                    if let Some(occupants) = grid.get(&(x, y)) {
                        for &other in occupants {
                            // Adjacent segments share a vertex by construction, not by crossing.
                            if other + 1 >= index {
                                continue;
                            }
                            if let Some((t, u)) =
                                segment_crossing(a, b, points[other].0, points[other + 1].0)
                                && best.is_none_or(|(seen, ..)| other < seen)
                            {
                                best = Some((other, t, u));
                            }
                        }
                    }
                }
            }
        } else {
            // A single segment spanning thousands of cells is pathological; fall back to the
            // linear test rather than allocate the bucket range.
            for other in 0..index.saturating_sub(1) {
                if let Some((t, u)) = segment_crossing(a, b, points[other].0, points[other + 1].0)
                    && best.is_none_or(|(seen, ..)| other < seen)
                {
                    best = Some((other, t, u));
                }
            }
        }
        if let Some((other, t, u)) = best {
            let at = [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t];
            // The crossing becomes one shared vertex in both pieces; averaging the two segments'
            // interpolated widths keeps the band the same weight whichever side arrives there.
            let arriving = points[other].1 + (points[other + 1].1 - points[other].1) * u;
            let leaving = points[index].1 + (points[index + 1].1 - points[index].1) * t;
            return Some((index, other, (at, (arriving + leaving) * 0.5)));
        }
        for x in x0..=x1 {
            for y in y0..=y1 {
                grid.entry((x, y)).or_default().push(index);
            }
        }
    }
    None
}

/// The parameter along `a`→`b` where it properly crosses `c`→`d`. Proper means both parameters
/// strictly inside the segments: an end-to-end touch is a join, not a crossing, and splitting
/// there would leave an endpoint duplicate that the next scan considers again.
fn segment_crossing(a: [f32; 2], b: [f32; 2], c: [f32; 2], d: [f32; 2]) -> Option<(f32, f32)> {
    // The margin keeps float noise at a shared vertex from reading as a crossing: a cut within
    // a fiftieth of a segment of an existing vertex would only duplicate that vertex and the
    // splitter would find the same crossing again.
    const EDGE: f32 = 0.02;
    let cross = |u: [f32; 2], v: [f32; 2]| u[0] * v[1] - u[1] * v[0];
    let r = [b[0] - a[0], b[1] - a[1]];
    let s = [d[0] - c[0], d[1] - c[1]];
    let denominator = cross(r, s);
    if denominator.abs() < 1e-6 {
        return None;
    }
    let q = [c[0] - a[0], c[1] - a[1]];
    let t = cross(q, s) / denominator;
    let u = cross(q, r) / denominator;
    (t > EDGE && t < 1.0 - EDGE && u > EDGE && u < 1.0 - EDGE).then_some((t, u))
}

/// Cut the polyline at its crossings until every piece is simple. A crossing between a later
/// segment and an earlier one splits the path into two: the loop between the two visits, and the
/// rest of the stroke that skips over that loop. Simply ending one half at the crossing leaves
/// the crossing inside the other half's body, where the outline's cap meets the band at a
/// T-junction and crosses itself again, so both segments are cut at the shared point. The
/// crossing is stored with its interpolated width, so each piece keeps the band's weight through
/// the join. A clean stroke has no crossing and comes back as the single input piece.
fn split_at_self_intersections(points: &[([f32; 2], f32)]) -> Vec<Vec<([f32; 2], f32)>> {
    // A scribble with hundreds of crossings would produce a shape per crossing; cap the work at
    // a count that still covers any plausible hand stroke and leave the rest unsplit.
    const MAX_PIECES: usize = 128;
    let mut pieces = Vec::new();
    let mut pending = vec![points.to_vec()];
    while let Some(piece) = pending.pop() {
        if pieces.len() + pending.len() >= MAX_PIECES {
            pieces.push(piece);
            continue;
        }
        match earliest_crossing(&piece) {
            Some((index, other, crossing)) => {
                // The loop: enter at the crossing, walk the path up to where it was first
                // crossed, and come back. Its endpoints are the same point, which the open
                // outline closes with two coincident round caps.
                let mut loop_piece = Vec::with_capacity(index - other + 2);
                loop_piece.push(crossing);
                loop_piece.extend_from_slice(&piece[other + 1..=index]);
                loop_piece.push(crossing);
                // The rest: the path before the first visit, the crossing, then the path after
                // the second visit.
                let mut outside = Vec::with_capacity(piece.len() - (index - other) + 1);
                outside.extend_from_slice(&piece[..=other]);
                outside.push(crossing);
                outside.extend_from_slice(&piece[index + 1..]);
                pending.push(loop_piece);
                pending.push(outside);
            }
            None => pieces.push(piece),
        }
    }
    pieces
}

/// Whether a closed polygon has no crossing edges. The ear clipper needs this: its fallback fan
/// paints a non-simple outline as a solid blob, which on a ribbon means the fold is flooded with
/// stroke colour. Edges are bucketed in the same coarse grid as the centreline scan, so a clean
/// outline costs linear time; only real edge crossings are reported, since adjacent edges share
/// an endpoint by construction.
fn polygon_is_simple(points: &[[f32; 2]]) -> bool {
    const CELL: f32 = 8.0;
    const MAX_CELLS: i64 = 8_192;
    let count = points.len();
    if count < 3 {
        return true;
    }
    let cell_of = |at: [f32; 2]| ((at[0] / CELL).floor() as i32, (at[1] / CELL).floor() as i32);
    let mut grid: HashMap<(i32, i32), Vec<usize>> = HashMap::new();
    for index in 0..count {
        let a = points[index];
        let b = points[(index + 1) % count];
        if (b[0] - a[0]).abs() < 1e-6 && (b[1] - a[1]).abs() < 1e-6 {
            continue;
        }
        let (x0, y0) = cell_of([a[0].min(b[0]), a[1].min(b[1])]);
        let (x1, y1) = cell_of([a[0].max(b[0]), a[1].max(b[1])]);
        let cells = (i64::from(x1) - i64::from(x0) + 1) * (i64::from(y1) - i64::from(y0) + 1);
        let adjacent = |other: usize| (other + 1) % count == index || (index + 1) % count == other;
        if cells <= MAX_CELLS {
            for x in x0..=x1 {
                for y in y0..=y1 {
                    if let Some(occupants) = grid.get(&(x, y)) {
                        for &other in occupants {
                            if adjacent(other) {
                                continue;
                            }
                            if segment_crossing(a, b, points[other], points[(other + 1) % count])
                                .is_some()
                            {
                                return false;
                            }
                        }
                    }
                }
            }
        } else {
            for other in 0..index {
                if adjacent(other) {
                    continue;
                }
                if segment_crossing(a, b, points[other], points[(other + 1) % count]).is_some() {
                    return false;
                }
            }
        }
        for x in x0..=x1 {
            for y in y0..=y1 {
                grid.entry((x, y)).or_default().push(index);
            }
        }
    }
    true
}

/// The inside vertices of a round cap arc from `from` to `to` around `center`, bulging toward
/// `outward`. Both endpoints already sit on their side's chain, so the arc excludes them; the
/// radius is interpolated between the two because a decimated side may save a vertex from an
/// earlier point than the cap's centre.
fn cap_arc(
    from: [f32; 2],
    to: [f32; 2],
    center: [f32; 2],
    outward: [f32; 2],
    segments: usize,
) -> Vec<[f32; 2]> {
    let angle_of = |at: [f32; 2]| (at[1] - center[1]).atan2(at[0] - center[0]);
    let start_angle = angle_of(from);
    let mut sweep = (angle_of(to) - start_angle).rem_euclid(std::f32::consts::TAU);
    // Of the two ways round, take the one whose middle faces away from the stroke. The usual
    // case is a half turn and this only matters when the end vector is not perpendicular.
    let faces_out = |sweep: f32| {
        let middle = start_angle + sweep * 0.5;
        middle.cos() * outward[0] + middle.sin() * outward[1]
    };
    if faces_out(sweep - std::f32::consts::TAU) > faces_out(sweep) {
        sweep -= std::f32::consts::TAU;
    }
    let start_radius = {
        let (dx, dy) = (from[0] - center[0], from[1] - center[1]);
        (dx * dx + dy * dy).sqrt()
    };
    let end_radius = {
        let (dx, dy) = (to[0] - center[0], to[1] - center[1]);
        (dx * dx + dy * dy).sqrt()
    };
    (1..segments)
        .map(|step| {
            let t = step as f32 / segments as f32;
            let angle = start_angle + sweep * t;
            let radius = start_radius + (end_radius - start_radius) * t;
            [
                center[0] + angle.cos() * radius,
                center[1] + angle.sin() * radius,
            ]
        })
        .collect()
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
        // vertex must sit on the 29pt inner or 61pt outer edge, or on a cap arc between them.
        // A notch or lobe here is what reads as a lumpy stroke edge on a wide pen.
        let points: Vec<([f32; 2], f32)> = (0..=150)
            .map(|index| {
                let angle = std::f32::consts::TAU * index as f32 / 150.0;
                ([45.0 * angle.cos(), 45.0 * angle.sin()], 32.0)
            })
            .collect();
        let sections = stroke_outline(&points, true);
        // A long loop is split into band sections so the ear clipper never sees the wrapped
        // ring; at this width the circle is short enough to stay one section.
        assert!(!sections.is_empty());
        assert!(
            sections.iter().all(|section| section.len() < 100),
            "a loop section kept too many vertices"
        );
        let outline: Vec<[f32; 2]> = sections.iter().flatten().copied().collect();
        assert!(
            outline.len() >= 20,
            "outline collapsed to {}",
            outline.len()
        );
        let radius = |at: [f32; 2]| (at[0] * at[0] + at[1] * at[1]).sqrt();
        let outer = outline.iter().map(|at| radius(*at)).fold(0.0_f32, f32::max);
        let inner = outline
            .iter()
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
        // Every vertex is on one of the two edges or inside the band; none may fold past the
        // centreline, which is what a fan triangulation of a crossed outline produced.
        for at in &outline {
            let r = radius(*at);
            assert!(
                (29.0..=61.0).contains(&r),
                "outline vertex at radius {r} left the band"
            );
        }
    }

    #[test]
    fn a_long_closed_loop_is_split_into_overlapping_strips() {
        // The clipper cannot fill a wrapped annulus: the closing cut repeats two vertices, every
        // ear test then fails and the fallback fan fills the hole. A long loop must therefore
        // arrive as several ordinary strips, each reaching far enough into the next one that its
        // antialiased cut lands on opaque ink. These chains stand in for a decimated loop: the
        // x coordinate encodes the chain position, so a strip's forward half is easy to read
        // back.
        let left: Vec<[f32; 2]> = (0..200).map(|index| [index as f32, 0.0]).collect();
        let right: Vec<[f32; 2]> = (0..200).map(|index| [index as f32, 1.0]).collect();
        let strips = closed_band_sections(&left, &right);
        assert!(strips.len() > 1, "a 200-vertex loop stayed one polygon");

        let mut runs: Vec<Vec<usize>> = Vec::new();
        for strip in &strips {
            let unique = strip
                .iter()
                .map(|at| (at[0] as usize, at[1] as u8))
                .collect::<std::collections::HashSet<_>>();
            assert_eq!(unique.len(), strip.len(), "a strip repeated a vertex");
            let forward: Vec<usize> = strip
                .iter()
                .take_while(|at| at[1] == 0.0)
                .map(|at| at[0] as usize)
                .collect();
            assert!(forward.len() > 1, "a strip carried no forward edge");
            for pair in forward.windows(2) {
                assert_eq!(
                    pair[1],
                    (pair[0] + 1) % 200,
                    "a strip jumped along the chain: {pair:?}"
                );
            }
            runs.push(forward);
        }
        // Every chain position is drawn, and each cut (a strip start) is covered by the strip
        // before it.
        let mut covered = [false; 200];
        for run in &runs {
            for position in run {
                covered[*position] = true;
            }
        }
        assert!(
            covered.iter().all(|seen| *seen),
            "a loop position was left out"
        );
        for pair in runs.windows(2) {
            let (previous, next) = (&pair[0], &pair[1]);
            assert!(
                next[0] >= previous[0] && next[0] < previous[0] + previous.len(),
                "strip starting at {} left its cut outside the previous strip",
                next[0]
            );
        }
    }

    #[test]
    fn a_crossing_stroke_splits_into_simple_pieces() {
        // An X written as one stroke crosses at its centre. Left whole, the outline polygon is
        // non-simple: the clipper fans and the enclosed lens floods with ink. The splitter must
        // return simple halves, each retaining the crossing point and its width.
        let points: Vec<([f32; 2], f32)> = vec![
            ([0.0, 0.0], 4.0),
            ([40.0, 40.0], 4.0),
            ([80.0, 0.0], 4.0),
            ([80.0, 80.0], 4.0),
            ([0.0, 80.0], 4.0),
            ([40.0, 40.0], 4.0),
            ([20.0, 90.0], 4.0),
        ];
        assert!(
            earliest_crossing(&points).is_some(),
            "the X was not detected"
        );
        let pieces = split_at_self_intersections(&points);
        assert!(pieces.len() > 1, "a crossing stroke stayed one piece");
        for piece in &pieces {
            assert!(
                earliest_crossing(piece).is_none(),
                "a split piece still crosses itself"
            );
            assert!(piece.len() >= 2, "a split piece collapsed");
            for (at, width) in piece {
                assert!(at[0].is_finite() && at[1].is_finite());
                assert!((*width - 4.0).abs() < 0.01);
            }
        }
    }

    #[test]
    fn a_straight_or_gentle_stroke_is_one_piece() {
        let line: Vec<([f32; 2], f32)> = (0..=40).map(|i| ([i as f32, 0.0], 4.0)).collect();
        assert!(earliest_crossing(&line).is_none());
        assert_eq!(split_at_self_intersections(&line).len(), 1);
        let curve: Vec<([f32; 2], f32)> = (0..=60)
            .map(|i| {
                let angle = std::f32::consts::TAU * i as f32 / 60.0 * 0.75;
                ([30.0 * angle.cos(), 30.0 * angle.sin()], 4.0)
            })
            .collect();
        assert_eq!(split_at_self_intersections(&curve).len(), 1);
    }

    #[test]
    fn closure_is_measured_in_world_units() {
        // The round ends touch only when the centreline gap is within the two radii. A five
        // world-point gap with a two-point-wide band must stay open at every zoom; the call
        // takes world coordinates, so no screen scale can flip it.
        assert!(!stroke_ends_meet([0.0, 0.0], 2.0, [5.0, 0.0], 2.0));
        assert!(stroke_ends_meet([0.0, 0.0], 2.0, [1.5, 0.0], 2.0));
        assert!(stroke_ends_meet([0.0, 0.0], 2.0, [2.0, 0.0], 2.0));
        assert!(!stroke_ends_meet([0.0, 0.0], 2.0, [2.5, 0.0], 2.0));
    }

    #[test]
    fn the_background_polygon_drops_duplicates_and_degenerate_shapes() {
        // A loop arrives with its start repeated at the end plus a duplicate sample; the fill
        // must remove both. A straight line has no interior and must not paint.
        let square: Vec<([f32; 2], f32)> = vec![
            ([0.0, 0.0], 4.0),
            ([0.0, 0.0], 4.0),
            ([100.0, 0.0], 4.0),
            ([100.0, 100.0], 4.0),
            ([0.0, 100.0], 4.0),
            ([0.0, 0.0], 4.0),
        ];
        let polygon = background_polygon(&square).expect("square should fill");
        assert!(polygon.len() >= 4, "fill lost the square's corners");
        for pair in polygon.windows(2) {
            assert!(pair[0] != pair[1], "fill kept a duplicate point");
        }
        let line: Vec<([f32; 2], f32)> = (0..=10).map(|i| ([i as f32 * 10.0, 0.0], 4.0)).collect();
        assert!(background_polygon(&line).is_none(), "a line must not fill");
    }

    #[test]
    fn a_straight_stroke_outline_is_two_edges_and_round_caps() {
        let points: Vec<([f32; 2], f32)> =
            (0..=100).map(|index| ([index as f32, 0.0], 10.0)).collect();
        let outlines = stroke_outline(&points, false);
        assert_eq!(outlines.len(), 1, "an open stroke is one polygon");
        let outline = &outlines[0];
        let min_x = outline.iter().map(|at| at[0]).fold(f32::MAX, f32::min);
        let max_x = outline.iter().map(|at| at[0]).fold(f32::MIN, f32::max);
        let min_y = outline.iter().map(|at| at[1]).fold(f32::MAX, f32::min);
        let max_y = outline.iter().map(|at| at[1]).fold(f32::MIN, f32::max);
        assert!((min_x + 5.0).abs() < 0.6, "left cap at {min_x}");
        assert!((max_x - 105.0).abs() < 0.6, "right cap at {max_x}");
        assert!((min_y + 5.0).abs() < 0.6, "top edge at {min_y}");
        assert!((max_y - 5.0).abs() < 0.6, "bottom edge at {max_y}");
        for at in outline {
            assert!(
                (-5.6..=5.6).contains(&at[1]),
                "outline vertex off the straight band: {at:?}"
            );
        }
        // Each side is a straight run of vertices covering the line, not a single point.
        for edge in [-5.0_f32, 5.0] {
            let spans: Vec<f32> = outline
                .iter()
                .filter(|at| (at[1] - edge).abs() < 0.6)
                .map(|at| at[0])
                .collect();
            let low = spans.iter().copied().fold(f32::MAX, f32::min);
            let high = spans.iter().copied().fold(f32::MIN, f32::max);
            assert!(high - low > 90.0, "edge at {edge} spans only {low}..{high}");
        }
    }
}
