//! Pure, deterministic canvas logic: camera transforms, brush processing, outlines, hit tests.
//!
//! Nothing here mutates the shared document or touches a GPU.

pub mod brush;
pub mod camera;
pub mod geometry;

pub use brush::{BrushSettings, RawSample, StrokeBuilder};
pub use camera::Camera;
pub use geometry::{
    MeshBuffer, MeshVertex, arrow_head_segments, diamond_points, distance_to_polyline,
    ellipse_points, line_endpoints, rectangle_points, shape_bounds, shape_hit_clearance,
    stroke_bounds, stroke_mesh,
};
