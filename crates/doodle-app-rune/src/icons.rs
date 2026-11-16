//! Lucide icons, drawn as Rune shape strokes.
//!
//! The path data is vendored from Lucide (ISC; see `assets/icons/`) and keeps the set's 24-unit
//! view box, 2-unit stroke, round caps and round joins. [`icon`] scales both the geometry and the
//! stroke to the requested size, so a 16pt icon is a true 16pt Lucide glyph.
//!
//! Paths are parsed per view rather than cached: each glyph is a handful of cubic segments and
//! the data is static, so rebuilding it is cheaper than sharing a shape across views.

use rune::{Color, El, LineCap, Path, Shape, canvas};

/// Lucide's view box and stroke width, in icon units.
const VIEW_BOX: f32 = 24.0;
const STROKE: f32 = 2.0;

struct IconDef {
    name: &'static str,
    paths: &'static [&'static str],
}

/// One entry per vendored SVG in `assets/icons/lucide/`. A multi-element icon keeps one `d` per
/// element; `<circle>` and `<rect>` elements are written out as equivalent path data.
const ICONS: &[IconDef] = &[
    IconDef {
        name: "arrow-right",
        paths: &["M5 12h14", "m12 5 7 7-7 7"],
    },
    IconDef {
        name: "book-open",
        paths: &[
            "M12 5v16",
            "M20.001 19A2 2 0 0022 17V5a2 2 0 00-1.999-2L16 3.002A5 5 0 0012 5a5 5 0 00-4-2H4a2 2 0 00-2 2v12a2 2 0 001.999 2H8a5 5 0 014 2 5 5 0 014-2z",
        ],
    },
    IconDef {
        name: "circle",
        // `<circle cx="12" cy="12" r="10" />`
        paths: &["M2 12a10 10 0 1 0 20 0a10 10 0 1 0-20 0"],
    },
    IconDef {
        name: "circle-question-mark",
        paths: &[
            // `<circle cx="12" cy="12" r="10" />`
            "M2 12a10 10 0 1 0 20 0a10 10 0 1 0-20 0",
            "M9 9.5a3 2.5 0 016 0c0 2-2 2-3 3.5",
            "M12 17h.01",
        ],
    },
    IconDef {
        name: "diamond",
        paths: &[
            "M2.7 10.3a2.41 2.41 0 0 0 0 3.41l7.59 7.59a2.41 2.41 0 0 0 3.41 0l7.59-7.59a2.41 2.41 0 0 0 0-3.41l-7.59-7.59a2.41 2.41 0 0 0-3.41 0Z",
        ],
    },
    IconDef {
        name: "ellipsis-vertical",
        paths: &[
            // `<circle cx="12" cy="12" r="1" />`
            "M11 12a1 1 0 1 0 2 0a1 1 0 1 0-2 0",
            // `<circle cx="12" cy="5" r="1" />`
            "M11 5a1 1 0 1 0 2 0a1 1 0 1 0-2 0",
            // `<circle cx="12" cy="19" r="1" />`
            "M11 19a1 1 0 1 0 2 0a1 1 0 1 0-2 0",
        ],
    },
    IconDef {
        name: "eraser",
        paths: &[
            "M21 21H8a2 2 0 0 1-1.42-.587l-3.994-3.999a2 2 0 0 1 0-2.828l10-10a2 2 0 0 1 2.829 0l5.999 6a2 2 0 0 1 0 2.828L12.834 21",
            "m5.082 11.09 8.828 8.828",
        ],
    },
    IconDef {
        name: "hand",
        paths: &[
            "M18 11V6a2 2 0 0 0-2-2a2 2 0 0 0-2 2",
            "M14 10V4a2 2 0 0 0-2-2a2 2 0 0 0-2 2v2",
            "M10 10.5V6a2 2 0 0 0-2-2a2 2 0 0 0-2 2v8",
            "M18 8a2 2 0 1 1 4 0v6a8 8 0 0 1-8 8h-2c-2.8 0-4.5-.86-5.99-2.34l-3.6-3.6a2 2 0 0 1 2.83-2.82L7 15",
        ],
    },
    IconDef {
        name: "lock",
        paths: &[
            // `<rect width="18" height="11" x="3" y="11" rx="2" ry="2" />`
            "M5 11h14a2 2 0 0 1 2 2v7a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-7a2 2 0 0 1 2-2z",
            "M7 11V7a5 5 0 0 1 10 0v4",
        ],
    },
    IconDef {
        name: "map",
        paths: &[
            "M14.106 5.553a2 2 0 0 0 1.788 0l3.659-1.83A1 1 0 0 1 21 4.619v12.764a1 1 0 0 1-.553.894l-4.553 2.277a2 2 0 0 1-1.788 0l-4.212-2.106a2 2 0 0 0-1.788 0l-3.659 1.83A1 1 0 0 1 3 19.381V6.618a1 1 0 0 1 .553-.894l4.553-2.277a2 2 0 0 1 1.788 0z",
            "M15 5.764v15",
            "M9 3.236v15",
        ],
    },
    IconDef {
        name: "minus",
        paths: &["M5 12h14"],
    },
    IconDef {
        name: "mouse-pointer-2",
        paths: &[
            "M4.037 4.688a.495.495 0 0 1 .651-.651l16 6.5a.5.5 0 0 1-.063.947l-6.124 1.58a2 2 0 0 0-1.438 1.435l-1.579 6.126a.5.5 0 0 1-.947.063z",
        ],
    },
    IconDef {
        name: "pencil",
        paths: &[
            "M21.174 6.812a1 1 0 0 0-3.986-3.987L3.842 16.174a2 2 0 0 0-.5.83l-1.321 4.352a.5.5 0 0 0 .623.622l4.353-1.32a2 2 0 0 0 .83-.497z",
            "m15 5 4 4",
        ],
    },
    IconDef {
        name: "redo-2",
        paths: &[
            "m15 14 5-5-5-5",
            "M20 9H9.5A5.5 5.5 0 0 0 4 14.5A5.5 5.5 0 0 0 9.5 20H13",
        ],
    },
    IconDef {
        name: "square",
        paths: &[
            // `<rect width="18" height="18" x="3" y="3" rx="2" />`
            "M5 3h14a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2z",
        ],
    },
    IconDef {
        name: "sticky-note",
        paths: &[
            "M21 9a2.4 2.4 0 0 0-.706-1.706l-3.588-3.588A2.4 2.4 0 0 0 15 3H5a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2z",
            "M15 3v5a1 1 0 0 0 1 1h5",
        ],
    },
    IconDef {
        name: "type",
        paths: &[
            "M12 4v16",
            "M4 7V5a1 1 0 0 1 1-1h14a1 1 0 0 1 1 1v2",
            "M9 20h6",
        ],
    },
    IconDef {
        name: "undo-2",
        paths: &[
            "M9 14 4 9l5-5",
            "M4 9h10.5a5.5 5.5 0 0 1 5.5 5.5a5.5 5.5 0 0 1-5.5 5.5H11",
        ],
    },
    IconDef {
        name: "x",
        paths: &["M18 6 6 18", "m6 6 12 12"],
    },
];

