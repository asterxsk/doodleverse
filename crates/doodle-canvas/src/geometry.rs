use doodle_types::{Point, Pose, Rect, ShapeKind, ShapeObject, StrokeGeometry};

/// Half-angle of the arrowhead barbs, in radians.
const ARROW_HEAD_ANGLE: f64 = 0.5;
/// Rendered arrowhead length in stroke widths.
const ARROW_HEAD_STROKE_WIDTHS: f64 = 3.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeshVertex {
    pub pos: [f32; 2],
    pub color: [u8; 4],
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct MeshBuffer {
    pub vertices: Vec<MeshVertex>,
    pub indices: Vec<u32>,
}

impl MeshBuffer {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }
}

/// Tests the centerline plus its local half-width. A negative clearance means a hit.
#[must_use]
pub fn stroke_hit_clearance(geometry: &StrokeGeometry, pose: Pose, world: Point) -> Option<f64> {
    let points = &geometry.points;
    let first = points.first()?;
    let mut best = f64::INFINITY;
    if points.len() == 1 {
        let local = Point::new(first.x + pose.x, first.y + pose.y);
        best = world.distance(local) - f64::from(first.width) * 0.5;
        return Some(best);
    }
    for pair in points.windows(2) {
        let a = Point::new(pair[0].x + pose.x, pair[0].y + pose.y);
        let b = Point::new(pair[1].x + pose.x, pair[1].y + pose.y);
        let half = f64::from(pair[0].width.max(pair[1].width)) * 0.5;
        let clearance = point_segment_distance(world, a, b) - half;
        if clearance < best {
            best = clearance;
        }
    }
    Some(best)
}

#[must_use]
pub fn distance_to_polyline(world: Point, a: Point, b: Point) -> f64 {
    point_segment_distance(world, a, b)
}

fn point_segment_distance(p: Point, a: Point, b: Point) -> f64 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let length_sq = dx * dx + dy * dy;
    if length_sq <= f64::EPSILON {
        return p.distance(a);
    }
    let t = (((p.x - a.x) * dx + (p.y - a.y) * dy) / length_sq).clamp(0.0, 1.0);
    let projection = Point::new(a.x + dx * t, a.y + dy * t);
    p.distance(projection)
}

/// Builds a screen-space triangle mesh for one stroke.
///
/// `transform` converts world-space points into final mesh coordinates (the camera lives in the
/// renderer). The output is deterministic for the same geometry, pose, color, and viewport.
#[must_use]
pub fn stroke_mesh<F>(
    geometry: &StrokeGeometry,
    pose: Pose,
    rgba: [u8; 4],
    transform: F,
) -> MeshBuffer
where
    F: Fn(Point) -> [f32; 2],
{
    let mut mesh = MeshBuffer::default();
    let points = &geometry.points;
    if points.is_empty() {
        return mesh;
    }
    let to_world = |p: &doodle_types::StrokePoint| Point::new(p.x + pose.x, p.y + pose.y);
    if points.len() == 1 {
        let center = to_world(&points[0]);
        let radius = f64::from(points[0].width) * 0.5;
        push_circle(&mut mesh, center, radius, rgba, &transform, 16);
        return mesh;
    }

    let mut normals: Vec<(Point, f64)> = Vec::with_capacity(points.len());
    for index in 0..points.len() {
        let current = to_world(&points[index]);
        let prev = if index > 0 {
            Some(to_world(&points[index - 1]))
        } else {
            None
        };
        let next = if index + 1 < points.len() {
            Some(to_world(&points[index + 1]))
        } else {
            None
        };
        let dir = match (prev, next) {
            (Some(p), Some(n)) => Point::new(n.x - p.x, n.y - p.y),
            (None, Some(n)) => Point::new(n.x - current.x, n.y - current.y),
            (Some(p), None) => Point::new(current.x - p.x, current.y - p.y),
            (None, None) => Point::new(1.0, 0.0),
        };
        let len = dir.x.hypot(dir.y).max(1e-6);
        let normal = Point::new(-dir.y / len, dir.x / len);
        normals.push((normal, f64::from(points[index].width) * 0.5));
    }

    mesh.vertices.reserve(points.len() * 2);
    for index in 0..points.len() {
        let center = to_world(&points[index]);
        let (normal, half) = normals[index];
        let left = Point::new(center.x + normal.x * half, center.y + normal.y * half);
        let right = Point::new(center.x - normal.x * half, center.y - normal.y * half);
        mesh.vertices.push(MeshVertex {
            pos: transform(left),
            color: rgba,
        });
        mesh.vertices.push(MeshVertex {
            pos: transform(right),
            color: rgba,
        });
    }
    for index in 0..points.len().saturating_sub(1) {
        let base = (index * 2) as u32;
        mesh.indices
            .extend_from_slice(&[base, base + 1, base + 2, base + 1, base + 3, base + 2]);
    }

    // Round caps hide miter gaps and give tap dots a deliberate shape.
    push_circle(
        &mut mesh,
        to_world(&points[0]),
        f64::from(points[0].width) * 0.5,
        rgba,
        &transform,
        12,
    );
    let last = points.last().expect("non-empty");
    push_circle(
        &mut mesh,
        to_world(last),
        f64::from(last.width) * 0.5,
        rgba,
        &transform,
        12,
    );
    mesh
}

