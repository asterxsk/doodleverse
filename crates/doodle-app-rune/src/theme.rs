use rune::tok::{self, motion};
use rune::{Color, El, col, el, text};

use crate::icons;

#[derive(Debug, Clone, Copy)]
pub struct Palette {
    /// Which token set this is; a few controls (slider thumb border) need the dark-specific
    /// counterpart token rather than a computed tint.
    pub dark: bool,
    pub bg: Color,
    pub card: Color,
    pub border: Color,
    /// coss `--input`: the filled control surface (slider track, switch off, outlined border).
    pub input: Color,
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
            dark: false,
            bg: color([255, 255, 255, 255]),
            card: color([255, 255, 255, 255]),
            border: color([0, 0, 0, 20]),
            input: color([0, 0, 0, 26]),
            text: color([38, 38, 38, 255]),
            muted: color([96, 96, 96, 255]),
            badge_ink: color([95, 95, 95, 255]),
            accent: color([38, 38, 38, 255]),
            accent_ink: color([250, 250, 250, 255]),
            danger: color([251, 44, 54, 255]),
            selection: color([163, 163, 163, 255]),
            hover: color([0, 0, 0, 10]),
            press: color([0, 0, 0, 20]),
            selected_bg: color([38, 38, 38, 255]),
            shadow: color([0, 0, 0, 26]),
            scrim: color([0, 0, 0, 82]),
            statement_ink: color([45, 40, 28, 255]),
        }
    } else {
        Palette {
            dark: true,
            bg: color([22, 22, 22, 255]),
            card: color([27, 27, 27, 255]),
            border: color([255, 255, 255, 15]),
            input: color([255, 255, 255, 20]),
            text: color([245, 245, 245, 255]),
            muted: color([128, 128, 128, 255]),
            badge_ink: color([176, 176, 176, 255]),
            accent: color([245, 245, 245, 255]),
            accent_ink: color([38, 38, 38, 255]),
            danger: color([251, 44, 54, 255]),
            selection: color([115, 115, 115, 255]),
            hover: color([255, 255, 255, 10]),
            press: color([255, 255, 255, 20]),
            selected_bg: color([245, 245, 245, 255]),
            shadow: color([0, 0, 0, 90]),
            scrim: color([0, 0, 0, 82]),
            statement_ink: color([45, 40, 28, 255]),
        }
    }
}

/// coss ui's radius scale on Tailwind's defaults: `rounded-lg` 8 for buttons, inputs, popovers
/// and menu surfaces, `rounded-md` 6 for menu items, `rounded-xl` 12 for the toolbar and
/// `rounded-2xl` 16 for dialogs.
pub const RADIUS_SURFACE: f32 = 8.0;
pub const RADIUS_CONTROL: f32 = 8.0;
pub const RADIUS_TOOLBAR: f32 = 12.0;
pub const RADIUS_DIALOG: f32 = 16.0;
pub const RADIUS_ITEM: f32 = 6.0;

pub fn body() -> rune::TextStyle {
    tok::type_::BODY
}

pub fn caption() -> rune::TextStyle {
    rune::TextStyle::new(12.0, 500.0)
}

pub fn heading(size: f32) -> rune::TextStyle {
    rune::TextStyle::new(size, 650.0)
}

/// coss `bg-input/64`: the pressed state of a toggle or toolbar button, a translucent input
/// wash over the card rather than a filled primary surface.
pub fn pressed_fill(pal: &Palette) -> Color {
    Color {
        a: pal.input.a * 0.64,
        ..pal.input
    }
}

/// Column variant of [`island`], for panels whose content stacks vertically and may scroll.
pub fn island_col(pal: &Palette) -> El {
    col()
        .hit()
        .padding(4.0)
        .radius(RADIUS_SURFACE)
        .fill(pal.card)
        .border(1.0, pal.border)
        .shadow(7.0, 14.0, pal.shadow)
}

pub fn island(pal: &Palette) -> El {
    el().hit()
        .padding(4.0)
        .radius(RADIUS_SURFACE)
        .fill(pal.card)
        .border(1.0, pal.border)
        .shadow(7.0, 14.0, pal.shadow)
}

