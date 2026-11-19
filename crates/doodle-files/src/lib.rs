//! Portable board file codec.
//!
//! The payload is a fresh materialization of the visible shared board, encoded as a compact
//! `BoardSnapshot`. Import always produces independent objects with fresh IDs; it never applies
//! imported CRDT history to the live document.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use doodle_types::{
    BoardSnapshot, ENVELOPE_VERSION, MAX_IMPORT_BYTES, MAX_OBJECTS, MAX_TOTAL_STROKE_POINTS,
    ObjectId, ObjectKind, Pose, Rect, StoredObject,
};
use thiserror::Error;

const MAGIC: [u8; 8] = *b"DVRSE\x00\x00\x00";
const CODEC_POSTCARD: u16 = 1;
const HEADER_LEN: usize = 24;

#[derive(Debug, Error)]
pub enum FileError {
    #[error("file is too large")]
    TooLarge,
    #[error("file is truncated")]
    Truncated,
    #[error("not a Doodleverse board file")]
    BadMagic,
    #[error("unsupported envelope version {0}")]
    UnsupportedEnvelope(u16),
    #[error("unsupported document schema version {0}")]
    UnsupportedSchema(u16),
    #[error("unsupported codec")]
    UnsupportedCodec,
    #[error("unsupported file flags")]
    UnsupportedFlags,
    #[error("file is corrupt")]
    Corrupt,
    #[error("board content is invalid")]
    InvalidBoard,
    #[error("board contains too much geometry")]
    TooMuchGeometry,
    #[error("board is empty")]
    Empty,
    #[error("I/O error: {0}")]
    Io(String),
}

/// A validated, independent copy of an imported board.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportPayload {
    /// Objects with fresh application IDs, remapped parents, normalized to origin.
    pub objects: Vec<StoredObject>,
    /// Top-level wrapper candidates (objects with no parent inside the payload).
    pub root_ids: Vec<ObjectId>,
    /// Content bounds after normalization; minimum corner is at (0,0).
    pub bounds: Rect,
    pub object_count: usize,
}

impl ImportPayload {
    #[must_use]
    pub fn with_pose(objects: &[StoredObject], dx: f64, dy: f64) -> Vec<StoredObject> {
        objects
            .iter()
            .cloned()
            .map(|mut object| {
                object.pose = Pose::new(object.pose.x + dx, object.pose.y + dy);
                object
            })
            .collect()
    }

    /// Builds the final imported object list plus a wrapper group at `wrapper_id`.
    #[must_use]
    pub fn into_commit(
        &self,
        wrapper_id: ObjectId,
        dx: f64,
        dy: f64,
        order_key: i64,
    ) -> Vec<StoredObject> {
        let mut objects = Self::with_pose(&self.objects, dx, dy);
        let wrapper = StoredObject {
            id: wrapper_id,
            parent: None,
            order_key,
            pose: Pose::default(),
            kind: ObjectKind::Group,
        };
        for object in &mut objects {
            if object.parent.is_none() {
                object.parent = Some(wrapper_id);
            }
        }
        objects.push(wrapper);
        objects
    }
}

/// Serializes a visible board snapshot into the portable envelope.
pub fn encode(snapshot: &BoardSnapshot) -> Result<Vec<u8>, FileError> {
    snapshot.validate().map_err(|_| FileError::InvalidBoard)?;
    if snapshot.objects.len() > MAX_OBJECTS {
        return Err(FileError::InvalidBoard);
    }
    let total_points: usize = snapshot
        .objects
        .iter()
        .filter_map(|object| match &object.kind {
            ObjectKind::Stroke(stroke) => Some(stroke.geometry.points.len()),
            _ => None,
        })
        .sum();
    if total_points > MAX_TOTAL_STROKE_POINTS {
        return Err(FileError::TooMuchGeometry);
    }
    let payload = postcard::to_allocvec(snapshot).map_err(|_| FileError::InvalidBoard)?;
    if payload.len() as u64 > MAX_IMPORT_BYTES {
        return Err(FileError::TooLarge);
    }
    let checksum = crc32fast::hash(&payload);
    let mut bytes = Vec::with_capacity(HEADER_LEN + payload.len());
    bytes.extend_from_slice(&MAGIC);
    bytes.extend_from_slice(&ENVELOPE_VERSION.to_le_bytes());
    bytes.extend_from_slice(&snapshot.schema_version.to_le_bytes());
    bytes.extend_from_slice(&CODEC_POSTCARD.to_le_bytes());
    bytes.extend_from_slice(&0_u16.to_le_bytes());
    bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&checksum.to_le_bytes());
    bytes.extend_from_slice(&payload);
    Ok(bytes)
}

