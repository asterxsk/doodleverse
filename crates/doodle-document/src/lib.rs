//! Shared board document.
//!
//! The CRDT (`yrs`) is an implementation detail behind commands and a normalized projection.
//! Local undo records only local command deltas; remote-origin transactions are never undone.

mod store;

pub use store::{
    BoardDocument, BoardProjection, DocumentError, ORIGIN_LOCAL, ORIGIN_REMOTE, ProjectedObject,
    ProjectionError,
};