/// The icon named `name` in `ink`, laid out at `size` points square. Panics on an icon that is
/// not vendored, so a typo fails loudly in development instead of drawing an empty box.
pub fn icon(name: &str, size: f32, ink: Color) -> El {
    // The surrounding control carries the accessible name; the glyph itself is decoration.
    canvas(shape(name, size, ink)).size(size, size).role("none")
}

fn shape(name: &str, size: f32, ink: Color) -> Shape {
    let def = ICONS
        .iter()
        .find(|def| def.name == name)
        .unwrap_or_else(|| panic!("icon {name:?} is not vendored"));
    // `Shape::view_box` scales the geometry but keeps stroke widths in points, so scale Lucide's
    // 2-unit stroke to the requested size here.
    let stroke = STROKE * size / VIEW_BOX;
    let mut shape = Shape::new().view_box(VIEW_BOX, VIEW_BOX);
    for d in def.paths {
        shape = shape.stroke_path(&parse_path(d), stroke, LineCap::Round, ink);
    }
    shape
}

/// Parses an SVG path into Rune's flattened line/cubic model. Every SVG path command is
/// supported (`M L H V C S Q T A Z`, absolute and relative); quadratics become equivalent cubics
/// and elliptical arcs become cubic segments of at most a quarter turn.
fn parse_path(d: &str) -> Path {
    let mut parser = Parser {
        bytes: d.as_bytes(),
        offset: 0,
    };
    let mut path = Path::new();
    let mut current = [0.0f32; 2];
    let mut start = [0.0f32; 2];
    let mut last_cubic = [0.0f32; 2];
    let mut last_quad = [0.0f32; 2];
    let mut previous = None;
    let mut ran = b' ';

    loop {
        parser.skip_separators();
        if parser.offset >= parser.bytes.len() {
            break;
        }
        let command = if parser.bytes[parser.offset].is_ascii_alphabetic() {
            let command = parser.bytes[parser.offset];
            parser.offset += 1;
            previous = Some(command);
            command
        } else {
            // A command may repeat its parameters without repeating the letter. After `M` the
            // implicit command is `L`, as SVG specifies.
            match previous {
                Some(command) => command,
                None => break,
            }
        };
        let relative = command.is_ascii_lowercase();
        let was_cubic = matches!(ran, b'C' | b'c' | b'S' | b's');
        let was_quad = matches!(ran, b'Q' | b'q' | b'T' | b't');

        match command.to_ascii_uppercase() {
            b'M' => {
                let mut point = [parser.number(), parser.number()];
                if relative {
                    point = add(point, current);
                }
                current = point;
                start = point;
                path = path.move_to(point);
                previous = Some(if relative { b'l' } else { b'L' });
            }
            b'L' => {
                let mut point = [parser.number(), parser.number()];
                if relative {
                    point = add(point, current);
                }
                current = point;
                path = path.line_to(point);
            }
            b'H' => {
                let x = parser.number();
                current[0] = if relative { current[0] + x } else { x };
                path = path.line_to(current);
            }
            b'V' => {
                let y = parser.number();
                current[1] = if relative { current[1] + y } else { y };
                path = path.line_to(current);
            }
            b'C' => {
                let mut c1 = [parser.number(), parser.number()];
                let mut c2 = [parser.number(), parser.number()];
                let mut end = [parser.number(), parser.number()];
                if relative {
                    c1 = add(c1, current);
                    c2 = add(c2, current);
                    end = add(end, current);
                }
                path = path.cubic_to(c1, c2, end);
                current = end;
                last_cubic = c2;
            }
            b'S' => {
                let mut c2 = [parser.number(), parser.number()];
                let mut end = [parser.number(), parser.number()];
                if relative {
                    c2 = add(c2, current);
                    end = add(end, current);
                }
                let c1 = if was_cubic {
                    reflect(last_cubic, current)
                } else {
                    current
                };
                path = path.cubic_to(c1, c2, end);
                current = end;
                last_cubic = c2;
            }
            b'Q' => {
                let mut control = [parser.number(), parser.number()];
                let mut end = [parser.number(), parser.number()];
                if relative {
                    control = add(control, current);
                    end = add(end, current);
                }
                path = path.cubic_to(
                    quad_control(current, control),
                    quad_control(end, control),
                    end,
                );
                current = end;
                last_quad = control;
            }
            b'T' => {
                let mut end = [parser.number(), parser.number()];
                if relative {
                    end = add(end, current);
                }
                let control = if was_quad {
                    reflect(last_quad, current)
                } else {
                    current
                };
                path = path.cubic_to(
                    quad_control(current, control),
                    quad_control(end, control),
                    end,
                );
                current = end;
                last_quad = control;
            }
            b'A' => {
                let rx = parser.number();
                let ry = parser.number();
                let rotation = parser.number();
                let large = parser.flag();
                let sweep = parser.flag();
                let mut end = [parser.number(), parser.number()];
                if relative {
                    end = add(end, current);
                }
                let cubics = arc_cubics(current, rx, ry, rotation, large, sweep, end);
                if cubics.is_empty() {
                    // A zero radius or a zero-length arc is a straight line, as SVG specifies.
                    if end != current {
                        path = path.line_to(end);
                    }
                } else {
                    for (c1, c2, point) in cubics {
                        path = path.cubic_to(c1, c2, point);
                    }
                }
                current = end;
            }
            b'Z' => {
                path = path.close();
                current = start;
                previous = None;
            }
            other => panic!("unsupported SVG path command {:?}", other as char),
        }
        ran = command;
    }
    path
}

