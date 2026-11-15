use doodle_types::{Pose, StrokeGeometry, StrokePoint, StrokeStyle};

/// Brush behavior settings. All values are bounded before use.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BrushSettings {
    pub base_width: f32,
    pub pressure_sensitivity: f32,
    pub smoothing: f32,
    pub taper_start: f32,
    pub taper_end: f32,
}

impl Default for BrushSettings {
    fn default() -> Self {
        Self {
            base_width: 4.0,
            pressure_sensitivity: 0.65,
            smoothing: 0.35,
            taper_start: 0.08,
            taper_end: 0.0,
        }
    }
}

impl BrushSettings {
    #[must_use]
    pub fn sanitized(self) -> Self {
        Self {
            base_width: if self.base_width.is_finite() {
                self.base_width.clamp(0.5, 64.0)
            } else {
                4.0
            },
            pressure_sensitivity: if self.pressure_sensitivity.is_finite() {
                self.pressure_sensitivity.clamp(0.0, 1.0)
            } else {
                0.65
            },
            smoothing: if self.smoothing.is_finite() {
                self.smoothing.clamp(0.0, 1.0)
            } else {
                0.35
            },
            taper_start: if self.taper_start.is_finite() {
                self.taper_start.clamp(0.0, 0.45)
            } else {
                0.08
            },
            taper_end: if self.taper_end.is_finite() {
                self.taper_end.clamp(0.0, 0.45)
            } else {
                0.0
            },
        }
    }