/// A floating panel surface: card fill, 1pt border, surface radius, 12pt padding and section
/// spacing. The left tool-settings panel and dialogs share it.
pub fn panel(pal: &Palette) -> El {
    col()
        .hit()
        .w_full()
        .padding(12.0)
        .gap(12.0)
        .radius(RADIUS_SURFACE)
        .fill(pal.card)
        .border(1.0, pal.border)
        .shadow(10.0, 24.0, pal.shadow)
}

/// coss button: `h-8` (32), `rounded-lg`, 1px input border, 14/500 label and `shadow-xs`.
pub fn button(key: impl Into<String>, label: &str, pal: &Palette) -> El {
    el().key(key)
        .h(32.0)
        .px(11.0)
        .radius(RADIUS_SURFACE)
        .items_center()
        .fill(pal.card)
        .border(1.0, pal.input)
        .shadow(1.0, 3.0, pal.shadow)
        .hover(pal.hover)
        .press(pal.press)
        .child(
            text(label)
                .font(tok::TextStyle::new(14.0, 500.0))
                .color(pal.text),
        )
}

/// Dialog and card action button: [`button`] with the spec's 1pt border. The border is what makes
/// the button visible on the card fill, so every caller that needs an affordance uses this.
pub fn outlined_button(key: impl Into<String>, label: &str, pal: &Palette) -> El {
    button(key, label, pal).border(1.0, pal.border)
}

/// coss ghost button: transparent border, accent hover and pressed, 14/500.
pub fn flat_button(key: impl Into<String>, label: &str, pal: &Palette) -> El {
    el().key(key)
        .h(32.0)
        .px(10.0)
        .radius(RADIUS_SURFACE)
        .items_center()
        .hover(pal.hover)
        .press(pal.press)
        .child(
            text(label)
                .font(tok::TextStyle::new(14.0, 500.0))
                .color(pal.text),
        )
}

/// [`flat_button`] without its text child: a 28×28 target for a lone icon. Callers keep the
/// accessible name with `.label(..)` and add the icon canvas.
pub fn flat_icon_button(key: impl Into<String>, pal: &Palette) -> El {
    el().key(key)
        .size(28.0, 28.0)
        .radius(RADIUS_CONTROL)
        .items_center()
        .justify_center()
        .hover(pal.hover)
        .press(pal.press)
}

/// A dropdown menu row: 32 tall, control radius, 14/400.
pub fn menu_row(key: impl Into<String>, label: &str, pal: &Palette) -> El {
    el().key(key)
        .h(28.0)
        .px(8.0)
        .radius(RADIUS_ITEM)
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
        .h(28.0)
        .px(8.0)
        .gap(8.0)
        .radius(RADIUS_ITEM)
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
        .size(32.0, 32.0)
        .radius(RADIUS_CONTROL)
        .items_center()
        .justify_center()
        .checked(selected)
        .fill(if selected {
            pressed_fill(pal)
        } else {
            Color::TRANSPARENT
        })
        .hover(pal.hover)
        .press(pal.press)
}

pub fn active_button(key: impl Into<String>, label: &str, active: bool, pal: &Palette) -> El {
    // coss Toggle: transparent border, accent hover, `bg-input/64` while pressed, label stays
    // in the primary ink. `checked` carries the same pressed state to assistive technology.
    el().key(key)
        .h(28.0)
        .px(10.0)
        .radius(RADIUS_CONTROL)
        .items_center()
        .fill(if active {
            pressed_fill(pal)
        } else {
            Color::TRANSPARENT
        })
        .checked(active)
        .hover(pal.hover)
        .press(pal.press)
        .motion(motion::SNAPPY)
        .child(
            text(label)
                .font(tok::TextStyle::new(14.0, 500.0))
                .color(pal.text),
        )
}

