use doodle_document::BoardProjection;
use doodle_files::ImportPayload;
use doodle_types::{Point, Rect};

pub const PLACEMENT_EPSILON: f64 = 0.5;

pub fn board_is_empty(projection: &BoardProjection) -> bool {
    projection.content_bounds().is_none()
}

pub fn placement_translation(payload: &ImportPayload, cursor: Point) -> (f64, f64) {
    (
        cursor.x - payload.bounds.width() * 0.5,
        cursor.y - payload.bounds.height() * 0.5,
    )
}

pub fn placement_rect(payload: &ImportPayload, translation: (f64, f64)) -> Rect {
    payload.bounds.translate(translation.0, translation.1)
}

pub fn placement_is_valid(candidate: Rect, occupied: &[Rect]) -> bool {
    if !candidate.is_finite() {
        return false;
    }
    occupied
        .iter()
        .all(|region| !candidate.expand(-PLACEMENT_EPSILON).overlaps(*region))
}

pub fn next_order_key(projection: &BoardProjection) -> i64 {
    projection
        .objects
        .iter()
        .map(|object| object.object.order_key)
        .max()
        .unwrap_or(0)
        .saturating_add(16)
}

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