fn push_circle<F>(
    mesh: &mut MeshBuffer,
    center: Point,
    radius: f64,
    rgba: [u8; 4],
    transform: &F,
    segments: usize,
) where
    F: Fn(Point) -> [f32; 2],
{
    if radius <= 0.01 {
        return;
    }
    let base = mesh.vertices.len() as u32;
    mesh.vertices.push(MeshVertex {
        pos: transform(center),
        color: rgba,
    });
    for index in 0..=segments {
        let angle = std::f64::consts::TAU * index as f64 / segments as f64;
        let p = Point::new(
            center.x + angle.cos() * radius,
            center.y + angle.sin() * radius,
        );
        mesh.vertices.push(MeshVertex {
            pos: transform(p),
            color: rgba,
        });
    }
    for index in 0..segments {
        mesh.indices
            .extend_from_slice(&[base, base + 1 + index as u32, base + 2 + index as u32]);
    }
}

/// Tight world-space bounds of a stroke including width.
#[must_use]
pub fn stroke_bounds(geometry: &StrokeGeometry, pose: Pose) -> Option<Rect> {
    geometry
        .local_bounds()
        .map(|rect| rect.translate(pose.x, pose.y))
}

/// Axis-aligned world-space bounds of a shape's box.
#[must_use]
pub fn shape_bounds(shape: &ShapeObject, pose: Pose) -> Option<Rect> {
    shape
        .local_bounds()
        .map(|rect| rect.translate(pose.x, pose.y))
}

/// Corners of a rectangle shape in order around the outline.
#[must_use]
pub fn rectangle_points(shape: &ShapeObject, pose: Pose) -> [Point; 4] {
    let (x0, y0) = (pose.x, pose.y);
    let (x1, y1) = (pose.x + shape.width, pose.y + shape.height);
    [
        Point::new(x0, y0),
        Point::new(x1, y0),
        Point::new(x1, y1),
        Point::new(x0, y1),
    ]
}

/// Vertices of a diamond inscribed in the shape's box.
#[must_use]
pub fn diamond_points(shape: &ShapeObject, pose: Pose) -> [Point; 4] {
    let cx = pose.x + shape.width * 0.5;
    let cy = pose.y + shape.height * 0.5;
    [
        Point::new(cx, pose.y),
        Point::new(pose.x + shape.width, cy),
        Point::new(cx, pose.y + shape.height),
        Point::new(pose.x, cy),
    ]
}

/// Polyline approximation of an ellipse inscribed in the shape's box, without repeating the
/// first point.
#[must_use]
pub fn ellipse_points(shape: &ShapeObject, pose: Pose, segments: usize) -> Vec<Point> {
    let segments = segments.max(8);
    let cx = pose.x + shape.width * 0.5;
    let cy = pose.y + shape.height * 0.5;
    let rx = shape.width * 0.5;
    let ry = shape.height * 0.5;
    (0..segments)
        .map(|index| {
            let angle = std::f64::consts::TAU * index as f64 / segments as f64;
            Point::new(cx + angle.cos() * rx, cy + angle.sin() * ry)
        })
        .collect()
}

