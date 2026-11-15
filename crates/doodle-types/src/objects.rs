use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::geometry::{Point, Rect};
use crate::ids::ObjectId;
use crate::limits::{
    MAX_ABS_COORDINATE, MAX_GROUP_DEPTH, MAX_OBJECT_TEXT_BYTES, MAX_OBJECTS, MAX_STROKE_POINTS,
};
use crate::version::{DOCUMENT_SCHEMA_VERSION, GEOMETRY_SCHEMA_VERSION};

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Pose {
    pub x: f64,
    pub y: f64,
}

impl Pose {
    #[must_use]
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    #[must_use]
    pub fn is_finite(self) -> bool {
        self.x.is_finite()
            && self.y.is_finite()
            && self.x.abs() <= MAX_ABS_COORDINATE
            && self.y.abs() <= MAX_ABS_COORDINATE
    }

    #[must_use]
    pub fn as_point(self) -> Point {
        Point::new(self.x, self.y)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct StrokeStyle {
    pub rgba: [u8; 4],
    pub base_width: f32,
}

impl Default for StrokeStyle {
    fn default() -> Self {
        Self {
            rgba: [30, 30, 34, 255],
            base_width: 4.0,
        }
    }
}

impl StrokeStyle {
    #[must_use]
    pub fn is_valid(self) -> bool {
        self.base_width.is_finite() && (0.1..=400.0).contains(&self.base_width)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct StrokePoint {
    pub x: f64,
    pub y: f64,
    pub width: f32,
}

impl StrokePoint {
    #[must_use]
    pub fn is_valid(self) -> bool {
        self.x.is_finite()
            && self.y.is_finite()
            && self.x.abs() <= MAX_ABS_COORDINATE
            && self.y.abs() <= MAX_ABS_COORDINATE
            && self.width.is_finite()
            && (0.05..=400.0).contains(&self.width)
    }
}

/// Canonical, immutable, local-space stroke path. Width is already resolved.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StrokeGeometry {
    pub version: u16,
    pub points: Vec<StrokePoint>,
}

impl StrokeGeometry {
    #[must_use]
    pub fn new(points: Vec<StrokePoint>) -> Self {
        Self {
            version: GEOMETRY_SCHEMA_VERSION,
            points,
        }
    }

    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.version == GEOMETRY_SCHEMA_VERSION
            && !self.points.is_empty()
            && self.points.len() <= MAX_STROKE_POINTS
            && self.points.iter().all(|p| p.is_valid())
    }

    #[must_use]
    pub fn local_bounds(&self) -> Option<Rect> {
        let first = self.points.first()?;
        let mut rect = Rect::new(first.x, first.y, first.x, first.y);
        for p in &self.points {
            let half = f64::from(p.width) * 0.5;
            rect = rect.union(Rect::new(p.x - half, p.y - half, p.x + half, p.y + half));
        }
        Some(rect)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StrokeObject {
    pub style: StrokeStyle,
    pub geometry: StrokeGeometry,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct StatementStyle {
    pub rgba: [u8; 4],
}

impl Default for StatementStyle {
    fn default() -> Self {
        Self {
            rgba: [255, 241, 186, 255],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StatementObject {
    pub text: String,
    pub width: f64,
    pub height: f64,
    pub style: StatementStyle,
}

impl StatementObject {
    #[must_use]
    pub fn size_valid(&self) -> bool {
        self.width.is_finite()
            && self.height.is_finite()
            && (40.0..=4000.0).contains(&self.width)
            && (30.0..=4000.0).contains(&self.height)
            && self.text.len() <= MAX_OBJECT_TEXT_BYTES
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ObjectKind {
    Stroke(StrokeObject),
    Group,
    Statement(StatementObject),
}

impl ObjectKind {
    #[must_use]
    pub fn is_group(&self) -> bool {
        matches!(self, Self::Group)
    }

    #[must_use]
    pub fn local_bounds(&self) -> Option<Rect> {
        match self {
            Self::Stroke(stroke) => stroke.geometry.local_bounds(),
            Self::Group => None,
            Self::Statement(statement) => {
                Some(Rect::new(0.0, 0.0, statement.width, statement.height))
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoredObject {
    pub id: ObjectId,
    pub parent: Option<ObjectId>,
    pub order_key: i64,
    pub pose: Pose,
    pub kind: ObjectKind,
}

impl StoredObject {
    #[must_use]
    pub fn bounds(&self) -> Option<Rect> {
        self.kind
            .local_bounds()
            .map(|rect| rect.translate(self.pose.x, self.pose.y))
    }

    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.pose.is_finite()
            && match &self.kind {
                ObjectKind::Stroke(stroke) => stroke.style.is_valid() && stroke.geometry.is_valid(),
                ObjectKind::Group => true,
                ObjectKind::Statement(statement) => statement.size_valid(),
            }
    }

    #[must_use]
    pub fn with_parent(mut self, parent: Option<ObjectId>) -> Self {
        self.parent = parent;
        self
    }
}

/// Validated, self-contained visible board used by portable files.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoardSnapshot {
    pub schema_version: u16,
    pub objects: Vec<StoredObject>,
}

#[derive(Debug, Error, PartialEq)]
pub enum SnapshotError {
    #[error("unsupported document schema version {0}")]
    UnsupportedSchema(u16),
    #[error("board contains too many objects")]
    TooManyObjects,
    #[error("board contains duplicate object id")]
    DuplicateObjectId,
    #[error("board object is invalid")]
    InvalidObject,
    #[error("board contains a missing parent reference")]
    MissingParent,
    #[error("board groups nest deeper than the supported limit")]
    TooDeep,
    #[error("board contains a parent cycle")]
    Cycle,
}

impl BoardSnapshot {
    #[must_use]
    pub fn new(objects: Vec<StoredObject>) -> Self {
        Self {
            schema_version: DOCUMENT_SCHEMA_VERSION,
            objects,
        }
    }

    /// Validates schema, object fields, references, and group depth.
    pub fn validate(&self) -> Result<(), SnapshotError> {
        if self.schema_version != DOCUMENT_SCHEMA_VERSION {
            return Err(SnapshotError::UnsupportedSchema(self.schema_version));
        }
        if self.objects.len() > MAX_OBJECTS {
            return Err(SnapshotError::TooManyObjects);
        }
        let mut ids = std::collections::HashSet::with_capacity(self.objects.len());
        for object in &self.objects {
            if !ids.insert(object.id) {
                return Err(SnapshotError::DuplicateObjectId);
            }
            if !object.is_valid() {
                return Err(SnapshotError::InvalidObject);
            }
        }
        let by_id: std::collections::HashMap<_, _> =
            self.objects.iter().map(|o| (o.id, o)).collect();
        for object in &self.objects {
            let mut current = object;
            let mut depth = 0;
            while let Some(parent_id) = current.parent {
                depth += 1;
                if depth > MAX_GROUP_DEPTH {
                    return Err(SnapshotError::TooDeep);
                }
                let parent = by_id.get(&parent_id).ok_or(SnapshotError::MissingParent)?;
                if !parent.kind.is_group() {
                    return Err(SnapshotError::MissingParent);
                }
                current = parent;
                if current.id == object.id {
                    return Err(SnapshotError::Cycle);
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn group(id: ObjectId, parent: Option<ObjectId>) -> StoredObject {
        StoredObject {
            id,
            parent,
            order_key: 0,
            pose: Pose::default(),
            kind: ObjectKind::Group,
        }
    }

    fn stroke(id: ObjectId, parent: Option<ObjectId>) -> StoredObject {
        StoredObject {
            id,
            parent,
            order_key: 0,
            pose: Pose::default(),
            kind: ObjectKind::Stroke(StrokeObject {
                style: StrokeStyle::default(),
                geometry: StrokeGeometry::new(vec![StrokePoint {
                    x: 0.0,
                    y: 0.0,
                    width: 4.0,
                }]),
            }),
        }
    }

    #[test]
    fn validates_nested_groups() {
        let a = ObjectId::new();
        let b = ObjectId::new();
        let s = stroke(ObjectId::new(), Some(a));
        let snapshot = BoardSnapshot::new(vec![group(a, None), group(b, Some(a)), s]);
        assert_eq!(snapshot.validate(), Ok(()));
    }

    #[test]
    fn rejects_missing_parent_and_cycle() {
        let a = ObjectId::new();
        let b = ObjectId::new();
        let s = stroke(ObjectId::new(), Some(ObjectId::new()));
        assert_eq!(
            BoardSnapshot::new(vec![s]).validate(),
            Err(SnapshotError::MissingParent)
        );
        let snapshot = BoardSnapshot::new(vec![group(a, Some(b)), group(b, Some(a))]);
        assert_eq!(snapshot.validate(), Err(SnapshotError::Cycle));
    }

    #[test]
    fn stroke_bounds_account_for_width() {
        let stroke = StrokeObject {
            style: StrokeStyle::default(),
            geometry: StrokeGeometry::new(vec![
                StrokePoint {
                    x: 0.0,
                    y: 0.0,
                    width: 4.0,
                },
                StrokePoint {
                    x: 10.0,
                    y: 0.0,
                    width: 4.0,
                },
            ]),
        };
        let object = StoredObject {
            id: ObjectId::new(),
            parent: None,
            order_key: 0,
            pose: Pose::new(5.0, 5.0),
            kind: ObjectKind::Stroke(stroke),
        };
        let bounds = object.bounds().expect("bounds");
        assert_eq!(bounds, Rect::new(3.0, 3.0, 17.0, 7.0));
    }
}
