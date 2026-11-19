use doodle_ai::{AiCard, AiProvider, CardState, MascotInput, MascotMood, resolve_mascot};
use doodle_types::Point;
use rune::tok::{self, type_};
use rune::{El, ScrollSpec, TextEdit, col, edit_access, el, row, text, text_input};

use crate::app::{CHROME_BOTTOM_INSET, CHROME_TOP_INSET, DoodleApp};
use crate::icons;
use crate::theme::{self, Palette};

pub struct CardUi {
    pub card: AiCard,
    pub question: TextEdit,
    pub busy_started: Option<f64>,
    pub mood: MascotMood,
    pub mood_started: f64,
    pub hover_entry: Option<f64>,
    pub pinned: bool,
    pub last_change: f64,
    pub last_drag: f32,
    /// Whether the card was generating at the previous poll. The answer viewport jumps to its
    /// top when this changes, so a finished answer starts at its first line.
    pub was_busy: bool,
}

impl CardUi {
    pub fn new(id: u64, world_pos: Point) -> Self {
        Self {
            card: AiCard::new(id, world_pos),
            question: TextEdit::new(""),
            busy_started: None,
            mood: MascotMood::Neutral,
            mood_started: 0.0,
            hover_entry: None,
            pinned: false,
            last_change: 0.0,
            last_drag: 0.0,
            was_busy: false,
        }
    }

    pub fn mark_changed(&mut self, now: f64) {
        self.last_change = now;
    }

    /// Copies the text field into the card so `Generate` always reads what the user sees.
    pub fn sync_question(&mut self) -> bool {
        if self.question.text == self.card.question {
            return false;
        }
        let text = self.question.text.clone();
        self.card.set_question(text);
        self.busy_started = None;
        true
    }

    pub fn update_mood(&mut self, now: f64, reduced_motion: bool) {
        let hover_elapsed = self
            .hover_entry
            .map(|entry| ((now - entry).max(0.0) * 1000.0) as u64);
        let busy_elapsed = self
            .busy_started
            .map(|entry| ((now - entry).max(0.0) * 1000.0) as u64);
        let mood = resolve_mascot(MascotInput {
            reduced_motion,
            talking: matches!(self.card.state, CardState::Streaming),
            hover_elapsed_ms: hover_elapsed,
            busy_elapsed_ms: busy_elapsed,
        });
        if mood != self.mood {
            self.mood = mood;
            self.mood_started = now;
        }
    }
}

pub struct PinPreview {
    pub card_id: u64,
    pub text: String,
}

const CARD_WIDTH: f32 = 336.0;
/// Space kept between a card and the board's edge when a card is laid out.
const CARD_MARGIN: f32 = 12.0;
const CARD_PADDING: f32 = 12.0;
const CARD_GAP: f32 = 8.0;
/// Width the question text wraps at: the card minus the body padding and the field's own box.
const QUESTION_INNER_WIDTH: f32 = CARD_WIDTH - CARD_PADDING * 2.0 - 16.0;
const QUESTION_MIN_HEIGHT: f32 = 44.0;
/// The question box's 8pt padding top and bottom. Its 1pt border is inset, like the card's, so it
/// adds no height; counting it here would overstate the card and push the pin placement's rect
/// off the card that is actually drawn.
const QUESTION_BOX_CHROME: f32 = 16.0;
const HEADER_HEIGHT: f32 = 44.0;
const ACTION_ROW_HEIGHT: f32 = 27.0;
/// The answer's caption, box padding and border, above its scrolling text.
const ANSWER_BOX_CHROME: f32 = 37.0;
const ANSWER_FOOTER_HEIGHT: f32 = 27.0;
const ANSWER_GAP: f32 = 6.0;
/// A row of alternative-answer buttons.
const ANSWER_OPTIONS_HEIGHT: f32 = 31.0;
const MIN_ANSWER_HEIGHT: f32 = 48.0;
const MAX_ANSWER_HEIGHT: f32 = 420.0;

/// The wrapped height of the question field, which grows past its 44pt floor when the question
/// wraps. Measuring it the way the answer is measured keeps the card's height budget honest.
fn question_field_height(question: &str) -> f32 {
    let (_, height) =
        rune::text::measure(question, &[], tok::type_::BODY, Some(QUESTION_INNER_WIDTH));
    height.max(QUESTION_MIN_HEIGHT)
}