    #[must_use]
    pub fn style(self, rgba: [u8; 4]) -> StrokeStyle {
        StrokeStyle {
            rgba,
            base_width: self.sanitized().base_width,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RawSample {
    pub pos: doodle_types::Point,
    /// Normalized hardware pressure. `None` means unavailable, not zero.
    pub pressure: Option<f32>,
    pub time_s: f64,
}

impl RawSample {
    #[must_use]
    pub fn new(pos: doodle_types::Point, pressure: Option<f32>, time_s: f64) -> Self {
        let pos = if pos.is_finite() {
            pos
        } else {
            doodle_types::Point::default()
        };
        Self {
            pos,
            pressure: pressure.filter(|value| value.is_finite()),
            time_s: if time_s.is_finite() { time_s } else { 0.0 },
        }
    }
}

/// Collects pointer samples into a canonical stroke.
///
/// The preview is produced with the same `build_geometry` call used at finalization, so the
/// preview-to-final change is limited to the newly appended samples and taper envelope.
#[derive(Debug, Clone)]
pub struct StrokeBuilder {
    settings: BrushSettings,
    min_distance: f64,
    anchor: doodle_types::Point,
    smoothed: doodle_types::Point,
    last_raw: doodle_types::Point,
    last_time: f64,
    start_time: f64,
    speed_ema: f64,
    width_ema: f32,
    last_pressure: f32,
    points: Vec<(doodle_types::Point, f32)>,
    has_hardware_pressure: bool,
    saw_pressure: bool,
}

impl StrokeBuilder {
    #[must_use]
    pub fn new(settings: BrushSettings, start: RawSample, min_distance: f64) -> Self {
        let settings = settings.sanitized();
        let normalized_pressure = start
            .pressure
            .map(|p| p.clamp(0.0, 1.0) * 0.9 + 0.1)
            .unwrap_or(0.85);
        let factor = (1.0 - settings.pressure_sensitivity)
            + settings.pressure_sensitivity * normalized_pressure;
        let width = (settings.base_width * factor).clamp(settings.base_width * 0.3, 400.0);
        Self {
            settings,
            min_distance: min_distance.max(0.05),
            anchor: start.pos,
            smoothed: start.pos,
            last_raw: start.pos,
            last_time: start.time_s,
            start_time: start.time_s,
            speed_ema: 0.0,
            width_ema: width,
            last_pressure: normalized_pressure,
            points: vec![(doodle_types::Point::default(), width)],
            has_hardware_pressure: start.pressure.is_some(),
            saw_pressure: start.pressure.is_some(),
        }
    }

    #[must_use]
    pub fn settings(&self) -> BrushSettings {
        self.settings
    }

    pub fn push(&mut self, sample: RawSample) {
        if !sample.pos.is_finite() || !sample.time_s.is_finite() {
            return;
        }
        let dt = (sample.time_s - self.last_time).max(1e-4);
        let dist = sample.pos.distance(self.last_raw);
        if dist < self.min_distance {
            // Keep the latest timestamp so a paused pointer does not inflate the next speed sample.
            self.last_time = sample.time_s;
            if sample.pressure.is_some() && !self.saw_pressure {
                self.saw_pressure = true;
                self.has_hardware_pressure = true;
            }
            return;
        }

        let instant_speed = dist / dt;
        if self.speed_ema <= f64::EPSILON {
            self.speed_ema = instant_speed;
        } else {
            self.speed_ema = self.speed_ema * 0.72 + instant_speed * 0.28;
        }

        let position_alpha = 1.0 - f64::from(self.settings.smoothing) * 0.6;
        self.smoothed = doodle_types::Point::new(
            self.smoothed.x + (sample.pos.x - self.smoothed.x) * position_alpha,
            self.smoothed.y + (sample.pos.y - self.smoothed.y) * position_alpha,
        );

        let pressure = self.resolved_pressure(sample);
        let sensitivity = f64::from(self.settings.pressure_sensitivity);
        let factor = (1.0 - sensitivity) + sensitivity * pressure;
        let width = (f64::from(self.settings.base_width) * factor)
            .clamp(f64::from(self.settings.base_width) * 0.3, 400.0) as f32;
        let width_alpha = 0.35 + self.settings.smoothing * 0.25;
        self.width_ema = self.width_ema * (1.0 - width_alpha) + width * width_alpha;

        self.points.push((
            doodle_types::Point::new(
                self.smoothed.x - self.anchor.x,
                self.smoothed.y - self.anchor.y,
            ),
            self.width_ema,
        ));
        self.last_raw = sample.pos;
        self.last_time = sample.time_s;
    }

    fn resolved_pressure(&mut self, sample: RawSample) -> f64 {
        if let Some(pressure) = sample.pressure.filter(|value| value.is_finite()) {
            self.saw_pressure = true;
            self.has_hardware_pressure = true;
            let pressure = f64::from(pressure.clamp(0.0, 1.0)) * 0.9 + 0.1;
            self.last_pressure = pressure as f32;
            return pressure;
        }
        if self.has_hardware_pressure {
            // A pen stroke keeps its last known pressure response when a sample lacks it.
            return f64::from(self.last_pressure).clamp(0.1, 1.0);
        }
        let speed_norm =
            (self.speed_ema / (f64::from(self.settings.base_width) * 9.0)).clamp(0.0, 3.0);
        let speed_factor = 1.0 / (1.0 + 0.55 * speed_norm);
        let elapsed = (sample.time_s - self.start_time).clamp(0.0, 1.6);
        let envelope = 0.78 + 0.34 * (elapsed / 1.6);
        (speed_factor * envelope).clamp(0.22, 1.25)
    }

    #[must_use]
    pub fn current_geometry(&self) -> StrokeGeometry {
        self.build_geometry()
    }

    /// Anchor of the in-progress stroke in world space.
    #[must_use]
    pub fn current_pose(&self) -> Pose {
        Pose::new(self.anchor.x, self.anchor.y)
    }

    /// Consume the builder, producing the world pose and immutable local-space geometry.
    #[must_use]
    pub fn finish(self) -> (Pose, StrokeGeometry) {
        let geometry = self.build_geometry();
        (Pose::new(self.anchor.x, self.anchor.y), geometry)
    }

    fn build_geometry(&self) -> StrokeGeometry {
        if self.points.len() == 1 {
            let width = self.points[0].1.clamp(0.35, 400.0);
            return StrokeGeometry::new(vec![StrokePoint {
                x: 0.0,
                y: 0.0,
                width,
            }]);
        }
        let mut lengths = Vec::with_capacity(self.points.len());
        let mut total = 0.0_f64;
        lengths.push(0.0);
        for window in self.points.windows(2) {
            total += window[0].0.distance(window[1].0);
            lengths.push(total);
        }
        let total = total.max(1e-6);
        let mut points = Vec::with_capacity(self.points.len());
        for (index, (point, width)) in self.points.iter().enumerate() {
            let t = (lengths[index] / total).clamp(0.0, 1.0);
            let taper = taper_factor(
                t,
                f64::from(self.settings.taper_start),
                f64::from(self.settings.taper_end),
            );
            points.push(StrokePoint {
                x: point.x,
                y: point.y,
                width: (f64::from(*width) * taper).clamp(0.35, 400.0) as f32,
            });
        }
        StrokeGeometry::new(points)
    }

    #[must_use]
    pub fn point_count(&self) -> usize {
        self.points.len()
    }

    #[must_use]
    pub fn saw_hardware_pressure(&self) -> bool {
        self.saw_pressure
    }
}

fn taper_factor(t: f64, start: f64, end: f64) -> f64 {
    let mut factor = 1.0;
    if start > 1e-4 {
        let s = (t / start).clamp(0.0, 1.0);
        factor *= 0.35 + 0.65 * smoothstep(s);
    }
    if end > 1e-4 {
        let e = ((1.0 - t) / end).clamp(0.0, 1.0);
        factor *= 0.35 + 0.65 * smoothstep(e);
    }
    factor
}

fn smoothstep(t: f64) -> f64 {
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;
    use doodle_types::Point;

    fn trace(points: &[(f64, f64)], dt: f64) -> StrokeBuilder {
        let mut builder = StrokeBuilder::new(
            BrushSettings::default(),
            RawSample::new(Point::new(points[0].0, points[0].1), None, 0.0),
            0.5,
        );
        for (index, (x, y)) in points.iter().enumerate().skip(1) {
            builder.push(RawSample::new(Point::new(*x, *y), None, index as f64 * dt));
        }
        builder
    }

    #[test]
    fn tap_produces_a_dot() {
        let builder = StrokeBuilder::new(
            BrushSettings::default(),
            RawSample::new(Point::new(10.0, 10.0), None, 0.0),
            0.5,
        );
        let (pose, geometry) = builder.finish();
        assert_eq!(pose, Pose::new(10.0, 10.0));
        assert_eq!(geometry.points.len(), 1);
        assert!(geometry.points[0].width > 0.0);
        assert!(geometry.is_valid());
    }

    #[test]
    fn default_mouse_trace_ends_heavier() {
        let points: Vec<(f64, f64)> = (0..40).map(|i| (i as f64 * 6.0, 0.0)).collect();
        let builder = trace(&points, 0.016);
        let (_, geometry) = builder.finish();
        let first = geometry.points[0].width;
        let last = geometry.points.last().expect("last").width;
        assert!(last > first, "last {last} should exceed first {first}");
    }

    #[test]
    fn hardware_pressure_is_used() {
        let mut builder = StrokeBuilder::new(
            BrushSettings::default(),
            RawSample::new(Point::new(0.0, 0.0), Some(0.1), 0.0),
            0.1,
        );
        builder.push(RawSample::new(Point::new(5.0, 0.0), Some(1.0), 0.016));
        builder.push(RawSample::new(Point::new(10.0, 0.0), Some(0.2), 0.032));
        assert!(builder.saw_hardware_pressure());
        let (_, geometry) = builder.finish();
        assert!(geometry.points[1].width > geometry.points[0].width);
    }

    #[test]
    fn first_point_and_tap_follow_hardware_pressure() {
        let settings = BrushSettings::default();
        let light = StrokeBuilder::new(
            settings,
            RawSample::new(Point::new(0.0, 0.0), Some(0.0), 0.0),
            0.5,
        );
        let heavy = StrokeBuilder::new(
            settings,
            RawSample::new(Point::new(0.0, 0.0), Some(1.0), 0.0),
            0.5,
        );
        let (_, light_geometry) = light.finish();
        let (_, heavy_geometry) = heavy.finish();
        assert!(
            light_geometry.points[0].width < heavy_geometry.points[0].width,
            "a pressure tap must not use a fixed width"
        );
        assert!(
            light_geometry.points[0].width < settings.base_width,
            "first stored point must match the light start pressure"
        );
    }

    #[test]
    fn a_missing_pressure_sample_keeps_the_last_pen_pressure() {
        let mut builder = StrokeBuilder::new(
            BrushSettings {
                pressure_sensitivity: 0.65,
                taper_start: 0.0,
                ..BrushSettings::default()
            },
            RawSample::new(Point::new(0.0, 0.0), Some(1.0), 0.0),
            0.1,
        );
        builder.push(RawSample::new(Point::new(5.0, 0.0), None, 0.016));
        let (_, geometry) = builder.finish();
        let first = geometry.points[0].width;
        let second = geometry.points[1].width;
        assert!(
            (first - second).abs() < 0.5,
            "pressure glitch caused a width notch: {first} vs {second}"
        );
    }

    #[test]
    fn non_finite_settings_are_sanitized() {
        let sanitized = BrushSettings {
            pressure_sensitivity: f32::NAN,
            smoothing: f32::INFINITY,
            taper_start: f32::NAN,
            base_width: f32::NAN,
            ..BrushSettings::default()
        }
        .sanitized();
        assert!(sanitized.base_width.is_finite());
        assert!(sanitized.pressure_sensitivity.is_finite());
        assert!(sanitized.smoothing.is_finite());
        assert!(sanitized.taper_start.is_finite());
        let sample = RawSample::new(Point::new(f64::NAN, 0.0), Some(f32::NAN), f64::NAN);
        assert!(sample.pos.is_finite());
        assert_eq!(sample.pressure, None);
        assert!(sample.time_s.is_finite());
    }

    #[test]
    fn explicit_end_taper_thins_the_endpoint() {
        let settings = BrushSettings {
            taper_end: 0.3,
            ..BrushSettings::default()
        };
        let mut builder = StrokeBuilder::new(
            settings,
            RawSample::new(Point::new(0.0, 0.0), None, 0.0),
            0.5,
        );
        for index in 1..30 {
            builder.push(RawSample::new(
                Point::new(index as f64 * 4.0, 0.0),
                None,
                index as f64 * 0.016,
            ));
        }
        let (_, geometry) = builder.finish();
        assert!(geometry.points[0].width > geometry.points.last().expect("last").width);
    }

    #[test]
    fn duplicate_samples_do_not_grow_the_stroke() {
        let mut builder = StrokeBuilder::new(
            BrushSettings::default(),
            RawSample::new(Point::new(0.0, 0.0), None, 0.0),
            0.5,
        );
        for index in 0..100 {
            builder.push(RawSample::new(
                Point::new(0.0, 0.0),
                None,
                index as f64 * 0.01,
            ));
        }
        assert_eq!(builder.point_count(), 1);
    }
}
