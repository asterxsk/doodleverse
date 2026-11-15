//! Shared contracts for Doodleverse.
//!
//! This crate deliberately has no UI, GPU, network, credential, or provider dependencies.

pub mod commands;
pub mod geometry;
pub mod ids;
pub mod limits;
pub mod objects;
pub mod version;

pub use commands::{CommandError, DocumentCommand, DocumentOutcome, ObjectDelta};
pub use geometry::{Point, Rect};
pub use ids::{ImportId, ObjectId, PeerId, RequestId, RoomId};
pub use limits::*;
pub use objects::{
    BoardSnapshot, ObjectKind, Pose, StatementObject, StatementStyle, StoredObject, StrokeGeometry,
    StrokeObject, StrokePoint, StrokeStyle,
};
pub use version::{
    APP_VERSION, DOCUMENT_SCHEMA_VERSION, ENVELOPE_VERSION, GEOMETRY_SCHEMA_VERSION,
    WIRE_PROTOCOL_VERSION,
};