/// Everything in a card outside the answer's scrolling viewport: padding, header, question
/// field at its current wrapped height, actions, the answer caption and the pin footer. Used to
/// clamp a card inside the visible board, so a long wrapped question can never push the footer
/// under the window's edge.
fn chrome_height(card: &CardUi) -> f32 {
    let mut chrome = CARD_PADDING * 2.0
        + HEADER_HEIGHT
        + CARD_GAP * 2.0
        + question_field_height(&card.question.text)
        + QUESTION_BOX_CHROME
        + ACTION_ROW_HEIGHT;
    if !card.card.candidates.is_empty() {
        chrome += CARD_GAP;
        let options = card.card.candidates.len() > 1;
        let section_children = if options { 3.0 } else { 2.0 };
        chrome += ANSWER_GAP * (section_children - 1.0) + ANSWER_BOX_CHROME + ANSWER_FOOTER_HEIGHT;
        if options {
            chrome += ANSWER_OPTIONS_HEIGHT;
        }
    }
    chrome
}

fn status_line(card: &AiCard, pinned: bool) -> String {
    match &card.state {
        CardState::Draft => "Ask a question to begin".into(),
        CardState::Submitting => "Starting…".into(),
        CardState::Streaming => "Generating…".into(),
        CardState::Alternatives if pinned => "Pinned to board".into(),
        CardState::Alternatives => "Answer ready".into(),
        CardState::Failed { message, partial } => {
            let note = if *partial { " (partial)" } else { "" };
            format!("Error{note}: {message}")
        }
        CardState::Cancelled => "Cancelled".into(),
    }
}

/// A card's rect in window points, after the clamp that keeps the whole card inside the board
/// area the chrome leaves free. Pin placement reads this instead of the card's raw world
/// position, so a pinned note is placed against what the user actually sees.
pub struct CardLayout {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    /// Cap the answer's scrolling viewport gets at this card's current y.
    pub answer_max_height: f32,
}

/// The height of the answer's scrolling viewport, given the cap for the card's current position.
/// The pin placement needs the card's full drawn height, so this is shared with `answer_section`.
fn answer_viewport_height(card: &CardUi, max_height: f32) -> f32 {
    let ai = &card.card;
    let selected = ai.selected.unwrap_or(ai.candidates.len().saturating_sub(1));
    let Some(candidate) = ai.candidates.get(selected) else {
        return 0.0;
    };
    let content = if candidate.text.is_empty() {
        "…".to_string()
    } else {
        candidate.text.clone()
    };
    let (_, content_height) = rune::text::measure(
        &content,
        &[],
        tok::type_::BODY,
        Some(CARD_WIDTH - 24.0 - 18.0),
    );
    (content_height + 1.0).clamp(MIN_ANSWER_HEIGHT, max_height)
}

/// Where a card is drawn in window points. Wide windows keep the drop offset; a narrow or
/// resized window slides the card back in instead of clipping it or tucking it under the
/// toolbar, status island or zoom island. The chrome includes the question field's real wrapped
/// height, so a long question cannot grow the card past the free area and hide the footer.
pub fn card_layout(s: &DoodleApp, card: &CardUi) -> CardLayout {
    let viewport = s.viewport();
    let viewport_width = viewport.width() as f32;
    let viewport_height = viewport.height() as f32;
    let screen = s.camera.world_to_screen(card.card.world_pos, viewport);
    let chrome = chrome_height(card);
    let x_min = s.board_left_inset();
    let x = (screen.x as f32).clamp(
        x_min,
        (viewport_width - CARD_WIDTH - CARD_MARGIN).max(x_min),
    );
    let y_min = CHROME_TOP_INSET;
    let y_max = (viewport_height - CHROME_BOTTOM_INSET - chrome - MIN_ANSWER_HEIGHT).max(y_min);
    let y = (screen.y as f32).clamp(y_min, y_max);
    // The answer grows with the card until this window-derived cap, then scrolls.
    let answer_max_height = (viewport_height - CHROME_BOTTOM_INSET - y - chrome)
        .clamp(MIN_ANSWER_HEIGHT, MAX_ANSWER_HEIGHT);
    let height = chrome
        + if card.card.candidates.is_empty() {
            0.0
        } else {
            answer_viewport_height(card, answer_max_height)
        };
    CardLayout {
        x,
        y,
        width: CARD_WIDTH,
        height,
        answer_max_height,
    }
}

