use std::fmt;

use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use thiserror::Error;

use crate::geometry::{Point, Rect};
use crate::ids::ObjectId;
use crate::limits::{
    MAX_ABS_COORDINATE, MAX_GROUP_DEPTH, MAX_OBJECT_TEXT_BYTES, MAX_OBJECTS, MAX_SHAPE_SIZE,
    MAX_STROKE_POINTS, MIN_SHAPE_SIZE,
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

/// Whether a statement is a plain text block or a sticky note. Sticky is the default so
/// statements decoded from files written before the flag existed keep their note card.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum StatementKind {
    Text,
    #[default]
    Sticky,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StatementObject {
    pub text: String,
    pub width: f64,
    pub height: f64,
    pub style: StatementStyle,
    pub kind: StatementKind,
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

/// Postcard does not describe structs, so a field appended after a release is not optional for
/// `#[serde(default)]` alone: the decoder hits end-of-buffer instead of returning `None`. This
/// visitor treats end-of-buffer after the legacy four fields as the default statement kind, which
/// keeps snapshots and CRDT payloads written before `kind` existed readable.
impl<'de> Deserialize<'de> for StatementObject {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct StatementVisitor;

        impl<'de> Visitor<'de> for StatementVisitor {
            type Value = StatementObject;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a statement object")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let text = seq
                    .next_element::<String>()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;
                let width = seq
                    .next_element::<f64>()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?;
                let height = seq
                    .next_element::<f64>()?
                    .ok_or_else(|| de::Error::invalid_length(2, &self))?;
                let style = seq
                    .next_element::<StatementStyle>()?
                    .ok_or_else(|| de::Error::invalid_length(3, &self))?;
                let kind = match seq.next_element::<StatementKind>() {
                    Ok(Some(kind)) => kind,
                    Ok(None) => StatementKind::default(),
                    Err(error) if is_end_of_input(&error) => StatementKind::default(),
                    Err(error) => return Err(error),
                };
                Ok(StatementObject {
                    text,
                    width,
                    height,
                    style,
                    kind,
                })
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut text = None;
                let mut width = None;
                let mut height = None;
                let mut style = None;
                let mut kind = None;
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "text" => text = Some(map.next_value()?),
                        "width" => width = Some(map.next_value()?),
                        "height" => height = Some(map.next_value()?),
                        "style" => style = Some(map.next_value()?),
                        "kind" => kind = Some(map.next_value()?),
                        _ => {
                            let _: de::IgnoredAny = map.next_value()?;
                        }
                    }
                }
                Ok(StatementObject {
                    text: text.ok_or_else(|| de::Error::missing_field("text"))?,
                    width: width.ok_or_else(|| de::Error::missing_field("width"))?,
                    height: height.ok_or_else(|| de::Error::missing_field("height"))?,
                    style: style.ok_or_else(|| de::Error::missing_field("style"))?,
                    kind: kind.unwrap_or_default(),
                })
            }
        }

        deserializer.deserialize_struct(
            "StatementObject",
            &["text", "width", "height", "style", "kind"],
            StatementVisitor,
        )
    }
}

fn is_end_of_input<E: fmt::Display>(error: &E) -> bool {
    error.to_string().contains("end of buffer")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ShapeKind {
    #[default]
    Rectangle,
    Diamond,
    Ellipse,
    Arrow,
    Line,
}

impl ShapeKind {
    /// Lines and arrows are open paths: their box may be zero-extent on either axis and their
    /// endpoints follow the drag direction. Closed shapes keep a minimum box size.
    #[must_use]
    pub const fn is_open(self) -> bool {
        matches!(self, Self::Arrow | Self::Line)
    }
}

/// How a closed shape's interior is painted. Hachure is the default so shapes written before the
/// field existed keep the light, sketch-like fill rather than a solid block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum FillStyle {
    #[default]
    Hachure,
    CrossHatch,
    Solid,
}

/// A shape's box plus the drag direction for open paths.
///
/// `pose` is always the box's minimum corner and `width`/`height` are unsigned. For an arrow or a
/// line the pair `(flip_x, flip_y)` says which corner the drag started from, so a path dragged
/// right-to-left starts at the box's right edge instead of its left. Closed shapes must leave both
/// flags false; their pose already encodes every drag direction.
///
/// `fill_rgba` is the interior paint; alpha 0 means no fill. `fill_style` selects solid or one of
/// the two hatch patterns. Open paths (arrow, line) ignore both.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct ShapeObject {
    pub shape: ShapeKind,
    pub width: f64,
    pub height: f64,
    pub style: StrokeStyle,
    pub flip_x: bool,
    pub flip_y: bool,
    pub fill_rgba: [u8; 4],
    pub fill_style: FillStyle,
}