/// Start and end of a line or arrow: the box diagonal in the drag direction.
///
/// The pose is the minimum corner and the size is unsigned, so a drag right-to-left or upward is
/// replayed by starting at the opposite corner. With both flags false this is top-left to
/// bottom-right, matching every shape written before the flags existed.
#[must_use]
pub fn line_endpoints(shape: &ShapeObject, pose: Pose) -> (Point, Point) {
    let (left, right) = if shape.flip_x {
        (pose.x + shape.width, pose.x)
    } else {
        (pose.x, pose.x + shape.width)
    };
    let (top, bottom) = if shape.flip_y {
        (pose.y + shape.height, pose.y)
    } else {
        (pose.y, pose.y + shape.height)
    };
    (Point::new(left, top), Point::new(right, bottom))
}

/// The two barbs of an arrowhead, drawn from the end point back along the shaft.
#[must_use]
pub fn arrow_head_segments(shape: &ShapeObject, pose: Pose) -> [(Point, Point); 2] {
    let (start, end) = line_endpoints(shape, pose);
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let length = dx.hypot(dy);
    if length <= f64::EPSILON {
        return [(end, end), (end, end)];
    }
    let head = (f64::from(shape.style.base_width) * ARROW_HEAD_STROKE_WIDTHS)
        .clamp(1.0, (length * 0.45).max(1.0));
    let (sin, cos) = ARROW_HEAD_ANGLE.sin_cos();
    let ux = dx / length;
    let uy = dy / length;
    let back_x = end.x - ux * head * cos;
    let back_y = end.y - uy * head * cos;
    let side_x = -uy * head * sin;
    let side_y = ux * head * sin;
    [
        (end, Point::new(back_x + side_x, back_y + side_y)),
        (end, Point::new(back_x - side_x, back_y - side_y)),
    ]
}

/// Distance from `world` to the shape's drawn outline minus half its stroke width. A negative
/// value means the point is on the outline; the centre of a closed shape is not a hit.
#[must_use]
pub fn shape_hit_clearance(shape: &ShapeObject, pose: Pose, world: Point) -> f64 {
    let half = f64::from(shape.style.base_width) * 0.5;
    let distance = match shape.shape {
        ShapeKind::Rectangle => {
            let corners = rectangle_points(shape, pose);
            closed_outline_distance(world, &corners)
        }
        ShapeKind::Diamond => {
            let corners = diamond_points(shape, pose);
            closed_outline_distance(world, &corners)
        }
        ShapeKind::Ellipse => {
            let cx = pose.x + shape.width * 0.5;
            let cy = pose.y + shape.height * 0.5;
            ellipse_boundary_distance(
                Point::new(cx, cy),
                shape.width * 0.5,
                shape.height * 0.5,
                world,
            )
        }
        ShapeKind::Arrow => {
            let (start, end) = line_endpoints(shape, pose);
            let mut best = point_segment_distance(world, start, end);
            for (a, b) in arrow_head_segments(shape, pose) {
                best = best.min(point_segment_distance(world, a, b));
            }
            best
        }
        ShapeKind::Line => {
            let (start, end) = line_endpoints(shape, pose);
            point_segment_distance(world, start, end)
        }
    };
    distance - half
}

fn closed_outline_distance(world: Point, points: &[Point]) -> f64 {
    let mut best = f64::INFINITY;
    for index in 0..points.len() {
        let a = points[index];
        let b = points[(index + 1) % points.len()];
        best = best.min(point_segment_distance(world, a, b));
    }
    best
}