/// Decodes and validates a board file into a fresh independent import payload.
///
/// The live document is untouched on every error path.
pub fn decode_and_validate(bytes: &[u8]) -> Result<ImportPayload, FileError> {
    if bytes.len() as u64 > MAX_IMPORT_BYTES + HEADER_LEN as u64 {
        return Err(FileError::TooLarge);
    }
    if bytes.len() < HEADER_LEN {
        return Err(FileError::Truncated);
    }
    if bytes[..8] != MAGIC {
        return Err(FileError::BadMagic);
    }
    let envelope_version = u16::from_le_bytes([bytes[8], bytes[9]]);
    if envelope_version != ENVELOPE_VERSION {
        return Err(FileError::UnsupportedEnvelope(envelope_version));
    }
    let schema_version = u16::from_le_bytes([bytes[10], bytes[11]]);
    let codec = u16::from_le_bytes([bytes[12], bytes[13]]);
    if codec != CODEC_POSTCARD {
        return Err(FileError::UnsupportedCodec);
    }
    let flags = u16::from_le_bytes([bytes[14], bytes[15]]);
    if flags != 0 {
        return Err(FileError::UnsupportedFlags);
    }
    let payload_len = u32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]) as usize;
    let checksum = u32::from_le_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
    if payload_len > MAX_IMPORT_BYTES as usize {
        return Err(FileError::TooLarge);
    }
    let payload = bytes
        .get(HEADER_LEN..HEADER_LEN + payload_len)
        .ok_or(FileError::Truncated)?;
    if payload.len() != payload_len || bytes.len() != HEADER_LEN + payload_len {
        return Err(FileError::Corrupt);
    }
    if crc32fast::hash(payload) != checksum {
        return Err(FileError::Corrupt);
    }
    let (snapshot, remainder): (BoardSnapshot, &[u8]) =
        postcard::take_from_bytes(payload).map_err(|_| FileError::InvalidBoard)?;
    if !remainder.is_empty() {
        return Err(FileError::Corrupt);
    }
    if snapshot.schema_version != schema_version {
        return Err(FileError::UnsupportedSchema(schema_version));
    }
    if snapshot.schema_version != doodle_types::DOCUMENT_SCHEMA_VERSION {
        return Err(FileError::UnsupportedSchema(snapshot.schema_version));
    }
    snapshot.validate().map_err(|_| FileError::InvalidBoard)?;
    build_payload(snapshot)
}

fn build_payload(snapshot: BoardSnapshot) -> Result<ImportPayload, FileError> {
    if snapshot.objects.is_empty() {
        return Err(FileError::Empty);
    }
    let total_points: usize = snapshot
        .objects
        .iter()
        .filter_map(|object| match &object.kind {
            ObjectKind::Stroke(stroke) => Some(stroke.geometry.points.len()),
            _ => None,
        })
        .sum();
    if total_points > MAX_TOTAL_STROKE_POINTS {
        return Err(FileError::TooMuchGeometry);
    }
    let mut remap: HashMap<ObjectId, ObjectId> = HashMap::with_capacity(snapshot.objects.len());
    for object in &snapshot.objects {
        remap.insert(object.id, ObjectId::new());
    }
    let mut seen = HashSet::with_capacity(snapshot.objects.len());
    let mut objects = Vec::with_capacity(snapshot.objects.len());
    let mut bounds: Option<Rect> = None;
    for object in &snapshot.objects {
        let Some(new_id) = remap.get(&object.id).copied() else {
            return Err(FileError::InvalidBoard);
        };
        if !seen.insert(new_id) {
            return Err(FileError::InvalidBoard);
        }
        let mut cloned = object.clone();
        cloned.id = new_id;
        cloned.parent = match object.parent {
            Some(parent) => Some(remap.get(&parent).copied().ok_or(FileError::InvalidBoard)?),
            None => None,
        };
        if let Some(object_bounds) = cloned.bounds() {
            bounds = Some(bounds.map_or(object_bounds, |current| current.union(object_bounds)));
        }
        objects.push(cloned);
    }
    // Parents were remapped through the *new* id map above; verify they refer to groups.
    let group_ids: HashSet<ObjectId> = objects
        .iter()
        .filter(|object| object.kind.is_group())
        .map(|object| object.id)
        .collect();
    for object in &mut objects {
        if let Some(parent) = object.parent
            && !group_ids.contains(&parent)
        {
            object.parent = None;
        }
    }
    let bounds = bounds.ok_or(FileError::Empty)?;
    let (dx, dy) = normalization_offset(&objects, bounds).ok_or(FileError::InvalidBoard)?;
    for object in &mut objects {
        object.pose = Pose::new(object.pose.x + dx, object.pose.y + dy);
        if !object.pose.is_finite() {
            return Err(FileError::InvalidBoard);
        }
    }
    let bounds = Rect::new(
        bounds.min_x + dx,
        bounds.min_y + dy,
        bounds.max_x + dx,
        bounds.max_y + dy,
    );
    let root_ids = objects
        .iter()
        .filter(|object| object.parent.is_none())
        .map(|object| object.id)
        .collect();
    Ok(ImportPayload {
        object_count: objects.len(),
        objects,
        root_ids,
        bounds: Rect::new(0.0, 0.0, bounds.width(), bounds.height()),
    })
}