/// Postcard does not describe structs, so appended fields are not optional for `#[serde(default)]`
/// alone: the decoder hits end-of-buffer instead of returning `None`. This visitor treats
/// end-of-buffer after any legacy field group as that group's defaults, which keeps snapshots and
/// CRDT payloads written before the direction flags and the fill existed readable.
impl<'de> Deserialize<'de> for ShapeObject {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ShapeVisitor;

        impl<'de> Visitor<'de> for ShapeVisitor {
            type Value = ShapeObject;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a shape object")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let shape = seq
                    .next_element::<ShapeKind>()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;
                let width = seq
                    .next_element::<f64>()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?;
                let height = seq
                    .next_element::<f64>()?
                    .ok_or_else(|| de::Error::invalid_length(2, &self))?;
                let style = seq
                    .next_element::<StrokeStyle>()?
                    .ok_or_else(|| de::Error::invalid_length(3, &self))?;
                let (flip_x, legacy) = match seq.next_element::<bool>() {
                    Ok(Some(value)) => (value, false),
                    Ok(None) => (false, true),
                    Err(error) if is_end_of_input(&error) => (false, true),
                    Err(error) => return Err(error),
                };
                let (flip_y, legacy) = if legacy {
                    (false, true)
                } else {
                    match seq.next_element::<bool>() {
                        Ok(Some(value)) => (value, false),
                        Ok(None) => (false, true),
                        Err(error) if is_end_of_input(&error) => (false, true),
                        Err(error) => return Err(error),
                    }
                };
                let fill_rgba = if legacy {
                    [0, 0, 0, 0]
                } else {
                    match seq.next_element::<[u8; 4]>() {
                        Ok(Some(value)) => value,
                        Ok(None) => [0, 0, 0, 0],
                        Err(error) if is_end_of_input(&error) => [0, 0, 0, 0],
                        Err(error) => return Err(error),
                    }
                };
                let fill_style = if legacy {
                    FillStyle::default()
                } else {
                    match seq.next_element::<FillStyle>() {
                        Ok(Some(value)) => value,
                        Ok(None) => FillStyle::default(),
                        Err(error) if is_end_of_input(&error) => FillStyle::default(),
                        Err(error) => return Err(error),
                    }
                };
                Ok(ShapeObject {
                    shape,
                    width,
                    height,
                    style,
                    flip_x,
                    flip_y,
                    fill_rgba,
                    fill_style,
                })
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut shape = None;
                let mut width = None;
                let mut height = None;
                let mut style = None;
                let mut flip_x = None;
                let mut flip_y = None;
                let mut fill_rgba = None;
                let mut fill_style = None;
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "shape" => shape = Some(map.next_value()?),
                        "width" => width = Some(map.next_value()?),
                        "height" => height = Some(map.next_value()?),
                        "style" => style = Some(map.next_value()?),
                        "flip_x" => flip_x = Some(map.next_value()?),
                        "flip_y" => flip_y = Some(map.next_value()?),
                        "fill_rgba" => fill_rgba = Some(map.next_value()?),
                        "fill_style" => fill_style = Some(map.next_value()?),
                        _ => {
                            let _: de::IgnoredAny = map.next_value()?;
                        }
                    }
                }
                Ok(ShapeObject {
                    shape: shape.ok_or_else(|| de::Error::missing_field("shape"))?,
                    width: width.ok_or_else(|| de::Error::missing_field("width"))?,
                    height: height.ok_or_else(|| de::Error::missing_field("height"))?,
                    style: style.ok_or_else(|| de::Error::missing_field("style"))?,
                    flip_x: flip_x.unwrap_or(false),
                    flip_y: flip_y.unwrap_or(false),
                    fill_rgba: fill_rgba.unwrap_or([0, 0, 0, 0]),
                    fill_style: fill_style.unwrap_or_default(),
                })
            }
        }

        deserializer.deserialize_struct(
            "ShapeObject",
            &[
                "shape",
                "width",
                "height",
                "style",
                "flip_x",
                "flip_y",
                "fill_rgba",
                "fill_style",
            ],
            ShapeVisitor,
        )
    }
}