fn add(a: [f32; 2], b: [f32; 2]) -> [f32; 2] {
    [a[0] + b[0], a[1] + b[1]]
}

/// A dropped cubic or quadratic control point, mirrored through `current`.
fn reflect(control: [f32; 2], current: [f32; 2]) -> [f32; 2] {
    [2.0 * current[0] - control[0], 2.0 * current[1] - control[1]]
}

/// The cubic control point that turns a quadratic control point into the equivalent cubic.
fn quad_control(end: [f32; 2], control: [f32; 2]) -> [f32; 2] {
    [
        end[0] + 2.0 / 3.0 * (control[0] - end[0]),
        end[1] + 2.0 / 3.0 * (control[1] - end[1]),
    ]
}

/// Converts an SVG elliptical arc to cubic Bézier segments. Returns an empty list for a
/// zero-radius or zero-length arc; the caller then draws a straight line if needed.
fn arc_cubics(
    from: [f32; 2],
    mut rx: f32,
    mut ry: f32,
    rotation_degrees: f32,
    large: bool,
    sweep: bool,
    to: [f32; 2],
) -> Vec<([f32; 2], [f32; 2], [f32; 2])> {
    if from == to {
        return Vec::new();
    }
    rx = rx.abs();
    ry = ry.abs();
    if rx == 0.0 || ry == 0.0 {
        return Vec::new();
    }

    let phi = rotation_degrees.to_radians();
    let (sin_phi, cos_phi) = phi.sin_cos();
    let dx = (from[0] - to[0]) / 2.0;
    let dy = (from[1] - to[1]) / 2.0;
    let x1 = cos_phi * dx + sin_phi * dy;
    let y1 = -sin_phi * dx + cos_phi * dy;

    // Scale up radii too small to span the chord (SVG implementation notes F.6.6).
    let lambda = x1 * x1 / (rx * rx) + y1 * y1 / (ry * ry);
    if lambda > 1.0 {
        let scale = lambda.sqrt();
        rx *= scale;
        ry *= scale;
    }

    let rx2 = rx * rx;
    let ry2 = ry * ry;
    let numerator = (rx2 * ry2 - rx2 * y1 * y1 - ry2 * x1 * x1).max(0.0);
    let denominator = rx2 * y1 * y1 + ry2 * x1 * x1;
    let sign = if large == sweep { -1.0 } else { 1.0 };
    let coefficient = if denominator == 0.0 {
        0.0
    } else {
        sign * (numerator / denominator).sqrt()
    };
    let cxp = coefficient * rx * y1 / ry;
    let cyp = -coefficient * ry * x1 / rx;
    let cx = cos_phi * cxp - sin_phi * cyp + (from[0] + to[0]) / 2.0;
    let cy = sin_phi * cxp + cos_phi * cyp + (from[1] + to[1]) / 2.0;

    let ux = (x1 - cxp) / rx;
    let uy = (y1 - cyp) / ry;
    let vx = (-x1 - cxp) / rx;
    let vy = (-y1 - cyp) / ry;
    let start_angle = uy.atan2(ux);
    let mut sweep_angle = (ux * vy - uy * vx).atan2(ux * vx + uy * vy);
    if !sweep && sweep_angle > 0.0 {
        sweep_angle -= std::f32::consts::TAU;
    } else if sweep && sweep_angle < 0.0 {
        sweep_angle += std::f32::consts::TAU;
    }

    let count = (sweep_angle.abs() / std::f32::consts::FRAC_PI_2)
        .ceil()
        .max(1.0) as usize;
    let step = sweep_angle / count as f32;
    let ellipse = Ellipse {
        center: [cx, cy],
        radii: [rx, ry],
        sin_phi,
        cos_phi,
    };
    let mut angle = start_angle;
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        let next = angle + step;
        let (sin0, cos0) = angle.sin_cos();
        let (sin1, cos1) = next.sin_cos();
        let p0 = ellipse.point(cos0, sin0);
        let p1 = ellipse.point(cos1, sin1);
        let d0 = ellipse.derivative(cos0, sin0);
        let d1 = ellipse.derivative(cos1, sin1);
        let alpha = 4.0 / 3.0 * (step / 4.0).tan();
        out.push((
            [p0[0] + alpha * d0[0], p0[1] + alpha * d0[1]],
            [p1[0] - alpha * d1[0], p1[1] - alpha * d1[1]],
            p1,
        ));
        angle = next;
    }
    out
}