/// Keeps the usual origin placement when it fits the coordinate budget, and centers the board
/// instead for content spanning most of the valid world. Returns `None` when even centering
/// cannot keep every pose inside the documented limit.
fn normalization_offset(objects: &[StoredObject], bounds: Rect) -> Option<(f64, f64)> {
    let candidates = [
        (-bounds.min_x, -bounds.min_y),
        (-bounds.center().x, -bounds.center().y),
        (0.0, 0.0),
    ];
    candidates.into_iter().find(|(dx, dy)| {
        objects
            .iter()
            .all(|object| Pose::new(object.pose.x + dx, object.pose.y + dy).is_finite())
    })
}

/// Writes bytes to a sibling temporary file, then replaces `path`.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), FileError> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    if !parent.exists() {
        return Err(FileError::Io("destination folder does not exist".into()));
    }
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("board.doodleverse");
    let temp_path: PathBuf = parent.join(format!(".{file_name}.{}.tmp", std::process::id() as u64));
    let write_result = (|| -> std::io::Result<()> {
        let mut file = fs::File::create(&temp_path)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        Ok(())
    })();
    if let Err(error) = write_result {
        let _ = fs::remove_file(&temp_path);
        return Err(FileError::Io(error.to_string()));
    }
    if let Err(error) = fs::rename(&temp_path, path) {
        let _ = fs::remove_file(&temp_path);
        return Err(FileError::Io(error.to_string()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use doodle_types::{BoardSnapshot, StrokeGeometry, StrokeObject, StrokePoint, StrokeStyle};

    fn stroke(x: f64) -> StoredObject {
        StoredObject {
            id: ObjectId::new(),
            parent: None,
            order_key: (x as i64) * 16,
            pose: Pose::new(x, 0.0),
            kind: ObjectKind::Stroke(StrokeObject {
                style: StrokeStyle::default(),
                geometry: StrokeGeometry::new(vec![
                    StrokePoint {
                        x: 0.0,
                        y: 0.0,
                        width: 6.0,
                    },
                    StrokePoint {
                        x: 30.0,
                        y: 20.0,
                        width: 6.0,
                    },
                ]),
            }),
        }
    }

    fn statement(text: &str) -> StoredObject {
        StoredObject {
            id: ObjectId::new(),
            parent: None,
            order_key: 32,
            pose: Pose::new(80.0, 30.0),
            kind: ObjectKind::Statement(doodle_types::StatementObject {
                text: text.into(),
                width: 200.0,
                height: 100.0,
                style: doodle_types::StatementStyle::default(),
                kind: doodle_types::StatementKind::Sticky,
            }),
        }
    }

    fn shape(kind: doodle_types::ShapeKind) -> StoredObject {
        StoredObject {
            id: ObjectId::new(),
            parent: None,
            order_key: 48,
            pose: Pose::new(40.0, 60.0),
            kind: ObjectKind::Shape(doodle_types::ShapeObject {
                shape: kind,
                width: 160.0,
                height: 90.0,
                style: StrokeStyle::default(),
                flip_x: false,
                flip_y: false,
                fill_rgba: [0, 0, 0, 0],
                fill_style: doodle_types::FillStyle::default(),
            }),
        }
    }

    #[test]
    fn round_trip_preserves_shapes_and_statement_kinds() {
        let mut text = statement("plain");
        if let ObjectKind::Statement(statement) = &mut text.kind {
            statement.kind = doodle_types::StatementKind::Text;
        }
        let mut arrow = shape(doodle_types::ShapeKind::Arrow);
        if let ObjectKind::Shape(shape) = &mut arrow.kind {
            shape.width = 0.0;
            shape.flip_y = true;
        }
        let snapshot = BoardSnapshot::new(vec![
            shape(doodle_types::ShapeKind::Diamond),
            arrow,
            statement("sticky"),
            text,
        ]);
        let bytes = encode(&snapshot).expect("encode");
        let payload = decode_and_validate(&bytes).expect("decode");
        assert_eq!(payload.object_count, 4);
        assert!(payload.objects.iter().any(|object| matches!(
            &object.kind,
            ObjectKind::Shape(shape) if shape.shape == doodle_types::ShapeKind::Diamond
        )));
        let arrow = payload
            .objects
            .iter()
            .find_map(|object| match &object.kind {
                ObjectKind::Shape(shape) if shape.shape == doodle_types::ShapeKind::Arrow => {
                    Some(shape)
                }
                _ => None,
            })
            .expect("arrow");
        assert!(arrow.flip_y);
        assert!(!arrow.flip_x);
        assert_eq!((arrow.width, arrow.height), (0.0, 90.0));
        assert!(payload.objects.iter().any(|object| matches!(
            &object.kind,
            ObjectKind::Statement(statement)
                if statement.text == "plain" && statement.kind == doodle_types::StatementKind::Text
        )));
        assert!(payload.objects.iter().any(|object| matches!(
            &object.kind,
            ObjectKind::Statement(statement)
                if statement.text == "sticky" && statement.kind == doodle_types::StatementKind::Sticky
        )));
    }

    #[derive(serde::Serialize)]
    struct LegacyStatement {
        text: String,
        width: f64,
        height: f64,
        style: doodle_types::StatementStyle,
    }

    #[derive(serde::Serialize)]
    struct LegacyShape {
        shape: doodle_types::ShapeKind,
        width: f64,
        height: f64,
        style: StrokeStyle,
    }

    // Variant order must match `ObjectKind` so postcard's variant index is the same.
    #[allow(dead_code)]
    #[derive(serde::Serialize)]
    enum LegacyObjectKind {
        Stroke(StrokeObject),
        Group,
        Statement(LegacyStatement),
        Shape(LegacyShape),
    }

    #[derive(serde::Serialize)]
    struct LegacyStoredObject {
        id: ObjectId,
        parent: Option<ObjectId>,
        order_key: i64,
        pose: Pose,
        kind: LegacyObjectKind,
    }

    #[derive(serde::Serialize)]
    struct LegacySnapshot {
        schema_version: u16,
        objects: Vec<LegacyStoredObject>,
    }

    #[test]
    fn snapshot_written_before_statement_kind_still_decodes() {
        let legacy = LegacySnapshot {
            schema_version: doodle_types::DOCUMENT_SCHEMA_VERSION,
            objects: vec![LegacyStoredObject {
                id: ObjectId::new(),
                parent: None,
                order_key: 0,
                pose: Pose::new(5.0, 10.0),
                kind: LegacyObjectKind::Statement(LegacyStatement {
                    text: "old note".into(),
                    width: 180.0,
                    height: 90.0,
                    style: doodle_types::StatementStyle::default(),
                }),
            }],
        };
        let bytes = wrap_legacy(&legacy, legacy.schema_version);

        let decoded = decode_and_validate(&bytes).expect("legacy snapshot decodes");
        let statement = decoded
            .objects
            .iter()
            .find_map(|object| match &object.kind {
                ObjectKind::Statement(statement) => Some(statement),
                _ => None,
            })
            .expect("statement");
        assert_eq!(statement.text, "old note");
        assert_eq!(statement.kind, doodle_types::StatementKind::Sticky);
    }

    #[test]
    fn snapshot_written_before_shape_direction_flags_still_decodes() {
        let legacy = LegacySnapshot {
            schema_version: doodle_types::DOCUMENT_SCHEMA_VERSION,
            objects: vec![LegacyStoredObject {
                id: ObjectId::new(),
                parent: None,
                order_key: 0,
                pose: Pose::new(5.0, 10.0),
                kind: LegacyObjectKind::Shape(LegacyShape {
                    shape: doodle_types::ShapeKind::Arrow,
                    width: 120.0,
                    height: 80.0,
                    style: StrokeStyle::default(),
                }),
            }],
        };
        let bytes = wrap_legacy(&legacy, legacy.schema_version);

        let decoded = decode_and_validate(&bytes).expect("legacy shape decodes");
        let shape = decoded
            .objects
            .iter()
            .find_map(|object| match &object.kind {
                ObjectKind::Shape(shape) => Some(shape),
                _ => None,
            })
            .expect("arrow");
        assert_eq!(shape.shape, doodle_types::ShapeKind::Arrow);
        assert_eq!((shape.width, shape.height), (120.0, 80.0));
        assert!(!shape.flip_x);
        assert!(!shape.flip_y);
        assert_eq!(shape.fill_rgba, [0, 0, 0, 0]);
        assert_eq!(shape.fill_style, doodle_types::FillStyle::Hachure);
    }

    #[derive(serde::Serialize)]
    struct LegacyFlippedShape {
        shape: doodle_types::ShapeKind,
        width: f64,
        height: f64,
        style: StrokeStyle,
        flip_x: bool,
        flip_y: bool,
    }

    // Variant order must match `ObjectKind` so postcard's variant index is the same.
    #[allow(dead_code)]
    #[derive(serde::Serialize)]
    enum LegacyFlippedKind {
        Stroke(StrokeObject),
        Group,
        Statement(LegacyStatement),
        Shape(LegacyFlippedShape),
    }

    #[derive(serde::Serialize)]
    struct LegacyFlippedObject {
        id: ObjectId,
        parent: Option<ObjectId>,
        order_key: i64,
        pose: Pose,
        kind: LegacyFlippedKind,
    }

    #[derive(serde::Serialize)]
    struct LegacyFlippedSnapshot {
        schema_version: u16,
        objects: Vec<LegacyFlippedObject>,
    }

    #[test]
    fn snapshot_written_before_shape_fill_still_decodes() {
        // The direction flags existed, the fill did not: the decoder must stop cleanly after
        // flip_y and default to transparent hachure.
        let legacy = LegacyFlippedSnapshot {
            schema_version: doodle_types::DOCUMENT_SCHEMA_VERSION,
            objects: vec![LegacyFlippedObject {
                id: ObjectId::new(),
                parent: None,
                order_key: 0,
                pose: Pose::new(5.0, 10.0),
                kind: LegacyFlippedKind::Shape(LegacyFlippedShape {
                    shape: doodle_types::ShapeKind::Arrow,
                    width: 120.0,
                    height: 80.0,
                    style: StrokeStyle::default(),
                    flip_x: false,
                    flip_y: true,
                }),
            }],
        };
        let bytes = wrap_legacy(&legacy, legacy.schema_version);
        let decoded = decode_and_validate(&bytes).expect("legacy shape decodes");
        let shape = decoded
            .objects
            .iter()
            .find_map(|object| match &object.kind {
                ObjectKind::Shape(shape) => Some(shape),
                _ => None,
            })
            .expect("diamond");
        assert!(shape.flip_y);
        assert_eq!(shape.fill_rgba, [0, 0, 0, 0]);
        assert_eq!(shape.fill_style, doodle_types::FillStyle::Hachure);
    }

    #[test]
    fn round_trip_preserves_shape_fill() {
        let mut filled = shape(doodle_types::ShapeKind::Diamond);
        if let ObjectKind::Shape(value) = &mut filled.kind {
            value.fill_rgba = [232, 226, 214, 200];
            value.fill_style = doodle_types::FillStyle::CrossHatch;
        }
        let snapshot = BoardSnapshot::new(vec![filled]);
        let bytes = encode(&snapshot).expect("encode");
        let payload = decode_and_validate(&bytes).expect("decode");
        let shape = payload
            .objects
            .iter()
            .find_map(|object| match &object.kind {
                ObjectKind::Shape(shape) => Some(shape),
                _ => None,
            })
            .expect("diamond");
        assert_eq!(shape.fill_rgba, [232, 226, 214, 200]);
        assert_eq!(shape.fill_style, doodle_types::FillStyle::CrossHatch);
    }

    fn wrap_legacy<T: serde::Serialize>(snapshot: &T, schema_version: u16) -> Vec<u8> {
        let payload = postcard::to_allocvec(snapshot).expect("legacy payload");
        let mut bytes = Vec::with_capacity(HEADER_LEN + payload.len());
        bytes.extend_from_slice(&MAGIC);
        bytes.extend_from_slice(&doodle_types::ENVELOPE_VERSION.to_le_bytes());
        bytes.extend_from_slice(&schema_version.to_le_bytes());
        bytes.extend_from_slice(&CODEC_POSTCARD.to_le_bytes());
        bytes.extend_from_slice(&0_u16.to_le_bytes());
        bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&crc32fast::hash(&payload).to_le_bytes());
        bytes.extend_from_slice(&payload);
        bytes
    }

    #[test]
    fn round_trip_preserves_content_and_refreshes_ids() {
        let a = stroke(10.0);
        let b = statement("hello");
        let snapshot = BoardSnapshot::new(vec![a.clone(), b.clone()]);
        let bytes = encode(&snapshot).expect("encode");
        let payload = decode_and_validate(&bytes).expect("decode");
        assert_eq!(payload.object_count, 2);
        assert!(
            payload
                .objects
                .iter()
                .all(|object| object.id != a.id && object.id != b.id)
        );
        assert_eq!(payload.bounds.min_x, 0.0);
        assert_eq!(payload.bounds.min_y, 0.0);
        let statement = payload
            .objects
            .iter()
            .find(|object| matches!(object.kind, ObjectKind::Statement(_)))
            .expect("statement");
        if let ObjectKind::Statement(value) = &statement.kind {
            assert_eq!(value.text, "hello");
        }
    }

    #[test]
    fn importing_twice_yields_disjoint_ids() {
        let snapshot = BoardSnapshot::new(vec![stroke(0.0), stroke(50.0)]);
        let bytes = encode(&snapshot).expect("encode");
        let first = decode_and_validate(&bytes).expect("first");
        let second = decode_and_validate(&bytes).expect("second");
        let first_ids: HashSet<_> = first.objects.iter().map(|object| object.id).collect();
        let second_ids: HashSet<_> = second.objects.iter().map(|object| object.id).collect();
        assert!(first_ids.is_disjoint(&second_ids));
    }

    #[test]
    fn corrupt_checksum_is_rejected() {
        let bytes = encode(&BoardSnapshot::new(vec![stroke(0.0)])).expect("encode");
        let mut corrupt = bytes.clone();
        let last = corrupt.len() - 1;
        corrupt[last] ^= 0xff;
        assert!(matches!(
            decode_and_validate(&corrupt),
            Err(FileError::Corrupt)
        ));
    }

    #[test]
    fn truncated_and_bad_magic_are_rejected() {
        let bytes = encode(&BoardSnapshot::new(vec![stroke(0.0)])).expect("encode");
        assert!(matches!(
            decode_and_validate(&bytes[..HEADER_LEN - 1]),
            Err(FileError::Truncated)
        ));
        assert!(matches!(
            decode_and_validate(&bytes[..HEADER_LEN + 1]),
            Err(FileError::Truncated)
        ));
        let mut bad = bytes.clone();
        bad[0] = b'X';
        assert!(matches!(
            decode_and_validate(&bad),
            Err(FileError::BadMagic)
        ));
    }

    #[test]
    fn unsupported_schema_version_is_rejected() {
        let snapshot = BoardSnapshot {
            schema_version: 999,
            objects: vec![stroke(0.0)],
        };
        // Encode rejects unknown schema before writing.
        assert!(matches!(encode(&snapshot), Err(FileError::InvalidBoard)));

        let bytes = encode(&BoardSnapshot::new(vec![stroke(0.0)])).expect("encode");
        let mut future = bytes;
        future[10..12].copy_from_slice(&999_u16.to_le_bytes());
        // Envelope schema and payload schema disagree.
        assert!(matches!(
            decode_and_validate(&future),
            Err(FileError::UnsupportedSchema(999) | FileError::Corrupt)
        ));
    }

    #[test]
    fn deleted_sentinel_is_absent_from_export() {
        let mut doc =
            doodle_types::BoardSnapshot::new(vec![stroke(0.0), statement("DELETED-SENTINEL-9137")]);
        // Remove the sentinel the way the document does: only visible objects are exported.
        doc.objects.pop();
        let bytes = encode(&doc).expect("encode");
        assert!(!contains_bytes(&bytes, b"DELETED-SENTINEL-9137"));
    }

    fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
        haystack
            .windows(needle.len())
            .any(|window| window == needle)
    }

    #[test]
    fn import_commit_wraps_roots() {
        let bytes = encode(&BoardSnapshot::new(vec![stroke(0.0), stroke(50.0)])).expect("encode");
        let payload = decode_and_validate(&bytes).expect("decode");
        let wrapper = ObjectId::new();
        let objects = payload.into_commit(wrapper, 100.0, 200.0, 10_000);
        assert!(objects.iter().any(|object| object.id == wrapper));
        assert_eq!(
            objects
                .iter()
                .filter(|object| object.parent == Some(wrapper))
                .count(),
            payload.root_ids.len()
        );
    }

    #[test]
    fn import_preserves_nested_group_relationships() {
        let root = StoredObject {
            id: ObjectId::new(),
            parent: None,
            order_key: 0,
            pose: Pose::default(),
            kind: ObjectKind::Group,
        };
        let child = StoredObject {
            id: ObjectId::new(),
            parent: Some(root.id),
            order_key: 16,
            pose: Pose::default(),
            kind: ObjectKind::Group,
        };
        let mut leaf = stroke(10.0);
        leaf.parent = Some(child.id);
        let bytes = encode(&BoardSnapshot::new(vec![root, child, leaf])).expect("encode");
        let payload = decode_and_validate(&bytes).expect("decode");
        assert_eq!(payload.root_ids.len(), 1);
        let groups: Vec<_> = payload
            .objects
            .iter()
            .filter(|object| object.kind.is_group())
            .collect();
        assert_eq!(groups.len(), 2);
        let root_id = groups
            .iter()
            .find(|object| object.parent.is_none())
            .map(|object| object.id)
            .expect("root group");
        let child_group = groups
            .iter()
            .find(|object| object.parent == Some(root_id))
            .expect("child group keeps its parent");
        let leaf = payload
            .objects
            .iter()
            .find(|object| !object.kind.is_group())
            .expect("leaf");
        assert_eq!(leaf.parent, Some(child_group.id));
    }

    #[test]
    fn trailing_bytes_are_rejected() {
        let bytes = encode(&BoardSnapshot::new(vec![stroke(0.0)])).expect("encode");
        let mut extended = bytes;
        extended.extend_from_slice(&[0u8; 32]);
        assert!(matches!(
            decode_and_validate(&extended),
            Err(FileError::Corrupt)
        ));
    }

    #[test]
    fn extreme_span_normalizes_without_losing_validity() {
        let mut left = stroke(0.0);
        left.pose = Pose::new(-doodle_types::MAX_ABS_COORDINATE, 0.0);
        let mut right = stroke(0.0);
        right.pose = Pose::new(doodle_types::MAX_ABS_COORDINATE, 0.0);
        let bytes = encode(&BoardSnapshot::new(vec![left, right])).expect("encode");
        let payload = decode_and_validate(&bytes).expect("decode");
        assert!(payload.objects.iter().all(|object| object.pose.is_finite()));
        assert!(payload.bounds.width() > 1.0e9);
    }

    #[test]
    fn atomic_write_replaces_existing_file() {
        let dir = std::env::temp_dir().join(format!(
            "doodleverse-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("board.doodleverse");
        write_atomic(&path, b"old").expect("first write");
        write_atomic(&path, b"new").expect("second write");
        assert_eq!(fs::read(&path).expect("read"), b"new");
        let leftovers: Vec<_> = fs::read_dir(&dir)
            .expect("read dir")
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
            .collect();
        assert!(leftovers.is_empty());
        let _ = fs::remove_dir_all(&dir);
    }
}