/// One absolutely positioned element per local AI card, in window points.
pub fn card_views(s: &DoodleApp, pal: &Palette) -> Vec<El> {
    let provider_name = s.provider.name().to_string();
    let mut views = Vec::with_capacity(s.cards.len());
    for card in &s.cards {
        let layout = card_layout(s, card);
        let (x, y, answer_max_height) = (layout.x, layout.y, layout.answer_max_height);
        let id = card.card.local_id;

        let mut body = col()
            .hit()
            .w(CARD_WIDTH)
            .padding(12.0)
            .gap(8.0)
            .radius(10.0)
            .fill(pal.card)
            .border(1.0, pal.border)
            .shadow(10.0, 28.0, pal.shadow);

        // Header: dragging the avatar moves the card.
        let mood_elapsed = ((s.clock - card.mood_started).max(0.0) * 1000.0) as u64;
        let mut avatar = el()
            .size(44.0, 44.0)
            .radius(8.0)
            .fill(pal.hover)
            .on_drag(move |s: &mut DoodleApp, d, _| s.drag_card(id, d))
            .on_hover(move |s: &mut DoodleApp| {
                if let Some(card) = s.cards.iter_mut().find(|c| c.card.local_id == id) {
                    card.hover_entry = Some(s.clock);
                }
            });
        if let Some(mascot) = &s.mascot
            && let Some(frame) = mascot.frame(card.mood, mood_elapsed)
        {
            avatar = avatar.pixels(frame);
        }

        let failed = matches!(card.card.state, CardState::Failed { .. });
        let mut titles = col().grow(1.0).gap(1.0);
        titles = titles.child(text("Dibby").font(type_::TITLE).color(pal.text));
        titles = titles.child(
            text(status_line(&card.card, card.pinned))
                .font(theme::caption())
                .color(if failed { pal.danger } else { pal.muted }),
        );

        let header = row()
            .gap(10.0)
            .items_center()
            .w_full()
            .child(avatar)
            .child(titles)
            .child(
                theme::flat_icon_button(format!("card-close-{id}"), pal)
                    .label("Close Dibby")
                    .child(icons::icon("close", 16.0, pal.text))
                    .on_press(move |s: &mut DoodleApp| s.close_card(id)),
            );
        body = body.child(header);

        let field = text_input(&card.question, tok::type_::BODY)
            .label("Question for Dibby")
            .on_edit(edit_access(move |s: &mut DoodleApp| {
                let position = s.cards.iter().position(|c| c.card.local_id == id);
                match position {
                    Some(index) => &mut s.cards[index].question,
                    None => &mut s.scratch,
                }
            }))
            .placeholder("Ask Dibby for a doodle idea…", pal.muted)
            .color(pal.text)
            .caret(pal.text, pal.selection)
            .w_full()
            .min_h(44.0);
        body = body.child(
            el().w_full()
                .padding(8.0)
                .radius(8.0)
                .fill(pal.hover)
                .border(1.0, pal.border)
                .child(field),
        );

        let busy = card.card.is_busy();
        let mut actions = row().gap(8.0).items_center();
        if busy {
            actions = actions.child(
                theme::outlined_button(format!("card-cancel-{id}"), "Cancel", pal)
                    .label("Cancel")
                    .on_press(move |s: &mut DoodleApp| s.card_cancel(id)),
            );
        } else {
            let label = if card.card.candidates.is_empty() {
                "Generate"
            } else {
                "Generate another"
            };
            let enabled = !card.question.text.trim().is_empty();
            // A disabled element keeps its role and name when both are set, as the Undo/Redo
            // buttons do, so the disabled Generate reads as a dimmed button and not a group.
            let mut go =
                theme::outlined_button(format!("card-go-{id}"), label, pal).label(label.to_owned());
            if enabled {
                go = go
                    .focusable(true)
                    .on_press(move |s: &mut DoodleApp| s.card_submit(id));
            } else {
                // `disabled` stops focus but leaves no press target, so a press would reach the
                // board; `hit` keeps it on the button and consumes it.
                go = go.opacity(0.45).role("button").disabled(true).hit();
            }
            actions = actions.child(go);
        }
        actions = actions.child(
            text(provider_name.clone())
                .font(theme::caption())
                .color(pal.muted),
        );
        body = body.child(actions);

        if let Some(answer) = answer_section(card, pal, answer_max_height) {
            body = body.child(answer);
        }

        views.push(body.key(format!("card-{id}")).absolute(x, y));
    }
    views
}