/// Distance from a point to an ellipse boundary, solved with Newton's method on the parameter.
fn ellipse_boundary_distance(center: Point, rx: f64, ry: f64, world: Point) -> f64 {
    if rx <= 0.0 || ry <= 0.0 {
        return world.distance(center);
    }
    let x = world.x - center.x;
    let y = world.y - center.y;
    if x == 0.0 && y == 0.0 {
        return rx.min(ry);
    }
    let mut theta = (y / ry).atan2(x / rx);
    for _ in 0..16 {
        let (sin, cos) = theta.sin_cos();
        let qx = rx * cos;
        let qy = ry * sin;
        let dqx = -rx * sin;
        let dqy = ry * cos;
        let dx = qx - x;
        let dy = qy - y;
        let value = dx * dqx + dy * dqy;
        let slope = dqx * dqx + dqy * dqy - (dx * qx + dy * qy);
        if slope.abs() < 1e-12 {
            break;
        }
        let step = value / slope;
        theta -= step;
        if !theta.is_finite() || step.abs() < 1e-12 {
            break;
        }
    }
    if !theta.is_finite() {
        return world.distance(center) - rx.min(ry);
    }
    let nearest = Point::new(center.x + rx * theta.cos(), center.y + ry * theta.sin());
    world.distance(nearest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use doodle_types::StrokePoint;

    fn line(geometry: &[(f64, f64, f32)]) -> StrokeGeometry {
        StrokeGeometry::new(
            geometry
                .iter()
                .map(|(x, y, width)| StrokePoint {
                    x: *x,
                    y: *y,
                    width: *width,
                })
                .collect(),
        )
    }

    #[test]
    fn hit_test_respects_width() {
        let geometry = line(&[(0.0, 0.0, 10.0), (100.0, 0.0, 10.0)]);
        let pose = Pose::new(50.0, 50.0);
        let near = stroke_hit_clearance(&geometry, pose, Point::new(100.0, 54.0)).expect("hit");
        assert!(near < 0.0);
        let far = stroke_hit_clearance(&geometry, pose, Point::new(100.0, 80.0)).expect("miss");
        assert!(far > 0.0);
    }

    #[test]
    fn mesh_has_geometry_for_line_and_dot() {
        let transform = |p: Point| [p.x as f32, p.y as f32];
        let geometry = line(&[(0.0, 0.0, 8.0), (40.0, 0.0, 8.0)]);
        let mesh = stroke_mesh(&geometry, Pose::default(), [0, 0, 0, 255], transform);
        assert!(!mesh.is_empty());
        assert!(mesh.vertices.len() >= 4);
        let dot = line(&[(5.0, 5.0, 8.0)]);
        let dot_mesh = stroke_mesh(&dot, Pose::default(), [0, 0, 0, 255], transform);
        assert!(!dot_mesh.is_empty());
    }

    #[test]
    fn bounds_include_pose_and_width() {
        let geometry = line(&[(0.0, 0.0, 10.0), (20.0, 0.0, 10.0)]);
        let bounds = stroke_bounds(&geometry, Pose::new(10.0, 5.0)).expect("bounds");
        assert_eq!(bounds, Rect::new(5.0, 0.0, 35.0, 10.0));
    }

    fn shape(kind: ShapeKind, width: f64, height: f64) -> ShapeObject {
        ShapeObject {
            shape: kind,
            width,
            height,
            style: doodle_types::StrokeStyle::default(),
            flip_x: false,
            flip_y: false,
            fill_rgba: [0, 0, 0, 0],
            fill_style: doodle_types::FillStyle::default(),
        }
    }

    #[test]
    fn line_endpoints_follow_the_drag_direction() {
        let pose = Pose::new(10.0, 20.0);
        let base = shape(ShapeKind::Arrow, 100.0, 60.0);
        let (start, end) = line_endpoints(&base, pose);
        assert_eq!(
            (start, end),
            (Point::new(10.0, 20.0), Point::new(110.0, 80.0))
        );

        let left = ShapeObject {
            flip_x: true,
            ..base
        };
        assert_eq!(
            line_endpoints(&left, pose),
            (Point::new(110.0, 20.0), Point::new(10.0, 80.0))
        );

        let up = ShapeObject {
            flip_y: true,
            ..base
        };
        assert_eq!(
            line_endpoints(&up, pose),
            (Point::new(10.0, 80.0), Point::new(110.0, 20.0))
        );

        let up_left = ShapeObject {
            flip_x: true,
            flip_y: true,
            ..base
        };
        assert_eq!(
            line_endpoints(&up_left, pose),
            (Point::new(110.0, 80.0), Point::new(10.0, 20.0))
        );

        // The barbs hang off the end the drag finished on, whichever direction that was.
        for directed in [&base, &left, &up, &up_left] {
            let (start, end) = line_endpoints(directed, pose);
            let [first, second] = arrow_head_segments(directed, pose);
            assert_eq!(first.0, end);
            assert_eq!(second.0, end);
            let barb = Point::new(
                (first.1.x + second.1.x) * 0.5,
                (first.1.y + second.1.y) * 0.5,
            );
            assert!(barb.distance(end) < barb.distance(start));
        }
    }

    #[test]
    fn horizontal_line_has_zero_extent_and_still_draws() {
        let line = shape(ShapeKind::Line, 140.0, 0.0);
        let pose = Pose::new(5.0, 5.0);
        assert_eq!(
            line_endpoints(&line, pose),
            (Point::new(5.0, 5.0), Point::new(145.0, 5.0))
        );
        let left = ShapeObject {
            flip_x: true,
            ..line
        };
        assert_eq!(
            line_endpoints(&left, pose),
            (Point::new(145.0, 5.0), Point::new(5.0, 5.0))
        );
        assert!(shape_hit_clearance(&line, pose, Point::new(75.0, 5.0)) <= 0.0);

        let vertical = shape(ShapeKind::Line, 0.0, 140.0);
        assert_eq!(
            line_endpoints(&vertical, pose),
            (Point::new(5.0, 5.0), Point::new(5.0, 145.0))
        );
        let vertical_up = ShapeObject {
            flip_y: true,
            ..vertical
        };
        assert_eq!(
            line_endpoints(&vertical_up, pose),
            (Point::new(5.0, 145.0), Point::new(5.0, 5.0))
        );
    }

    #[test]
    fn rectangle_hit_tests_its_outline_only() {
        let rect = shape(ShapeKind::Rectangle, 100.0, 50.0);
        let pose = Pose::new(10.0, 20.0);
        assert!(shape_hit_clearance(&rect, pose, Point::new(10.0, 45.0)) <= 0.0);
        assert!(shape_hit_clearance(&rect, pose, Point::new(110.0, 45.0)) <= 0.0);
        assert!(shape_hit_clearance(&rect, pose, Point::new(60.0, 45.0)) > 0.0);
        assert!(shape_hit_clearance(&rect, pose, Point::new(60.0, 20.0)) <= 0.0);
    }

    #[test]
    fn diamond_hit_tests_its_edges() {
        let diamond = shape(ShapeKind::Diamond, 100.0, 100.0);
        let pose = Pose::new(0.0, 0.0);
        assert!(shape_hit_clearance(&diamond, pose, Point::new(50.0, 0.0)) <= 0.0);
        assert!(shape_hit_clearance(&diamond, pose, Point::new(50.0, 50.0)) > 0.0);
        assert!(shape_hit_clearance(&diamond, pose, Point::new(75.0, 27.0)) <= 2.0);
    }

    #[test]
    fn ellipse_hit_tests_its_curve() {
        let ellipse = shape(ShapeKind::Ellipse, 200.0, 100.0);
        let pose = Pose::new(0.0, 0.0);
        assert!(shape_hit_clearance(&ellipse, pose, Point::new(100.0, 0.0)) <= 0.0);
        assert!(shape_hit_clearance(&ellipse, pose, Point::new(200.0, 50.0)) <= 0.0);
        assert!(shape_hit_clearance(&ellipse, pose, Point::new(100.0, 50.0)) > 0.0);
        assert!(shape_hit_clearance(&ellipse, pose, Point::new(100.0, 70.0)) > 0.0);
        let near = shape_hit_clearance(&ellipse, pose, Point::new(100.0, 2.0));
        assert!(near.abs() < 0.5, "near clearance was {near}");
    }

    #[test]
    fn line_and_arrow_hit_their_segments() {
        let line = shape(ShapeKind::Line, 100.0, 100.0);
        let pose = Pose::new(0.0, 0.0);
        assert!(shape_hit_clearance(&line, pose, Point::new(50.0, 50.0)) <= 0.0);
        assert!(shape_hit_clearance(&line, pose, Point::new(50.0, 0.0)) > 0.0);

        let arrow = shape(ShapeKind::Arrow, 100.0, 100.0);
        let [first, _] = arrow_head_segments(&arrow, pose);
        let barb = Point::new((first.0.x + first.1.x) * 0.5, (first.0.y + first.1.y) * 0.5);
        assert!(shape_hit_clearance(&arrow, pose, barb) <= 0.0);
        assert!(shape_hit_clearance(&arrow, pose, Point::new(95.0, 0.0)) > 0.0);
    }
}
