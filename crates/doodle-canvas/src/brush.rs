use doodle_types::{Pose, StrokeGeometry, StrokePoint, StrokeStyle};

/// Brush behavior settings. All values are bounded before use.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BrushSettings {
    pub base_width: f32,
    pub pressure_sensitivity: f32,
    pub smoothing: f32,
    pub taper_start: f32,
    pub taper_end: f32,
    /// Whether mouse samples simulate pressure from speed. Hardware pen pressure is always
    /// honoured when present; this only switches the fallback for devices without one.
    pub simulate_pressure: bool,
}

impl Default for BrushSettings {
    fn default() -> Self {
        Self {
            base_width: 4.0,
            pressure_sensitivity: 0.8,
            smoothing: 0.35,
            taper_start: 0.08,
            taper_end: 0.0,
            simulate_pressure: true,
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
                0.8
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
            simulate_pressure: self.simulate_pressure,
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
            .unwrap_or(if settings.simulate_pressure {
                0.85
            } else {
                1.0
            });
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
            // Responsive enough that a change of pace shows within a few samples: slow strokes
            // thicken, fast ones thin, and neither waits for an elapsed-time envelope.
            self.speed_ema = self.speed_ema * 0.65 + instant_speed * 0.35;
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
        if !self.settings.simulate_pressure {
            // Pressure off is a constant-width brush. Hardware pressure, when present, was
            // handled above and still wins.
            return 1.0;
        }
        // Directional mouse simulation: slow strokes are thick, fast strokes are thin. There is
        // no elapsed-time term, so a stroke never grows just because it took longer. Speed is
        // converted to screen points per second so the response feels the same at every zoom:
        // the builder's `min_distance` is derived from the zoom (`0.7 / zoom`).
        let screen_speed = self.speed_ema * 0.7 / self.min_distance;
        let speed_norm = (screen_speed / 300.0).clamp(0.0, 1.0);
        (1.6 - 1.3 * speed_norm).clamp(0.25, 1.6)
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
        // At low zoom accepted pointer samples are far apart in world space, so straight
        // segments between them lose the curve. Refine each segment with a Catmull-Rom spline
        // through its neighbours at roughly three screen points per step (the builder's
        // `min_distance` is the accepted sample spacing, derived from the zoom).
        let dense = self.densified_points();
        let mut lengths = Vec::with_capacity(dense.len());
        let mut total = 0.0_f64;
        lengths.push(0.0);
        for window in dense.windows(2) {
            total += window[0].0.distance(window[1].0);
            lengths.push(total);
        }
        let total = total.max(1e-6);
        let mut points = Vec::with_capacity(dense.len());
        for (index, (point, width)) in dense.iter().enumerate() {
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

    /// The accepted samples with each segment refined by a Catmull-Rom spline. Steps are at
    /// most four times `min_distance` (about three screen points) and capped per segment so a
    /// single long jump cannot explode the point count.
    fn densified_points(&self) -> Vec<(doodle_types::Point, f32)> {
        const STEP_FACTOR: f64 = 4.0;
        const MAX_STEPS: usize = 48;
        let mut dense = Vec::with_capacity(self.points.len() * 2);
        dense.push(self.points[0]);
        let step = (self.min_distance * STEP_FACTOR).max(1e-3);
        for index in 0..self.points.len() - 1 {
            let (p1, w1) = self.points[index];
            let (p2, w2) = self.points[index + 1];
            let p0 = if index > 0 {
                self.points[index - 1].0
            } else {
                doodle_types::Point::new(2.0 * p1.x - p2.x, 2.0 * p1.y - p2.y)
            };
            let p3 = if index + 2 < self.points.len() {
                self.points[index + 2].0
            } else {
                doodle_types::Point::new(2.0 * p2.x - p1.x, 2.0 * p2.y - p1.y)
            };
            let steps = ((p1.distance(p2) / step).ceil() as usize).clamp(1, MAX_STEPS);
            for step_index in 1..=steps {
                let t = step_index as f64 / steps as f64;
                dense.push((catmull_rom(p0, p1, p2, p3, t), w1 + (w2 - w1) * t as f32));
            }
        }
        dense
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

/// Uniform Catmull-Rom interpolation between `p1` and `p2`, with `p0`/`p3` giving the tangents.
fn catmull_rom(
    p0: doodle_types::Point,
    p1: doodle_types::Point,
    p2: doodle_types::Point,
    p3: doodle_types::Point,
    t: f64,
) -> doodle_types::Point {
    let t2 = t * t;
    let t3 = t2 * t;
    let at = |a: f64, b: f64, c: f64, d: f64| {
        0.5 * (2.0 * b
            + (-a + c) * t
            + (2.0 * a - 5.0 * b + 4.0 * c - d) * t2
            + (-a + 3.0 * b - 3.0 * c + d) * t3)
    };
    doodle_types::Point::new(at(p0.x, p1.x, p2.x, p3.x), at(p0.y, p1.y, p2.y, p3.y))
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
    fn slow_strokes_are_thicker_than_fast_strokes() {
        // Same distance travelled, different pace. Slow: 1 world point every 50 ms (20/s).
        // Fast: 20 world points every 16 ms (1250/s).
        let slow_points: Vec<(f64, f64)> = (0..40).map(|i| (i as f64, 0.0)).collect();
        let fast_points: Vec<(f64, f64)> = (0..40).map(|i| (i as f64 * 20.0, 0.0)).collect();
        let slow = trace(&slow_points, 0.05);
        let fast = trace(&fast_points, 0.016);
        let (_, slow_geometry) = slow.finish();
        let (_, fast_geometry) = fast.finish();
        let mean = |geometry: &StrokeGeometry| {
            geometry.points.iter().map(|point| point.width).sum::<f32>()
                / geometry.points.len() as f32
        };
        let slow_mean = mean(&slow_geometry);
        let fast_mean = mean(&fast_geometry);
        assert!(
            slow_mean > fast_mean * 1.2,
            "slow mean {slow_mean} should clearly exceed fast mean {fast_mean}"
        );
    }

    #[test]
    fn pressure_off_keeps_a_constant_width() {
        let settings = BrushSettings {
            simulate_pressure: false,
            smoothing: 0.0,
            taper_start: 0.0,
            taper_end: 0.0,
            ..BrushSettings::default()
        };
        for spacing in [1.0, 20.0] {
            let mut builder = StrokeBuilder::new(
                settings,
                RawSample::new(Point::new(0.0, 0.0), None, 0.0),
                0.5,
            );
            for index in 1..40 {
                builder.push(RawSample::new(
                    Point::new(index as f64 * spacing, 0.0),
                    None,
                    index as f64 * 0.016,
                ));
            }
            let (_, geometry) = builder.finish();
            let min = geometry
                .points
                .iter()
                .map(|point| point.width)
                .fold(f32::INFINITY, f32::min);
            let max = geometry
                .points
                .iter()
                .map(|point| point.width)
                .fold(0.0, f32::max);
            assert!(
                (max - min) < 0.05,
                "pressure off should not vary width: {min}..{max} at spacing {spacing}"
            );
            assert!((min - settings.base_width).abs() < 0.05);
        }
    }

    #[test]
    fn pressure_off_still_honours_hardware_pressure() {
        let settings = BrushSettings {
            simulate_pressure: false,
            taper_start: 0.0,
            ..BrushSettings::default()
        };
        let mut builder = StrokeBuilder::new(
            settings,
            RawSample::new(Point::new(0.0, 0.0), Some(0.0), 0.0),
            0.1,
        );
        builder.push(RawSample::new(Point::new(5.0, 0.0), Some(1.0), 0.016));
        assert!(builder.saw_hardware_pressure());
        let (_, geometry) = builder.finish();
        assert!(geometry.points[1].width > geometry.points[0].width);
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
            simulate_pressure: false,
            ..BrushSettings::default()
        }
        .sanitized();
        assert!(sanitized.base_width.is_finite());
        assert!(sanitized.pressure_sensitivity.is_finite());
        assert!(sanitized.smoothing.is_finite());
        assert!(sanitized.taper_start.is_finite());
        assert!(!sanitized.simulate_pressure);
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
    fn sparse_curved_samples_stay_curved_at_low_zoom() {
        // A low zoom makes the accepted sample spacing (0.7 / zoom) large, so the builder sees
        // only a few far-apart samples. The rendered geometry must still bend off the chord.
        let mut builder = StrokeBuilder::new(
            BrushSettings {
                smoothing: 0.0,
                ..BrushSettings::default()
            },
            RawSample::new(Point::new(0.0, 0.0), None, 0.0),
            14.0,
        );
        builder.push(RawSample::new(Point::new(100.0, 0.0), None, 0.05));
        builder.push(RawSample::new(Point::new(100.0, 100.0), None, 0.1));
        let (_, geometry) = builder.finish();
        assert!(
            geometry.points.len() > 3,
            "sparse samples were not refined: {} points",
            geometry.points.len()
        );
        let off_chord = geometry
            .points
            .iter()
            .any(|point| point.y < -1.0 && point.x > 1.0 && point.x < 99.0);
        assert!(
            off_chord,
            "the first chord was rendered straight; no intermediate point left y = 0"
        );
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
