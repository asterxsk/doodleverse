//! Independently versioned boundaries.

/// Application release (matches Cargo package version).
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Portable-file envelope major version.
pub const ENVELOPE_VERSION: u16 = 1;

/// Shared-document schema version stored in board files.
pub const DOCUMENT_SCHEMA_VERSION: u16 = 1;

/// Canonical stroke geometry version.
pub const GEOMETRY_SCHEMA_VERSION: u16 = 1;

/// Peer wire protocol version (reserved; networking is not shipped in this milestone).
pub const WIRE_PROTOCOL_VERSION: u16 = 1;