/// A compact choice inside a panel row: three fill styles or three width presets fit the
/// 184pt inner width without wrapping.
pub fn choice_button(key: impl Into<String>, label: &str, active: bool, pal: &Palette) -> El {
    // coss Toggle (sm): 28 tall, 14/500, pressed = `bg-input/64`.
    el().key(key)
        .h(28.0)
        .px(8.0)
        .radius(RADIUS_CONTROL)
        .fill(if active {
            pressed_fill(pal)
        } else {
            Color::TRANSPARENT
        })
        .checked(active)
        .items_center()
        .justify_center()
        .hover(pal.hover)
        .press(pal.press)
        .motion(motion::SNAPPY)
        .child(
            text(label)
                .font(rune::TextStyle::new(14.0, 500.0))
                .color(pal.text),
        )
}

/// coss switch: an 18×30 rounded track (`--thumb-size` 16 plus the 1pt padding), primary when
/// on and `input` when off, with a background-coloured thumb that springs between the ends.
pub fn switch(key: impl Into<String>, on: bool, pal: &Palette) -> El {
    let left = if on { 13.0 } else { 1.0 };
    el().key(key)
        .size(30.0, 18.0)
        .radius(9.0)
        .fill(if on { pal.selected_bg } else { pal.input })
        .checked(on)
        .hit()
        .motion(motion::SNAPPY)
        .child(
            el().absolute(left, 1.0)
                .size(16.0, 16.0)
                .radius(8.0)
                .fill(if pal.dark {
                    pal.bg
                } else {
                    color([255, 255, 255, 255])
                })
                .shadow(2.0, 4.0, pal.shadow)
                .motion(motion::SNAPPY)
                .hit(),
        )
}

/// A labelled switch row: a 14/500 label and an optional muted description.
pub fn switch_row(key: impl Into<String>, label: &str, on: bool, pal: &Palette) -> El {
    let key: String = key.into();
    el().key(key.clone())
        .w_full()
        .items_center()
        .gap(8.0)
        .child(
            text(label)
                .font(tok::TextStyle::new(14.0, 500.0))
                .color(pal.text)
                .grow(1.0),
        )
        .child(switch(format!("{key}-switch"), on, pal))
}

pub fn swatch(key: impl Into<String>, rgba: [u8; 4], selected: bool, pal: &Palette) -> El {
    // The selected ring must read on the swatch itself, not on the card: a dark ink gets a
    // light ring and a pale ink gets a dark one. The coss neutral ring alone disappears on
    // both extremes.
    let luma =
        0.2126 * f32::from(rgba[0]) + 0.7152 * f32::from(rgba[1]) + 0.0722 * f32::from(rgba[2]);
    let ring = if luma < 96.0 {
        color([250, 250, 250, 255])
    } else {
        color([38, 38, 38, 255])
    };
    el().key(key)
        .size(22.0, 22.0)
        .radius(RADIUS_CONTROL)
        .fill(color(rgba))
        // The raised ring marks the active ink visually; `checked` marks it for readers.
        .checked(selected)
        .border(
            if selected { 2.0 } else { 1.0 },
            if selected { ring } else { pal.border },
        )
        .hover(pal.hover)
        .press(pal.press)
}

/// A 10pt keycap hint pinned to a 36×36 tool button's bottom-right corner, clear of the 16pt
/// icon that sits centred above it. The ink follows the button's state: a selected tool is a
/// filled primary surface, so its badge uses the primary-foreground ink and clears WCAG AA
/// there; an unselected one uses the dedicated badge ink on the card.
pub fn key_badge(key: &str, pal: &Palette) -> El {
    // The pressed toolbar fill is a translucent input wash, so one badge ink works on both
    // states.
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
    text(title_text).font(section_style()).color(pal.muted)
}

/// coss section labels: 12pt, medium weight, sentence case, muted.
pub fn section_style() -> rune::TextStyle {
    rune::TextStyle::new(12.0, 500.0)
}

/// coss Field label: 14pt medium in the primary ink, the weight every control caption uses.
pub fn field_label_style() -> rune::TextStyle {
    rune::TextStyle::new(14.0, 500.0)
}
