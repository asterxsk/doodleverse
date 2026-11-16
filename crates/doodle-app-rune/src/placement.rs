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

/// Natural width of a pinned answer's note and the style its text is laid out with. The style
/// matches `canvas::statement_texts`, so the measured height is the height the board actually
/// draws.
pub const STATEMENT_WIDTH: f32 = 340.0;
const STATEMENT_TEXT: rune::TextStyle = rune::TextStyle::new(14.0, 500.0);
/// The statement's text starts 12 pt in and 10 pt down, and its box adds the same 10 pt at the
/// bottom; 24 pt of the width is that horizontal inset.
const STATEMENT_INSET_X: f32 = 24.0;
const STATEMENT_INSET_Y: f32 = 20.0;
const STATEMENT_MIN_HEIGHT: f64 = 96.0;
pub const STATEMENT_MAX_HEIGHT: f64 = 520.0;

/// A pinned answer is sized to its wrapped text plus the box's padding, so a short answer is a
/// small note rather than a tall card with an empty footer. Long answers stay clamped, as before.
pub fn statement_size(text: &str) -> (f64, f64) {
    statement_size_at(text, STATEMENT_WIDTH)
}

/// The note's size when its text wraps at `width`. The pin placement narrows the note when no
/// side of the card has room for the natural width, re-measuring so the text still fits.
pub fn statement_size_at(text: &str, width: f32) -> (f64, f64) {
    let natural = statement_natural_height(text, width);
    let height = natural.clamp(STATEMENT_MIN_HEIGHT, STATEMENT_MAX_HEIGHT);
    (f64::from(width), height)
}

/// The height `text` needs at `width` before the min/max clamp. A width whose text overflows the
/// clamp would be drawn clipped, so the placement rejects it instead of pinning a cut-off note.
pub fn statement_natural_height(text: &str, width: f32) -> f64 {
    let (_, text_height) =
        rune::text::measure(text, &[], STATEMENT_TEXT, Some(width - STATEMENT_INSET_X));
    f64::from(text_height + STATEMENT_INSET_Y)
}
