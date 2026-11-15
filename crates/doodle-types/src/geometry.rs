use serde::{Deserialize, Serialize};

use crate::limits::MAX_ABS_COORDINATE;

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
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
    pub fn distance(self, other: Self) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        dx.hypot(dy)
    }
}

/// Axis-aligned rectangle in world space.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

impl Rect {
    #[must_use]
    pub fn new(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Self {
        Self {
            min_x,
            min_y,
            max_x,
            max_y,
        }
    }

    #[must_use]
    pub fn from_point(p: Point) -> Self {
        Self::new(p.x, p.y, p.x, p.y)
    }

    #[must_use]
    pub fn width(self) -> f64 {
        self.max_x - self.min_x
    }

    #[must_use]
    pub fn height(self) -> f64 {
        self.max_y - self.min_y
    }

    #[must_use]
    pub fn center(self) -> Point {
        Point::new(
            (self.min_x + self.max_x) * 0.5,
            (self.min_y + self.max_y) * 0.5,
        )
    }

    #[must_use]
    pub fn union(self, other: Self) -> Self {
        Self::new(
            self.min_x.min(other.min_x),
            self.min_y.min(other.min_y),
            self.max_x.max(other.max_x),
            self.max_y.max(other.max_y),
        )
    }

    #[must_use]
    pub fn expand(self, amount: f64) -> Self {
        Self::new(
            self.min_x - amount,
            self.min_y - amount,
            self.max_x + amount,
            self.max_y + amount,
        )
    }

    #[must_use]
    pub fn translate(self, dx: f64, dy: f64) -> Self {
        Self::new(
            self.min_x + dx,
            self.min_y + dy,
            self.max_x + dx,
            self.max_y + dy,
        )
    }

    #[must_use]
    pub fn contains(self, p: Point) -> bool {
        p.x >= self.min_x && p.x <= self.max_x && p.y >= self.min_y && p.y <= self.max_y
    }

    /// Interior overlap test. Touching edges are not an overlap.
    #[must_use]
    pub fn overlaps(self, other: Self) -> bool {
        self.min_x < other.max_x
            && self.max_x > other.min_x
            && self.min_y < other.max_y
            && self.max_y > other.min_y
    }

    #[must_use]
    pub fn intersects_rect(self, other: Self) -> bool {
        self.min_x <= other.max_x
            && self.max_x >= other.min_x
            && self.min_y <= other.max_y
            && self.max_y >= other.min_y
    }

    #[must_use]
    pub fn is_finite(self) -> bool {
        self.min_x.is_finite()
            && self.min_y.is_finite()
            && self.max_x.is_finite()
            && self.max_y.is_finite()
            && self.min_x <= self.max_x
            && self.min_y <= self.max_y
            && self.min_x.abs() <= MAX_ABS_COORDINATE
            && self.min_y.abs() <= MAX_ABS_COORDINATE
            && self.max_x.abs() <= MAX_ABS_COORDINATE
            && self.max_y.abs() <= MAX_ABS_COORDINATE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlap_excludes_touching_edges() {
        let a = Rect::new(0.0, 0.0, 10.0, 10.0);
        let b = Rect::new(10.0, 0.0, 20.0, 10.0);
        assert!(!a.overlaps(b));
        assert!(a.intersects_rect(b));
        assert!(a.overlaps(Rect::new(5.0, 5.0, 15.0, 15.0)));
    }

    #[test]
    fn union_and_expand() {
        let a = Rect::new(0.0, 0.0, 1.0, 1.0);
        let b = Rect::new(-2.0, 3.0, 4.0, 5.0);
        assert_eq!(a.union(b), Rect::new(-2.0, 0.0, 4.0, 5.0));
        assert_eq!(a.expand(1.0), Rect::new(-1.0, -1.0, 2.0, 2.0));
    }
}
