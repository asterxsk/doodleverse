use doodle_document::BoardProjection;
use doodle_files::ImportPayload;
use doodle_types::{Point, Rect};

/// Minimum gap between placement rectangles before overlap is allowed (touching edges are fine).
pub const PLACEMENT_EPSILON: f64 = 0.5;

#[must_use]
pub fn board_is_empty(projection: &BoardProjection) -> bool {
    projection.content_bounds().is_none()
}

/// Top-left translation that centers the imported payload on the cursor.
#[must_use]
pub fn placement_translation(payload: &ImportPayload, cursor: Point) -> (f64, f64) {
    (
        cursor.x - payload.bounds.width() * 0.5,
        cursor.y - payload.bounds.height() * 0.5,
    )
}

#[must_use]
pub fn placement_rect(payload: &ImportPayload, translation: (f64, f64)) -> Rect {
    payload.bounds.translate(translation.0, translation.1)
}

#[must_use]
pub fn placement_is_valid(candidate: Rect, occupied: &[Rect]) -> bool {
    if !candidate.is_finite() {
        return false;
    }
    occupied
        .iter()
        .all(|region| !candidate.expand(-PLACEMENT_EPSILON).overlaps(*region))
}

#[must_use]
pub fn next_order_key(projection: &BoardProjection) -> i64 {
    projection
        .objects
        .iter()
        .map(|object| object.object.order_key)
        .max()
        .unwrap_or(0)
        .saturating_add(16)
}

/// Deterministic shared-statement dimensions from its text. Pinned answers store this size so all
/// peers lay out the same statement box.
#[must_use]
pub fn statement_size(text: &str) -> (f64, f64) {
    let chars_per_line = 42usize;
    let mut lines = 1usize;
    for line in text.lines() {
        let count = line.chars().count().max(1);
        lines += count.div_ceil(chars_per_line).saturating_sub(1);
        if line.is_empty() {
            lines += 1;
        }
    }
    let extra_breaks = text.matches('\n').count();
    let height = 64.0 + (lines + extra_breaks) as f64 * 19.0;
    (340.0, height.clamp(96.0, 520.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use doodle_files::ImportPayload;
    use doodle_types::{ObjectId, ObjectKind, Pose, StoredObject};

    fn payload(bounds: Rect) -> ImportPayload {
        let wrapper = ObjectId::new();
        let objects = vec![StoredObject {
            id: wrapper,
            parent: None,
            order_key: 0,
            pose: Pose::default(),
            kind: ObjectKind::Group,
        }];
        ImportPayload {
            objects,
            root_ids: vec![wrapper],
            bounds,
            object_count: 1,
        }
    }

    #[test]
    fn centered_translation_matches_bounds() {
        let payload = payload(Rect::new(0.0, 0.0, 200.0, 100.0));
        let translation = placement_translation(&payload, Point::new(500.0, 300.0));
        assert_eq!(translation, (400.0, 250.0));
        assert_eq!(
            placement_rect(&payload, translation),
            Rect::new(400.0, 250.0, 600.0, 350.0)
        );
    }

    #[test]
    fn overlap_and_touching_rules() {
        let occupied = [Rect::new(0.0, 0.0, 100.0, 100.0)];
        assert!(!placement_is_valid(
            Rect::new(50.0, 50.0, 150.0, 150.0),
            &occupied
        ));
        assert!(placement_is_valid(
            Rect::new(100.0, 0.0, 200.0, 100.0),
            &occupied
        ));
        assert!(placement_is_valid(
            Rect::new(150.0, 0.0, 250.0, 100.0),
            &occupied
        ));
    }

    #[test]
    fn placement_inside_epsilon_gap_is_allowed() {
        let occupied = [Rect::new(0.0, 0.0, 100.0, 100.0)];
        // 0.2 units of overlap is treated as touching.
        assert!(placement_is_valid(
            Rect::new(99.8, 0.0, 200.0, 100.0),
            &occupied
        ));
    }

    #[test]
    fn statement_size_is_bounded() {
        let (width, height) = statement_size("short");
        assert_eq!(width, 340.0);
        assert!(height >= 96.0);
        let (_, tall) = statement_size(&"word ".repeat(400));
        assert_eq!(tall, 520.0);
    }
}
