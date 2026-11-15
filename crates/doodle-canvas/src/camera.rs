use doodle_types::{MAX_ABS_COORDINATE, MAX_ZOOM, MIN_ZOOM, Point, Rect};

/// Viewport camera. World coordinates are `f64`; the renderer converts to camera-relative pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    pub center: Point,
    pub zoom: f64,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            center: Point::default(),
            zoom: 1.0,
        }
    }
}

impl Camera {
    #[must_use]
    pub fn new(center: Point, zoom: f64) -> Self {
        Self {
            center,
            zoom: zoom.clamp(MIN_ZOOM, MAX_ZOOM),
        }
    }

    #[must_use]
    pub fn world_to_screen(&self, world: Point, viewport: Rect) -> Point {
        let center_x = (viewport.min_x + viewport.max_x) * 0.5;
        let center_y = (viewport.min_y + viewport.max_y) * 0.5;
        Point::new(
            center_x + (world.x - self.center.x) * self.zoom,
            center_y + (world.y - self.center.y) * self.zoom,
        )
    }

    #[must_use]
    pub fn screen_to_world(&self, screen: Point, viewport: Rect) -> Point {
        let center_x = (viewport.min_x + viewport.max_x) * 0.5;
        let center_y = (viewport.min_y + viewport.max_y) * 0.5;
        Point::new(
            self.center.x + (screen.x - center_x) / self.zoom,
            self.center.y + (screen.y - center_y) / self.zoom,
        )
    }

    /// Screen-space rectangle currently visible in world coordinates.
    #[must_use]
    pub fn visible_world(&self, viewport: Rect) -> Rect {
        let top_left = self.screen_to_world(Point::new(viewport.min_x, viewport.min_y), viewport);
        let bottom_right =
            self.screen_to_world(Point::new(viewport.max_x, viewport.max_y), viewport);
        Rect::new(top_left.x, top_left.y, bottom_right.x, bottom_right.y)
    }

    pub fn pan_by_screen_delta(&mut self, dx_screen: f64, dy_screen: f64) {
        if !dx_screen.is_finite() || !dy_screen.is_finite() {
            return;
        }
        self.center.x -= dx_screen / self.zoom;
        self.center.y -= dy_screen / self.zoom;
        self.center = clamp_center(self.center);
    }

    /// Multiplicative zoom that keeps the world point under `cursor_screen` fixed.
    pub fn zoom_at(&mut self, cursor_screen: Point, viewport: Rect, factor: f64) {
        if !factor.is_finite() || factor <= 0.0 {
            return;
        }
        let before = self.screen_to_world(cursor_screen, viewport);
        let new_zoom = (self.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
        if (new_zoom - self.zoom).abs() < f64::EPSILON {
            return;
        }
        self.zoom = new_zoom;
        let after = self.screen_to_world(cursor_screen, viewport);
        self.center.x += before.x - after.x;
        self.center.y += before.y - after.y;
        self.center = clamp_center(self.center);
    }

    pub fn reset(&mut self) {
        self.center = Point::default();
        self.zoom = 1.0;
    }

    /// Fit a world-space rect inside the viewport with padding; empty/invalid input is ignored.
    pub fn fit(&mut self, bounds: Rect, viewport: Rect, padding_px: f64) {
        if !bounds.is_finite() || viewport.width() <= 0.0 || viewport.height() <= 0.0 {
            return;
        }
        let width = bounds.width().max(1.0);
        let height = bounds.height().max(1.0);
        let usable_w = (viewport.width() - padding_px * 2.0).max(32.0);
        let usable_h = (viewport.height() - padding_px * 2.0).max(32.0);
        let zoom = (usable_w / width).min(usable_h / height);
        self.zoom = zoom.clamp(MIN_ZOOM, MAX_ZOOM);
        self.center = bounds.center();
    }
}

fn clamp_center(center: Point) -> Point {
    Point::new(
        center.x.clamp(-MAX_ABS_COORDINATE, MAX_ABS_COORDINATE),
        center.y.clamp(-MAX_ABS_COORDINATE, MAX_ABS_COORDINATE),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn viewport() -> Rect {
        Rect::new(0.0, 0.0, 1000.0, 800.0)
    }

    #[test]
    fn world_screen_round_trip() {
        let camera = Camera::new(Point::new(120.0, -40.0), 2.5);
        let world = Point::new(500.0, 300.0);
        let screen = camera.world_to_screen(world, viewport());
        let back = camera.screen_to_world(screen, viewport());
        assert!((back.x - world.x).abs() < 1e-9);
        assert!((back.y - world.y).abs() < 1e-9);
    }

    #[test]
    fn zoom_keeps_cursor_world_point_fixed() {
        let mut camera = Camera::default();
        let vp = viewport();
        let cursor = Point::new(321.0, 654.0);
        let before = camera.screen_to_world(cursor, vp);
        camera.zoom_at(cursor, vp, 1.8);
        let after = camera.screen_to_world(cursor, vp);
        assert!((before.x - after.x).abs() < 1e-6);
        assert!((before.y - after.y).abs() < 1e-6);
    }

    #[test]
    fn zoom_is_clamped() {
        let mut camera = Camera::default();
        camera.zoom_at(Point::new(0.0, 0.0), viewport(), 1e9);
        assert_eq!(camera.zoom, MAX_ZOOM);
        camera.zoom_at(Point::new(0.0, 0.0), viewport(), 1e-9);
        assert_eq!(camera.zoom, MIN_ZOOM);
    }

    #[test]
    fn fit_contains_bounds() {
        let mut camera = Camera::default();
        let bounds = Rect::new(-200.0, -100.0, 200.0, 100.0);
        camera.fit(bounds, viewport(), 40.0);
        let visible = camera.visible_world(viewport());
        assert!(visible.min_x <= bounds.min_x);
        assert!(visible.max_x >= bounds.max_x);
        assert!(visible.min_y <= bounds.min_y);
        assert!(visible.max_y >= bounds.max_y);
    }
}
