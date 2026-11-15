use rune::tok::{self, motion};
use rune::{Color, El, col, el, text};

#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub bg: Color,
    pub card: Color,
    pub border: Color,
    pub text: Color,
    pub muted: Color,
    pub accent: Color,
    pub accent_ink: Color,
    pub danger: Color,
    pub selection: Color,
    pub hover: Color,
    pub press: Color,
    pub shadow: Color,
    pub statement_ink: Color,
}

pub fn color(rgba: [u8; 4]) -> Color {
    Color {
        r: linear(rgba[0]),
        g: linear(rgba[1]),
        b: linear(rgba[2]),
        a: f32::from(rgba[3]) / 255.0,
    }
}

fn linear(value: u8) -> f32 {
    let c = f32::from(value) / 255.0;
    if c <= 0.040_45 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

pub fn palette(light: bool) -> Palette {
    if light {
        Palette {
            bg: color([250, 248, 243, 255]),
            card: color([255, 253, 248, 255]),
            border: color([222, 215, 202, 255]),
            text: color([34, 32, 37, 255]),
            muted: color([112, 107, 116, 255]),
            accent: color([222, 108, 85, 255]),
            accent_ink: Color::WHITE,
            danger: color([180, 56, 48, 255]),
            selection: color([55, 145, 141, 255]),
            hover: color([244, 239, 230, 255]),
            press: color([235, 228, 217, 255]),
            shadow: color([40, 35, 30, 28]),
            statement_ink: color([45, 40, 28, 255]),
        }
    } else {
        Palette {
            bg: color([24, 23, 28, 255]),
            card: color([36, 35, 42, 255]),
            border: color([66, 63, 74, 255]),
            text: color([238, 234, 227, 255]),
            muted: color([164, 158, 172, 255]),
            accent: color([255, 155, 130, 255]),
            accent_ink: color([34, 20, 16, 255]),
            danger: color([255, 129, 118, 255]),
            selection: color([94, 201, 192, 255]),
            hover: color([48, 46, 55, 255]),
            press: color([58, 55, 65, 255]),
            shadow: color([0, 0, 0, 70]),
            statement_ink: color([45, 40, 28, 255]),
        }
    }
}

pub fn body() -> rune::TextStyle {
    tok::type_::BODY
}

pub fn caption() -> rune::TextStyle {
    tok::TextStyle::new(11.0, 500.0)
}

pub fn heading(size: f32) -> rune::TextStyle {
    rune::TextStyle::new(size, 650.0)
}

/// Column variant of [`island`], for panels whose content stacks vertically and may scroll.
pub fn island_col(pal: &Palette) -> El {
    col()
        .hit()
        .padding(7.0)
        .radius(14.0)
        .fill(pal.card)
        .border(1.0, pal.border)
        .shadow(6.0, 22.0, pal.shadow)
}

pub fn island(pal: &Palette) -> El {
    el().hit()
        .padding(7.0)
        .radius(14.0)
        .fill(pal.card)
        .border(1.0, pal.border)
        .shadow(6.0, 22.0, pal.shadow)
}

pub fn button(key: impl Into<String>, label: &str, pal: &Palette) -> El {
    el().key(key)
        .padding(9.0)
        .radius(8.0)
        .fill(pal.card)
        .hover(pal.hover)
        .press(pal.press)
        .border(1.0, pal.border)
        .child(
            text(label)
                .font(tok::TextStyle::new(12.5, 550.0))
                .color(pal.text),
        )
}

pub fn flat_button(key: impl Into<String>, label: &str, pal: &Palette) -> El {
    el().key(key)
        .padding(8.0)
        .radius(7.0)
        .hover(pal.hover)
        .press(pal.press)
        .child(
            text(label)
                .font(tok::TextStyle::new(12.5, 550.0))
                .color(pal.text),
        )
}

pub fn active_button(key: impl Into<String>, label: &str, active: bool, pal: &Palette) -> El {
    let fill = if active { pal.accent } else { pal.card };
    let ink = if active { pal.accent_ink } else { pal.text };
    el().key(key)
        .padding(8.0)
        .radius(7.0)
        .fill(fill)
        .hover(if active { pal.accent } else { pal.hover })
        .press(pal.press)
        .border(1.0, if active { pal.accent } else { pal.border })
        .motion(motion::SNAPPY)
        .child(
            text(label)
                .font(tok::TextStyle::new(12.5, 600.0))
                .color(ink),
        )
}

pub fn swatch(key: impl Into<String>, rgba: [u8; 4], selected: bool, pal: &Palette) -> El {
    el().key(key)
        .size(22.0, 22.0)
        .radius(11.0)
        .fill(color(rgba))
        .border(
            if selected { 2.5 } else { 1.0 },
            if selected { pal.text } else { pal.border },
        )
        .hover(pal.hover)
        .press(pal.press)
}

pub fn divider_h(pal: &Palette) -> El {
    el().w_full().h(1.0).fill(pal.border)
}

pub fn divider_v(pal: &Palette) -> El {
    el().w(1.0).h(24.0).fill(pal.border)
}

pub fn section(title_text: &str, pal: &Palette) -> El {
    text(title_text).font(tok::type_::SECTION).color(pal.muted)
}

pub fn section_style() -> rune::TextStyle {
    tok::type_::SECTION
}
