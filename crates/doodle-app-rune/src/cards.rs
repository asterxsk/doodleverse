use doodle_ai::{AiCard, AiProvider, CardState, MascotInput, MascotMood, resolve_mascot};
use doodle_types::Point;
use rune::tok::{self, type_};
use rune::{El, ScrollSpec, TextEdit, col, edit_access, el, row, text, text_input};

use crate::app::DoodleApp;
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

/// One absolutely positioned element per local AI card, in window points.
pub fn card_views(s: &DoodleApp, pal: &Palette) -> Vec<El> {
    let viewport = s.viewport();
    let provider_name = s.provider.name().to_string();
    let mut views = Vec::with_capacity(s.cards.len());
    for card in &s.cards {
        let screen = s.camera.world_to_screen(card.card.world_pos, viewport);
        let id = card.card.local_id;

        let mut body = col()
            .hit()
            .w(CARD_WIDTH)
            .padding(12.0)
            .gap(8.0)
            .radius(14.0)
            .fill(pal.card)
            .border(1.0, pal.border)
            .shadow(10.0, 28.0, pal.shadow);

        // Header: dragging the avatar moves the card.
        let mood_elapsed = ((s.clock - card.mood_started).max(0.0) * 1000.0) as u64;
        let mut avatar = el()
            .size(44.0, 44.0)
            .radius(12.0)
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
                theme::flat_button(format!("card-close-{id}"), "×", pal)
                    .on_press(move |s: &mut DoodleApp| s.close_card(id)),
            );
        body = body.child(header);

        let field = text_input(&card.question, tok::type_::BODY)
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
                .radius(9.0)
                .fill(pal.hover)
                .border(1.0, pal.border)
                .child(field),
        );

        let busy = card.card.is_busy();
        let mut actions = row().gap(8.0).items_center();
        if busy {
            actions = actions.child(
                theme::button(format!("card-cancel-{id}"), "Cancel", pal)
                    .on_press(move |s: &mut DoodleApp| s.card_cancel(id)),
            );
        } else {
            let label = if card.card.candidates.is_empty() {
                "Generate"
            } else {
                "Generate another"
            };
            let enabled = !card.question.text.trim().is_empty();
            let mut go = theme::button(format!("card-go-{id}"), label, pal);
            if enabled {
                go = go.on_press(move |s: &mut DoodleApp| s.card_submit(id));
            } else {
                go = go.opacity(0.45);
            }
            actions = actions.child(go);
        }
        actions = actions.child(
            text(provider_name.clone())
                .font(theme::caption())
                .color(pal.muted),
        );
        body = body.child(actions);

        if let Some(answer) = answer_section(card, pal) {
            body = body.child(answer);
        }

        views.push(body.absolute(screen.x as f32, screen.y as f32));
    }
    views
}

fn answer_section(card: &CardUi, pal: &Palette) -> Option<El> {
    let ai = &card.card;
    if ai.candidates.is_empty() {
        return None;
    }
    let id = ai.local_id;
    let selected = ai.selected.unwrap_or(ai.candidates.len().saturating_sub(1));
    let candidate = ai.candidates.get(selected)?;
    let streaming = matches!(ai.state, CardState::Streaming);
    let mut section = col().w_full().gap(6.0);

    let mut boxed = col()
        .w_full()
        .padding(9.0)
        .radius(9.0)
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
    boxed = boxed.child(
        el().w_full()
            .max_h(170.0)
            .scroll(ScrollSpec {
                stick_bottom: streaming,
            })
            .child(
                text(content)
                    .font(tok::type_::BODY)
                    .color(pal.text)
                    .w_full()
                    .wrap(),
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
    let mut pin = theme::button(format!("card-pin-{id}"), "Pin answer", pal);
    if can_pin {
        pin = pin.on_press(move |s: &mut DoodleApp| s.preview_pin_answer(id));
    } else {
        pin = pin.opacity(0.45);
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
