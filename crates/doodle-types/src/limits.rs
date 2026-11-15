//! Centralized engineering limits. These are budgets, not measured capacity claims.

/// Maximum absolute world coordinate. Rendering is only claimed inside this range.
pub const MAX_ABS_COORDINATE: f64 = 1.0e9;
pub const MIN_ZOOM: f64 = 0.02;
pub const MAX_ZOOM: f64 = 64.0;

pub const MAX_OBJECTS: usize = 100_000;
pub const MAX_STROKE_POINTS: usize = 200_000;
pub const MAX_TOTAL_STROKE_POINTS: usize = 2_000_000;
pub const MAX_GROUP_DEPTH: usize = 64;
pub const MAX_OBJECT_TEXT_BYTES: usize = 64 * 1024;
pub const MAX_UNDO_STEPS: usize = 200;

pub const MAX_IMPORT_BYTES: u64 = 64 * 1024 * 1024;
pub const MAX_EXPORT_OBJECTS: usize = MAX_OBJECTS;

pub const MAX_QUESTION_BYTES: usize = 64 * 1024;
pub const MAX_ANSWER_BYTES: usize = 1024 * 1024;
pub const MAX_AI_ALTERNATIVES: usize = 8;
pub const MAX_CONCURRENT_GENERATIONS: usize = 4;

pub const MAX_ROOM_PEERS: usize = 8;
pub const MAX_WIRE_CHUNK_BYTES: usize = 16 * 1024;
pub const MAX_SNAPSHOT_BYTES: usize = MAX_IMPORT_BYTES as usize;