struct Ellipse {
    center: [f32; 2],
    radii: [f32; 2],
    sin_phi: f32,
    cos_phi: f32,
}

impl Ellipse {
    fn point(&self, cos_t: f32, sin_t: f32) -> [f32; 2] {
        let [cx, cy] = self.center;
        let [rx, ry] = self.radii;
        [
            cx + self.cos_phi * rx * cos_t - self.sin_phi * ry * sin_t,
            cy + self.sin_phi * rx * cos_t + self.cos_phi * ry * sin_t,
        ]
    }

    fn derivative(&self, cos_t: f32, sin_t: f32) -> [f32; 2] {
        let [rx, ry] = self.radii;
        [
            -self.cos_phi * rx * sin_t - self.sin_phi * ry * cos_t,
            -self.sin_phi * rx * sin_t + self.cos_phi * ry * cos_t,
        ]
    }
}

struct Parser<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl Parser<'_> {
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.offset).copied()
    }

    fn skip_separators(&mut self) {
        while let Some(byte) = self.peek() {
            if matches!(byte, b' ' | b'\t' | b'\r' | b'\n' | b',') {
                self.offset += 1;
            } else {
                break;
            }
        }
    }

    /// Reads one SVG number: optional sign, digits, optional fraction and exponent. Adjacent
    /// numbers need no separator (`M.5.5`, `m5-3`), so the scan stops at the second `.` or a
    /// sign that starts the next number.
    fn number(&mut self) -> f32 {
        self.skip_separators();
        let start = self.offset;
        if matches!(self.peek(), Some(b'+' | b'-')) {
            self.offset += 1;
        }
        let mut digits = false;
        while self.peek().is_some_and(|byte| byte.is_ascii_digit()) {
            self.offset += 1;
            digits = true;
        }
        if self.peek() == Some(b'.') {
            self.offset += 1;
            while self.peek().is_some_and(|byte| byte.is_ascii_digit()) {
                self.offset += 1;
                digits = true;
            }
        }
        if digits && matches!(self.peek(), Some(b'e' | b'E')) {
            let before_exponent = self.offset;
            self.offset += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.offset += 1;
            }
            let mut exponent_digits = false;
            while self.peek().is_some_and(|byte| byte.is_ascii_digit()) {
                self.offset += 1;
                exponent_digits = true;
            }
            if !exponent_digits {
                self.offset = before_exponent;
            }
        }
        let text = std::str::from_utf8(&self.bytes[start..self.offset])
            .expect("path data is UTF-8")
            .trim();
        text.parse()
            .unwrap_or_else(|_| panic!("malformed number {text:?} in icon path"))
    }

    /// Reads an arc flag. Flags can be packed without separators (`0 01 20` is large=0,
    /// sweep=1, x=20), so read a single `0`/`1` byte rather than a number.
    fn flag(&mut self) -> bool {
        self.skip_separators();
        match self.peek() {
            Some(b'0') => {
                self.offset += 1;
                false
            }
            Some(b'1') => {
                self.offset += 1;
                true
            }
            _ => self.number() != 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXPECTED: &[&str] = &[
        "arrow-right",
        "book-open",
        "circle",
        "circle-question-mark",
        "diamond",
        "ellipsis-vertical",
        "eraser",
        "hand",
        "lock",
        "map",
        "minus",
        "mouse-pointer-2",
        "pencil",
        "redo-2",
        "square",
        "sticky-note",
        "type",
        "undo-2",
        "x",
    ];

    #[test]
    fn every_expected_icon_is_vendored() {
        for name in EXPECTED {
            assert!(
                ICONS.iter().any(|def| def.name == *name),
                "{name} has no vendored path data"
            );
        }
    }

    #[test]
    fn every_icon_parses_to_a_non_empty_shape() {
        for name in EXPECTED {
            let shape = shape(name, 16.0, Color::WHITE);
            assert!(!shape.is_empty(), "{name} produced no strokes");
        }
    }

    #[test]
    fn icon_coordinates_are_finite() {
        for def in ICONS {
            for d in def.paths {
                let debug = format!("{:?}", parse_path(d));
                assert!(!debug.contains("NaN"), "{} produced NaN", def.name);
                assert!(
                    !debug.contains("inf"),
                    "{} produced an infinite coordinate",
                    def.name
                );
            }
        }
    }

    #[test]
    fn path_commands_convert_to_cubics() {
        // Arc-only circle, a quadratic with smooth continuation, a cubic with smooth
        // continuation, and a relative L/H/V/Z path.
        for d in [
            "M2 12a10 10 0 1 0 20 0a10 10 0 1 0-20 0",
            "M0 0q5 10 10 0t10 0",
            "M0 0c1 2 3 4 5 6s7 8 9 10",
            "M1 1h8v8H1z",
        ] {
            let path = parse_path(d);
            let shape = Shape::new().stroke_path(&path, 1.0, LineCap::Round, Color::WHITE);
            assert!(!shape.is_empty(), "{d} produced no strokes");
        }
    }

    #[test]
    fn arcs_scale_radii_too_small_for_the_chord() {
        // rx/ry cannot span the endpoints; the conversion must scale them up, not fold.
        let path = parse_path("M0 0A1 1 0 0 0 10 0");
        let debug = format!("{path:?}");
        assert!(!debug.contains("NaN"), "corrected arc produced NaN");
        assert!(
            !Shape::new()
                .stroke_path(&path, 1.0, LineCap::Round, Color::WHITE)
                .is_empty(),
            "corrected arc produced no strokes"
        );
    }
}
