use rune::tok::{self, motion};
use rune::{Color, El, col, el, text};

use crate::icons;

#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub bg: Color,
    pub card: Color,
    pub border: Color,
    pub text: Color,
    pub muted: Color,
    /// Keybind badge ink: ≥4.5:1 on the card, the board background and `selected_bg` in both
    /// themes, so the 10pt badges clear WCAG AA (the plain `muted` does not: 3.3:1 on the light
    /// selected fill).
    pub badge_ink: Color,
    pub accent: Color,
    /// Ink on a filled accent surface (the spec's on-primary); the chrome has no filled primary
    /// button yet, so it is kept for when one lands.
    #[allow(dead_code)]
    pub accent_ink: Color,
    pub danger: Color,
    pub selection: Color,
    pub hover: Color,
    pub press: Color,
    pub selected_bg: Color,
    pub selected_ink: Color,
    /// Ring around the selected colour swatch: the spec's `#4a47b1`, since `selected_ink` reads
    /// as the swatch itself on dark inks.
    pub swatch_ring: Color,
    pub shadow: Color,
    pub scrim: Color,
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
            bg: color([255, 255, 255, 255]),
            card: color([255, 255, 255, 255]),
            border: color([241, 240, 255, 255]),
            text: color([27, 27, 31, 255]),
            muted: color([122, 122, 122, 255]),
            badge_ink: color([95, 95, 104, 255]),
            accent: color([105, 101, 219, 255]),
            accent_ink: Color::WHITE,
            danger: color([219, 105, 101, 255]),
            selection: color([105, 101, 219, 255]),
            hover: color([241, 240, 255, 255]),
            press: color([227, 226, 254, 255]),
            selected_bg: color([224, 223, 255, 255]),
            selected_ink: color([3, 0, 100, 255]),
            swatch_ring: color([74, 71, 177, 255]),
            shadow: color([0, 0, 0, 26]),
            scrim: color([12, 12, 14, 89]),
            statement_ink: color([45, 40, 28, 255]),
        }
    } else {
        Palette {
            bg: color([18, 18, 18, 255]),
            card: color([35, 35, 41, 255]),
            border: color([46, 45, 57, 255]),
            text: color([227, 227, 232, 255]),
            muted: color([160, 160, 170, 255]),
            badge_ink: color([184, 184, 190, 255]),
            accent: color([168, 165, 255, 255]),
            accent_ink: color([35, 35, 41, 255]),
            danger: color([255, 135, 135, 255]),
            selection: color([168, 165, 255, 255]),
            hover: color([46, 45, 57, 255]),
            press: color([64, 62, 106, 255]),
            selected_bg: color([64, 62, 106, 255]),
            selected_ink: color([224, 223, 255, 255]),
            swatch_ring: color([168, 165, 255, 255]),
            shadow: color([0, 0, 0, 90]),
            scrim: color([12, 12, 14, 166]),
            statement_ink: color([45, 40, 28, 255]),
        }
    }
}

pub fn body() -> rune::TextStyle {
    tok::type_::BODY
}

pub fn caption() -> rune::TextStyle {
    rune::TextStyle::new(12.0, 500.0)
}

pub fn heading(size: f32) -> rune::TextStyle {
    rune::TextStyle::new(size, 650.0)
}

/// Column variant of [`island`], for panels whose content stacks vertically and may scroll.
pub fn island_col(pal: &Palette) -> El {
    col()
        .hit()
        .padding(4.0)
        .radius(8.0)
        .fill(pal.card)
        .border(1.0, pal.border)
        .shadow(7.0, 14.0, pal.shadow)
}

pub fn island(pal: &Palette) -> El {
    el().hit()
        .padding(4.0)
        .radius(8.0)
        .fill(pal.card)
        .border(1.0, pal.border)
        .shadow(7.0, 14.0, pal.shadow)
}

pub fn button(key: impl Into<String>, label: &str, pal: &Palette) -> El {
    el().key(key)
        .padding(6.0)
        .radius(8.0)
        .fill(pal.card)
        .hover(pal.hover)
        .press(pal.press)
        .child(
            text(label)
                .font(tok::TextStyle::new(12.0, 600.0))
                .color(pal.text),
        )
}

/// Dialog and card action button: [`button`] with the spec's 1pt border. The border is what makes
/// the button visible on the card fill, so every caller that needs an affordance uses this.
pub fn outlined_button(key: impl Into<String>, label: &str, pal: &Palette) -> El {
    button(key, label, pal).border(1.0, pal.border)
}