impl ShapeObject {
    #[must_use]
    pub fn is_valid(&self) -> bool {
        let min = if self.shape.is_open() {
            0.0
        } else {
            MIN_SHAPE_SIZE
        };
        self.width.is_finite()
            && self.height.is_finite()
            && (min..=MAX_SHAPE_SIZE).contains(&self.width)
            && (min..=MAX_SHAPE_SIZE).contains(&self.height)
            && self.style.is_valid()
            && (self.shape.is_open() || (!self.flip_x && !self.flip_y))
    }

    #[must_use]
    pub fn local_bounds(&self) -> Option<Rect> {
        let drawable = if self.shape.is_open() {
            self.width >= 0.0 && self.height >= 0.0
        } else {
            self.width > 0.0 && self.height > 0.0
        };
        drawable.then(|| Rect::new(0.0, 0.0, self.width, self.height))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ObjectKind {
    Stroke(StrokeObject),
    Group,
    Statement(StatementObject),
    Shape(ShapeObject),
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
            Self::Shape(shape) => shape.local_bounds(),
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
                ObjectKind::Shape(shape) => shape.is_valid(),
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
    fn shape_validity_checks_size_and_style() {
        let mut shape = ShapeObject {
            shape: ShapeKind::Rectangle,
            width: 120.0,
            height: 80.0,
            style: StrokeStyle::default(),
            flip_x: false,
            flip_y: false,
            fill_rgba: [0, 0, 0, 0],
            fill_style: FillStyle::default(),
        };
        assert!(shape.is_valid());
        assert_eq!(shape.local_bounds(), Some(Rect::new(0.0, 0.0, 120.0, 80.0)));

        shape.width = 0.0;
        assert!(!shape.is_valid());
        shape.width = f64::NAN;
        assert!(!shape.is_valid());
        shape.width = 120.0;
        shape.height = MAX_SHAPE_SIZE + 1.0;
        assert!(!shape.is_valid());
        shape.height = 80.0;
        shape.style.base_width = 0.0;
        assert!(!shape.is_valid());

        let object = StoredObject {
            id: ObjectId::new(),
            parent: None,
            order_key: 0,
            pose: Pose::new(10.0, 20.0),
            kind: ObjectKind::Shape(shape),
        };
        assert!(!object.is_valid());
        shape.style = StrokeStyle::default();
        let object = StoredObject {
            kind: ObjectKind::Shape(shape),
            ..object
        };
        assert!(object.is_valid());
        assert_eq!(object.bounds(), Some(Rect::new(10.0, 20.0, 130.0, 100.0)));
    }

    #[test]
    fn open_shapes_accept_zero_extent_and_direction_flags() {
        let mut line = ShapeObject {
            shape: ShapeKind::Line,
            width: 0.0,
            height: 120.0,
            style: StrokeStyle::default(),
            flip_x: false,
            flip_y: true,
            fill_rgba: [0, 0, 0, 0],
            fill_style: FillStyle::default(),
        };
        assert!(line.is_valid());
        assert_eq!(line.local_bounds(), Some(Rect::new(0.0, 0.0, 0.0, 120.0)));

        line.width = 90.0;
        line.flip_x = true;
        assert!(line.is_valid());

        let arrow = ShapeObject {
            shape: ShapeKind::Arrow,
            ..line
        };
        assert!(arrow.is_valid());
        assert_eq!(arrow.local_bounds(), Some(Rect::new(0.0, 0.0, 90.0, 120.0)));

        // Closed shapes keep a positive box and never carry direction flags.
        let mut rect = ShapeObject {
            shape: ShapeKind::Rectangle,
            width: 0.0,
            height: 80.0,
            style: StrokeStyle::default(),
            flip_x: false,
            flip_y: false,
            fill_rgba: [0, 0, 0, 0],
            fill_style: FillStyle::default(),
        };
        assert!(!rect.is_valid());
        rect.width = 120.0;
        assert!(rect.is_valid());
        rect.flip_x = true;
        assert!(!rect.is_valid());
        assert_eq!(rect.local_bounds(), Some(Rect::new(0.0, 0.0, 120.0, 80.0)));
    }

    #[test]
    fn shape_kind_reports_open_paths() {
        assert!(ShapeKind::Line.is_open());
        assert!(ShapeKind::Arrow.is_open());
        assert!(!ShapeKind::Rectangle.is_open());
        assert!(!ShapeKind::Diamond.is_open());
        assert!(!ShapeKind::Ellipse.is_open());
    }

    #[test]
    fn statement_kind_defaults_to_sticky() {
        assert_eq!(StatementKind::default(), StatementKind::Sticky);
        assert_ne!(StatementKind::Text, StatementKind::Sticky);
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
