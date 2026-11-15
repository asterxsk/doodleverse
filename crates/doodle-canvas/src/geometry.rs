use doodle_types::{Point, Pose, Rect, StrokeGeometry};

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
}