pub fn flat_button(key: impl Into<String>, label: &str, pal: &Palette) -> El {
    el().key(key)
        .padding(8.0)
        .radius(8.0)
        .items_center()
        .hover(pal.hover)
        .press(pal.press)
        .child(
            text(label)
                .font(tok::TextStyle::new(12.0, 600.0))
                .color(pal.text),
        )
}

/// [`flat_button`] without its text child: a 28×28 target for a lone icon. Callers keep the
/// accessible name with `.label(..)` and add the icon canvas.
pub fn flat_icon_button(key: impl Into<String>, pal: &Palette) -> El {
    el().key(key)
        .size(28.0, 28.0)
        .radius(8.0)
        .items_center()
        .justify_center()
        .hover(pal.hover)
        .press(pal.press)
}

/// A dropdown menu row: 32 tall, radius 6, 14/400.
pub fn menu_row(key: impl Into<String>, label: &str, pal: &Palette) -> El {
    el().key(key)
        .h(32.0)
        .px(8.0)
        .radius(6.0)
        .items_center()
        .hover(pal.hover)
        .press(pal.press)
        .child(
            text(label)
                .font(tok::TextStyle::new(14.0, 400.0))
                .color(pal.text),
        )
}

/// [`menu_row`] with a leading Lucide icon, for a row whose action has a clear glyph.
pub fn menu_row_icon(key: impl Into<String>, label: &str, icon: &str, pal: &Palette) -> El {
    el().key(key)
        .h(32.0)
        .px(8.0)
        .gap(8.0)
        .radius(6.0)
        .items_center()
        .hover(pal.hover)
        .press(pal.press)
        .child(icons::icon(icon, 16.0, pal.text))
        .child(
            text(label)
                .font(tok::TextStyle::new(14.0, 400.0))
                .color(pal.text),
        )
}

/// A 36×36 toolbar tool box. Add the icon canvas as its child. `checked` carries the selected
/// tool to assistive technology; Rune reports it as the button's pressed state.
pub fn tool_button(key: impl Into<String>, selected: bool, pal: &Palette) -> El {
    el().key(key)
        .size(36.0, 36.0)
        .radius(8.0)
        .items_center()
        .justify_center()
        .checked(selected)
        .fill(if selected {
            pal.selected_bg
        } else {
            Color::TRANSPARENT
        })
        .hover(if selected { pal.selected_bg } else { pal.hover })
        .press(pal.press)
}

pub fn active_button(key: impl Into<String>, label: &str, active: bool, pal: &Palette) -> El {
    let fill = if active { pal.selected_bg } else { pal.card };
    let ink = if active { pal.selected_ink } else { pal.text };
    el().key(key)
        .padding(8.0)
        .radius(8.0)
        .fill(fill)
        // The fill shows which option is active; `checked` says the same to assistive
        // technology (Rune's pressed/toggled state).
        .checked(active)
        .hover(pal.hover)
        .press(pal.press)
        .border(1.0, if active { pal.selected_bg } else { pal.border })
        .motion(motion::SNAPPY)
        .child(
            text(label)
                .font(tok::TextStyle::new(12.0, 600.0))
                .color(ink),
        )
}

pub fn swatch(key: impl Into<String>, rgba: [u8; 4], selected: bool, pal: &Palette) -> El {
    el().key(key)
        .size(22.0, 22.0)
        .radius(4.0)
        .fill(color(rgba))
        // The raised ring marks the active ink visually; `checked` marks it for readers.
        .checked(selected)
        .border(
            if selected { 2.0 } else { 1.0 },
            if selected {
                pal.swatch_ring
            } else {
                pal.border
            },
        )
        .hover(pal.hover)
        .press(pal.press)
}

/// A 10pt keycap hint pinned to a 36×36 tool button's bottom-right corner, clear of the 16pt
/// icon that sits centred above it. Muted and small, so it reads as a hint, not a label, but
/// drawn in the dedicated badge ink so it clears WCAG AA on the card, the board and the
/// selected fill.
pub fn key_badge(key: &str, pal: &Palette) -> El {
    text(key)
        .font(rune::TextStyle::new(10.0, 600.0))
        .color(pal.badge_ink)
        // Text is drawn, not focused; the button's label and help already announce the tool.
        .role("none")
        .pin(None, None, Some(3.0), Some(1.0))
}

pub fn divider_h(pal: &Palette) -> El {
    el().w_full().h(1.0).fill(pal.border).my(6.0)
}

pub fn divider_v(pal: &Palette) -> El {
    el().w(1.0).h(24.0).fill(pal.border).mx(4.0)
}

pub fn section(title_text: &str, pal: &Palette) -> El {
    text(title_text).font(tok::type_::SECTION).color(pal.muted)
}

pub fn section_style() -> rune::TextStyle {
    tok::type_::SECTION
}
