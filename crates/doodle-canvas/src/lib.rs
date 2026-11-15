//! Pure, deterministic canvas logic: camera transforms, brush processing, outlines, hit tests.
//!
//! Nothing here mutates the shared document or touches a GPU.

pub mod brush;
pub mod camera;
pub mod geometry;

pub use brush::{BrushSettings, RawSample, StrokeBuilder};
pub use camera::Camera;
pub use geometry::{MeshBuffer, MeshVertex, distance_to_polyline, stroke_bounds, stroke_mesh};