fn answer_section(card: &CardUi, pal: &Palette, max_height: f32) -> Option<El> {
    let ai = &card.card;
    if ai.candidates.is_empty() {
        return None;
    }
    let id = ai.local_id;
    let selected = ai.selected.unwrap_or(ai.candidates.len().saturating_sub(1));
    let candidate = ai.candidates.get(selected)?;
    let streaming = matches!(ai.state, CardState::Streaming);
    let mut section = col().w_full().min_h(0.0).gap(6.0);

    let mut boxed = col()
        .w_full()
        .min_h(0.0)
        .padding(9.0)
        .radius(8.0)
        .gap(4.0)
        .fill(pal.hover)
        .border(
            1.0,
            if card.pinned {
                pal.selection
            } else {
                pal.border
            },
        );
    boxed = boxed.child(
        text(if candidate.complete {
            "Answer"
        } else {
            "Streaming…"
        })
        .font(theme::caption())
        .color(pal.muted),
    );
    let content = if candidate.text.is_empty() {
        "…".to_string()
    } else {
        candidate.text.clone()
    };
    // The answer area grows with its text up to the window-derived cap. Measuring the wrapped
    // text gives the natural height, so a short answer takes only what it needs; past the cap
    // the viewport is bounded and scrolls.
    let answer_height = answer_viewport_height(card, max_height);
    boxed = boxed.child(
        col().w_full().h(answer_height).child(
            col()
                // Polling resets this viewport to the top when generation starts or ends
                // (see `DoodleApp::poll_cards`), so a finished answer starts at its first line.
                .key(format!("card-answer-scroll-{id}"))
                .w_full()
                .grow(1.0)
                .min_h(0.0)
                .scroll(ScrollSpec {
                    stick_bottom: streaming,
                })
                .child(
                    col().w_full().child(
                        text(content)
                            .font(tok::type_::BODY)
                            .color(pal.text)
                            .w_full()
                            .wrap(),
                    ),
                ),
        ),
    );
    section = section.child(boxed);

    if ai.candidates.len() > 1 {
        let mut options = row().gap(6.0).flex_wrap();
        for (index, candidate) in ai.candidates.iter().enumerate() {
            if !candidate.complete {
                continue;
            }
            let selected_now = ai.selected == Some(index);
            options = options.child(
                theme::active_button(
                    format!("card-answer-{id}-{index}"),
                    &format!("Answer {}", index + 1),
                    selected_now,
                    pal,
                )
                .on_press(move |s: &mut DoodleApp| s.card_select(id, index)),
            );
        }
        section = section.child(options);
    }

    let can_pin = ai.pin_text().is_some() && !card.pinned;
    let mut footer = row().gap(8.0).items_center();
    let mut pin =
        theme::outlined_button(format!("card-pin-{id}"), "Pin answer", pal).label("Pin answer");
    if can_pin {
        pin = pin
            .focusable(true)
            // The pin dialog returns focus here on cancel (`focus_return` names the card);
            // this marks the button as the preferred control within it.
            .focus_selected(true)
            .on_press(move |s: &mut DoodleApp| s.preview_pin_answer(id));
    } else {
        // `disabled` stops focus but leaves no press target, so a press would reach the board;
        // `hit` keeps it on the button and consumes it.
        pin = pin.opacity(0.45).role("button").disabled(true).hit();
    }
    footer = footer.child(pin);
    if let CardState::Failed { message, partial } = &ai.state {
        footer = footer.child(
            text(if *partial {
                format!("Failed after partial text: {message}")
            } else {
                format!("Failed: {message}")
            })
            .font(theme::caption())
            .color(pal.danger),
        );
    }
    section = section.child(footer);
    Some(section)
}
