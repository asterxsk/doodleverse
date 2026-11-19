//! Converts immutable document projections and camera state into render-ready meshes.
//!
//! The app owns the GPU surface. This crate is pure and deterministic: same board, camera, and
//! viewport produce the same meshes.

use std::collections::{HashMap, HashSet};

use doodle_canvas::{Camera, MeshBuffer, MeshVertex, stroke_mesh};
use doodle_document::BoardProjection;
use doodle_types::{ObjectId, ObjectKind, Rect, StrokeGeometry, StrokeStyle};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridKind {
    None,
    Dots,
    Pluses,
    Lines,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GridSettings {
    pub kind: GridKind,
    pub spacing: f64,
    pub opacity: f32,
    /// Dot radius or plus-arm half-length in screen points.
    pub mark_size: f32,
    /// Stroke width for line grids in screen points.
    pub line_width: f32,
    pub line_rgba: [u8; 4],
    pub background_rgba: [u8; 4],
}

impl Default for GridSettings {
    fn default() -> Self {
        Self {
            kind: GridKind::Dots,
            spacing: 40.0,
            opacity: 0.5,
            mark_size: 1.25,
            line_width: 1.0,
            line_rgba: [212, 212, 212, 255],
            background_rgba: [255, 255, 255, 255],
        }
    }
}

impl GridSettings {
    #[must_use]
    pub fn sanitized(self) -> Self {
        Self {
            kind: self.kind,
            spacing: if self.spacing.is_finite() {
                self.spacing.clamp(8.0, 512.0)
            } else {
                40.0
            },
            opacity: if self.opacity.is_finite() {
                self.opacity.clamp(0.0, 1.0)
            } else {
                0.5
            },
            mark_size: if self.mark_size.is_finite() {
                self.mark_size.clamp(0.25, 8.0)
            } else {
                1.25
            },
            line_width: if self.line_width.is_finite() {
                self.line_width.clamp(0.25, 8.0)
            } else {
                1.0
            },
            line_rgba: self.line_rgba,
            background_rgba: self.background_rgba,
        }
    }

    #[must_use]
    pub fn with_alpha(self, alpha: f32) -> [u8; 4] {
        let mut rgba = self.line_rgba;
        rgba[3] = (f32::from(rgba[3]) * alpha.clamp(0.0, 1.0)).round() as u8;
        rgba
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct GridFrame {
    pub dots: Vec<[f32; 2]>,
    pub pluses: Vec<[f32; 2]>,
    pub vertical_lines: Vec<f32>,
    pub horizontal_lines: Vec<f32>,
    /// Effective world spacing after level-of-detail adaptation.
    pub effective_spacing: f64,
}

/// Screen-space grid geometry with bounded counts at any zoom level.
#[must_use]
pub fn grid_frame(camera: &Camera, viewport: Rect, settings: GridSettings) -> GridFrame {
    let settings = settings.sanitized();
    let mut frame = GridFrame {
        effective_spacing: settings.spacing,
        ..GridFrame::default()
    };
    if settings.kind == GridKind::None || camera.zoom <= 0.0 {
        return frame;
    }
    let mut spacing = settings.spacing;
    let mut screen_spacing = spacing * camera.zoom;
    let mut guard = 0;
    while screen_spacing < 16.0 && guard < 32 {
        spacing *= 2.0;
        screen_spacing = spacing * camera.zoom;
        guard += 1;
    }
    while screen_spacing > 320.0 && guard < 64 {
        spacing *= 0.5;
        screen_spacing = spacing * camera.zoom;
        guard += 1;
    }
    // A few thousand screen marks at most: more than that reads as a texture, not a grid, and
    // the per-mark work dominates a zoomed-out frame.
    let max_points = 3_000usize;
    let max_lines = 240usize;

    // The grid lives in world space. Adapt its spacing until the visible cell count fits the
    // budget; this keeps the whole viewport covered instead of silently dropping the far edge.
    let mut visible_world = camera.visible_world(viewport);
    loop {
        let columns = ((visible_world.width() / spacing).ceil() as usize).saturating_add(1);
        let rows = ((visible_world.height() / spacing).ceil() as usize).saturating_add(1);
        let over_budget = if settings.kind == GridKind::Lines {
            columns.max(rows) > max_lines
        } else {
            columns.saturating_mul(rows) > max_points
        };
        if !over_budget || guard >= 96 {
            break;
        }
        spacing *= 2.0;
        guard += 1;
    }
    frame.effective_spacing = spacing;
    visible_world = camera.visible_world(viewport);
    let start_x = (visible_world.min_x / spacing).ceil() * spacing;
    let start_y = (visible_world.min_y / spacing).ceil() * spacing;
    let columns = ((visible_world.max_x - start_x) / spacing).floor().max(0.0) as usize;
    let rows = ((visible_world.max_y - start_y) / spacing).floor().max(0.0) as usize;
    let to_screen_x = |world_x: f64| -> f32 {
        camera
            .world_to_screen(doodle_types::Point::new(world_x, 0.0), viewport)
            .x as f32
    };
    let to_screen_y = |world_y: f64| -> f32 {
        camera
            .world_to_screen(doodle_types::Point::new(0.0, world_y), viewport)
            .y as f32
    };
    if settings.kind == GridKind::Lines {
        for column in 0..=columns.min(max_lines) {
            frame
                .vertical_lines
                .push(to_screen_x(start_x + column as f64 * spacing));
        }
        for row in 0..=rows.min(max_lines) {
            frame
                .horizontal_lines
                .push(to_screen_y(start_y + row as f64 * spacing));
        }
        return frame;
    }

    for row in 0..=rows {
        for column in 0..=columns {
            if frame.dots.len() + frame.pluses.len() >= max_points {
                return frame;
            }
            let point = [
                to_screen_x(start_x + column as f64 * spacing),
                to_screen_y(start_y + row as f64 * spacing),
            ];
            if settings.kind == GridKind::Dots {
                frame.dots.push(point);
            } else {
                frame.pluses.push(point);
            }
        }
    }
    frame
}

#[derive(Debug, Clone, PartialEq)]
struct CachedMesh {
    hash: u64,
    local: MeshBuffer,
}

/// Per-stroke mesh cache. Meshes are built once in object-local space and transformed to screen
/// coordinates each frame, so panning and zooming never re-tessellate a stroke.
#[derive(Debug, Default)]
pub struct SceneRenderer {
    cache: HashMap<ObjectId, CachedMesh>,
    visible_objects: usize,
    drawn_strokes: usize,
}

impl SceneRenderer {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.cache.clear();
    }

    /// Returns stroke meshes in screen space keyed by object, in projection order.
    #[must_use]
    pub fn frame(
        &mut self,
        projection: &BoardProjection,
        camera: &Camera,
        viewport: Rect,
    ) -> Vec<(ObjectId, MeshBuffer)> {
        self.frame_with_offsets(projection, camera, viewport, &HashMap::new())
    }

    /// Like [`Self::frame`], but applies local preview offsets without mutating the document.
    #[must_use]
    pub fn frame_with_offsets(
        &mut self,
        projection: &BoardProjection,
        camera: &Camera,
        viewport: Rect,
        offsets: &HashMap<ObjectId, (f64, f64)>,
    ) -> Vec<(ObjectId, MeshBuffer)> {
        let visible = camera
            .visible_world(viewport)
            .expand(64.0 / camera.zoom.max(0.02));
        self.visible_objects = 0;
        self.drawn_strokes = 0;
        let mut live_ids: HashSet<ObjectId> = HashSet::new();
        let mut meshes = Vec::new();

        for projected in &projection.objects {
            let Some(bounds) = projected.bounds else {
                continue;
            };
            let offset = offsets
                .get(&projected.object.id)
                .or_else(|| offsets.get(&projected.top_id))
                .copied()
                .unwrap_or((0.0, 0.0));
            let preview_bounds = Rect::new(
                bounds.min_x + offset.0,
                bounds.min_y + offset.1,
                bounds.max_x + offset.0,
                bounds.max_y + offset.1,
            );
            if !preview_bounds.intersects_rect(visible) {
                continue;
            }
            let ObjectKind::Stroke(stroke) = &projected.object.kind else {
                continue;
            };
            live_ids.insert(projected.object.id);
            self.visible_objects += 1;
            let hash = stroke_hash(&stroke.geometry, stroke.style);
            let entry = self
                .cache
                .entry(projected.object.id)
                .or_insert_with(|| CachedMesh {
                    hash,
                    local: build_local_mesh(&stroke.geometry, stroke.style),
                });
            if entry.hash != hash {
                entry.hash = hash;
                entry.local = build_local_mesh(&stroke.geometry, stroke.style);
            }
            let pose = doodle_types::Pose::new(
                projected.object.pose.x + offset.0,
                projected.object.pose.y + offset.1,
            );
            let mesh = MeshBuffer {
                vertices: entry
                    .local
                    .vertices
                    .iter()
                    .map(|vertex| {
                        let world = doodle_types::Point::new(
                            f64::from(vertex.pos[0]) + pose.x,
                            f64::from(vertex.pos[1]) + pose.y,
                        );
                        let screen = camera.world_to_screen(world, viewport);
                        MeshVertex {
                            pos: [screen.x as f32, screen.y as f32],
                            color: vertex.color,
                        }
                    })
                    .collect(),
                indices: entry.local.indices.clone(),
            };
            meshes.push((projected.object.id, mesh));
            self.drawn_strokes += 1;
        }

        self.cache.retain(|id, _| live_ids.contains(id));
        meshes
    }

    #[must_use]
    pub fn visible_objects(&self) -> usize {
        self.visible_objects
    }

    #[must_use]
    pub fn drawn_strokes(&self) -> usize {
        self.drawn_strokes
    }

    #[must_use]
    pub fn cached_meshes(&self) -> usize {
        self.cache.len()
    }
}

fn build_local_mesh(geometry: &StrokeGeometry, style: StrokeStyle) -> MeshBuffer {
    stroke_mesh(geometry, doodle_types::Pose::default(), style.rgba, |p| {
        [p.x as f32, p.y as f32]
    })
}

fn stroke_hash(geometry: &StrokeGeometry, style: StrokeStyle) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    let mut feed = |byte: u8| {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    };
    feed(geometry.version.to_le_bytes()[0]);
    for point in &geometry.points {
        for byte in point.x.to_bits().to_le_bytes() {
            feed(byte);
        }
        for byte in point.y.to_bits().to_le_bytes() {
            feed(byte);
        }
        for byte in point.width.to_le_bytes() {
            feed(byte);
        }
    }
    for byte in style.rgba {
        feed(byte);
    }
    for byte in style.base_width.to_le_bytes() {
        feed(byte);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use doodle_types::{ObjectKind, Pose, StoredObject, StrokeObject, StrokePoint};

    fn projection(strokes: &[(f64, f64)]) -> BoardProjection {
        let objects: Vec<StoredObject> = strokes
            .iter()
            .enumerate()
            .map(|(index, (x, y))| StoredObject {
                id: ObjectId::new(),
                parent: None,
                order_key: index as i64 * 16,
                pose: Pose::new(*x, *y),
                kind: ObjectKind::Stroke(StrokeObject {
                    style: StrokeStyle::default(),
                    geometry: StrokeGeometry::new(vec![
                        StrokePoint {
                            x: 0.0,
                            y: 0.0,
                            width: 6.0,
                        },
                        StrokePoint {
                            x: 10.0,
                            y: 0.0,
                            width: 6.0,
                        },
                    ]),
                    fill_rgba: [0, 0, 0, 0],
                }),
            })
            .collect();
        // Build a projection through the document would add a docs dependency; assemble directly.
        let objects: Vec<doodle_document::ProjectedObject> = objects
            .into_iter()
            .map(|object| doodle_document::ProjectedObject {
                bounds: object.bounds(),
                parent: None,
                depth: 0,
                top_id: object.id,
                object,
            })
            .collect();
        let index = objects
            .iter()
            .enumerate()
            .map(|(index, object)| (object.object.id, index))
            .collect();
        BoardProjection {
            objects,
            revision: 1,
            total_stroke_points: strokes.len() * 2,
            index,
        }
    }

    #[test]
    fn culls_offscreen_strokes_and_caches() {
        let projection = projection(&[(10.0, 10.0), (50_000.0, 50_000.0)]);
        let mut renderer = SceneRenderer::new();
        let camera = Camera::default();
        let viewport = Rect::new(0.0, 0.0, 800.0, 600.0);
        let meshes = renderer.frame(&projection, &camera, viewport);
        assert_eq!(meshes.len(), 1);
        assert_eq!(renderer.cached_meshes(), 1);
        let again = renderer.frame(&projection, &camera, viewport);
        assert_eq!(again.len(), 1);
        assert_eq!(renderer.cached_meshes(), 1);
    }

    #[test]
    fn zoom_lod_keeps_grid_bounded() {
        let camera = Camera::new(doodle_types::Point::default(), 0.02);
        let frame = grid_frame(
            &camera,
            Rect::new(0.0, 0.0, 1920.0, 1080.0),
            GridSettings::default(),
        );
        assert!(frame.dots.len() <= 18_000);
        let camera = Camera::new(doodle_types::Point::default(), 64.0);
        let frame = grid_frame(
            &camera,
            Rect::new(0.0, 0.0, 1920.0, 1080.0),
            GridSettings::default(),
        );
        assert!(frame.dots.len() <= 18_000);
        assert!(frame.effective_spacing * camera.zoom <= 320.0);
    }

    #[test]
    fn grid_pans_with_the_camera() {
        let viewport = Rect::new(0.0, 0.0, 800.0, 600.0);
        let frame = grid_frame(
            &Camera::new(doodle_types::Point::new(100.0, 0.0), 1.0),
            viewport,
            GridSettings::default(),
        );
        // The world origin sits at screen (300, 300) and must be a grid point.
        assert!(
            frame
                .dots
                .iter()
                .any(|dot| (dot[0] - 300.0).abs() < 0.01 && (dot[1] - 300.0).abs() < 0.01),
            "grid follows the world origin after panning"
        );
        let centered = grid_frame(&Camera::default(), viewport, GridSettings::default());
        assert_ne!(frame.dots, centered.dots);
    }

    #[test]
    fn grid_adapts_spacing_instead_of_dropping_the_far_edge() {
        let viewport = Rect::new(0.0, 0.0, 5120.0, 1080.0);
        let frame = grid_frame(
            &Camera::new(doodle_types::Point::default(), 0.25),
            viewport,
            GridSettings::default(),
        );
        assert!(frame.dots.len() <= 18_000);
        let max_x = frame.dots.iter().map(|dot| dot[0]).fold(f32::MIN, f32::max);
        assert!(
            max_x > viewport.max_x as f32 - 40.0,
            "grid should reach the right edge, got {max_x}"
        );
    }

    #[test]
    fn grid_none_is_empty() {
        let frame = grid_frame(
            &Camera::default(),
            Rect::new(0.0, 0.0, 800.0, 600.0),
            GridSettings {
                kind: GridKind::None,
                ..GridSettings::default()
            },
        );
        assert!(frame.dots.is_empty());
        assert!(frame.vertical_lines.is_empty());
    }
}
