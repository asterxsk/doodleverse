use std::time::{Duration, Instant};

use doodle_ai::{AiCard, AiProvider, CardState, MascotInput, MascotMood, resolve_mascot};
use doodle_types::Point;
use eframe::egui;

use super::{DoodleApp, NoticeKind, screen_rect};

pub(crate) struct CardUi {
    pub card: AiCard,
    pub busy_started: Option<Instant>,
    pub mood: MascotMood,
    pub mood_started: Instant,
    pub hover_entry: Option<Instant>,
    pub pinned: bool,
    pub last_change: Instant,
}

impl CardUi {
    pub fn new(id: u64, world_pos: Point) -> Self {
        let now = Instant::now();
        Self {
            card: AiCard::new(id, world_pos),
            busy_started: None,
            mood: MascotMood::Idle,
            mood_started: now,
            hover_entry: None,
            pinned: false,
            last_change: now,
        }
    }

    pub fn mark_changed(&mut self, now: Instant) {
        self.last_change = now;
    }

    pub fn hover_active(&self, now: Instant) -> bool {
        self.hover_entry
            .is_some_and(|entry| now.duration_since(entry) < Duration::from_millis(600))
    }
}

pub(crate) struct PinPreview {
    pub card_id: u64,
    pub text: String,
}

enum CardAction {
    Delete(u64),
    Notice(String),
    PreviewPin(u64, String),
}

impl DoodleApp {
    pub(super) fn card_areas(&mut self, ctx: &egui::Context) {
        if self.cards.is_empty() {
            return;
        }
        let pal = self.palette();
        let viewport = screen_rect(self.canvas_rect);
        let mut actions: Vec<CardAction> = Vec::new();
        let now = Instant::now();
        let provider_name = self.provider.name();
        let provider_offline = self.provider.is_offline_demo();
        let reduced_motion = self.reduced_motion;
        let zoom = self.camera.zoom.max(0.02);

        let mascot = self.mascot.as_ref();
        for card in &mut self.cards {
            let screen = self.camera.world_to_screen(card.card.world_pos, viewport);
            let mut pos = egui::pos2(screen.x as f32, screen.y as f32);
            let card_id = card.card.local_id;
            let area_id = egui::Id::unique(("dibby-card", card_id));
            let mut header_drag = egui::Vec2::ZERO;
            egui::Area::new(area_id)
                .order(egui::Order::Foreground)
                .fixed_pos(pos)
                .constrain_to(self.canvas_rect)
                .show(ctx, |ui| {
                    egui::Frame::new()
                        .fill(pal.card)
                        .stroke(egui::Stroke::new(1.0, pal.border))
                        .corner_radius(14)
                        .inner_margin(egui::Margin::same(12))
                        .show(ui, |ui| {
                            ui.set_width(332.0);
                            ui.horizontal(|ui| {
                                let (avatar_rect, response) = ui.allocate_exact_size(
                                    egui::vec2(46.0, 46.0),
                                    egui::Sense::click_and_drag(),
                                );
                                header_drag = response.drag_delta();
                                let avatar_hovered = response.hovered() || response.has_focus();
                                if avatar_hovered {
                                    if card.hover_entry.is_none() {
                                        card.hover_entry = Some(now);
                                    }
                                } else {
                                    card.hover_entry = None;
                                }
                                response.on_hover_text("Drag to move this card");
                                let hover_elapsed = card
                                    .hover_entry
                                    .map(|entry| now.duration_since(entry).as_millis() as u64);
                                let busy_elapsed = card
                                    .busy_started
                                    .map(|entry| now.duration_since(entry).as_millis() as u64);
                                let mood = resolve_mascot(MascotInput {
                                    reduced_motion,
                                    talking: matches!(card.card.state, CardState::Streaming),
                                    hover_elapsed_ms: hover_elapsed,
                                    busy_elapsed_ms: busy_elapsed,
                                });
                                if mood != card.mood {
                                    card.mood = mood;
                                    card.mood_started = now;
                                }
                                let mood_elapsed =
                                    now.duration_since(card.mood_started).as_millis() as u64;
                                if let Some(mascot) = mascot {
                                    mascot.paint(
                                        ui.painter(),
                                        avatar_rect.shrink(2.0),
                                        mood,
                                        mood_elapsed,
                                    );
                                }
                                ui.vertical(|ui| {
                                    ui.label(
                                        egui::RichText::new("Dibby")
                                            .strong()
                                            .color(pal.text)
                                            .size(14.5),
                                    );
                                    let status = match &card.card.state {
                                        CardState::Draft => "Ask a question to begin".to_string(),
                                        CardState::Submitting => "Starting…".to_string(),
                                        CardState::Streaming => "Generating…".to_string(),
                                        CardState::Alternatives if card.pinned => {
                                            "Pinned to board".to_string()
                                        }
                                        CardState::Alternatives => "Answer ready".to_string(),
                                        CardState::Failed { message, partial } => {
                                            let note = if *partial { " (partial)" } else { "" };
                                            format!("Error{note}: {message}")
                                        }
                                        CardState::Cancelled => "Cancelled".to_string(),
                                    };
                                    ui.label(egui::RichText::new(status).size(11.5).color(
                                        if matches!(card.card.state, CardState::Failed { .. }) {
                                            pal.danger
                                        } else {
                                            pal.muted
                                        },
                                    ));
                                });
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui
                                            .small_button("×")
                                            .on_hover_text("Close this local card")
                                            .clicked()
                                        {
                                            actions.push(CardAction::Delete(card_id));
                                        }
                                    },
                                );
                            });

                            ui.add_space(6.0);
                            let mut question = card.card.question.clone();
                            let response = ui.add(
                                egui::TextEdit::multiline(&mut question)
                                    .hint_text("Ask Dibby for a doodle idea…")
                                    .desired_rows(2)
                                    .desired_width(f32::INFINITY),
                            );
                            if response.changed() {
                                card.card.set_question(question);
                                card.busy_started = None;
                            }

                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                                let busy = card.card.is_busy();
                                if busy {
                                    if ui.button("Cancel").clicked() {
                                        card.card.cancel();
                                        card.busy_started = None;
                                    }
                                } else {
                                    let label = if card.card.candidates.is_empty() {
                                        "Generate"
                                    } else {
                                        "Generate another"
                                    };
                                    let enabled = !card.card.question.trim().is_empty();
                                    if ui.add_enabled(enabled, egui::Button::new(label)).clicked() {
                                        card.busy_started = Some(Instant::now());
                                        match card.card.submit(&self.provider, None) {
                                            Ok(()) => card.pinned = false,
                                            Err(error) => {
                                                card.busy_started = None;
                                                actions.push(CardAction::Notice(format!(
                                                    "Could not start generation: {error}"
                                                )));
                                            }
                                        }
                                    }
                                }
                                ui.label(
                                    egui::RichText::new(if provider_offline {
                                        provider_name.to_string()
                                    } else {
                                        provider_name.clone()
                                    })
                                    .size(11.0)
                                    .color(pal.muted),
                                );
                            });

                            ui.add_space(6.0);
                            card_answer_ui(ui, card, &pal, &mut actions);
                        });
                });

            let drag_delta = header_drag;
            if drag_delta != egui::Vec2::ZERO {
                card.card.world_pos.x += f64::from(drag_delta.x) / zoom;
                card.card.world_pos.y += f64::from(drag_delta.y) / zoom;
                card.mark_changed(now);
            }
            pos = egui::pos2(pos.x + drag_delta.x, pos.y + drag_delta.y);
            let _ = pos;
        }

        for action in actions {
            match action {
                CardAction::Delete(card_id) => {
                    self.cards.retain(|card| card.card.local_id != card_id);
                    self.set_notice(NoticeKind::Info, "Closed the local AI card");
                }
                CardAction::Notice(message) => self.set_notice(NoticeKind::Error, message),
                CardAction::PreviewPin(card_id, text) => {
                    self.pin_preview = Some(PinPreview { card_id, text });
                }
            }
        }
    }

    pub(super) fn pin_modal(&mut self, ctx: &egui::Context) {
        let Some(preview) = self.pin_preview.as_ref() else {
            return;
        };
        let pal = self.palette();
        let card_id = preview.card_id;
        let text = preview.text.clone();
        let mut confirm = false;
        let mut cancel = false;
        egui::Modal::new(egui::Id::unique("pin-answer-modal")).show(ctx, |ui| {
            ui.set_width(420.0);
            ui.heading("Pin this answer?");
            ui.label(
                egui::RichText::new(
                    "Only the selected answer below becomes shared board content. Your question and \
                     other alternatives stay local to this session.",
                )
                .color(pal.muted)
                .size(12.5),
            );
            ui.add_space(8.0);
            egui::Frame::new()
                .fill(egui::Color32::from_rgb(255, 241, 186))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(216, 198, 132)))
                .corner_radius(8)
                .inner_margin(egui::Margin::same(10))
                .show(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .max_height(240.0)
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new(&text)
                                    .color(egui::Color32::from_rgb(45, 40, 28))
                                    .size(13.5),
                            );
                        });
                });
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui.button("Pin answer").clicked() {
                    confirm = true;
                }
                if ui.button("Cancel").clicked() {
                    cancel = true;
                }
            });
        });
        if confirm {
            self.confirm_pin(card_id);
        } else if cancel {
            self.pin_preview = None;
        }
    }
}

fn card_answer_ui(
    ui: &mut egui::Ui,
    card: &mut CardUi,
    pal: &super::Palette,
    actions: &mut Vec<CardAction>,
) {
    let ai = &mut card.card;
    if ai.candidates.is_empty() {
        return;
    }
    let selected = ai.selected.unwrap_or(ai.candidates.len().saturating_sub(1));
    if let Some(candidate) = ai.candidates.get(selected) {
        egui::Frame::new()
            .fill(if ui.visuals().dark_mode {
                egui::Color32::from_rgb(44, 43, 51)
            } else {
                egui::Color32::from_rgb(248, 245, 238)
            })
            .stroke(egui::Stroke::new(1.0, pal.border))
            .corner_radius(8)
            .inner_margin(egui::Margin::same(8))
            .show(ui, |ui| {
                let label = if candidate.complete {
                    "Answer"
                } else {
                    "Streaming…"
                };
                ui.label(egui::RichText::new(label).size(10.5).color(pal.muted));
                egui::ScrollArea::vertical()
                    .max_height(170.0)
                    .stick_to_bottom(matches!(ai.state, CardState::Streaming))
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new(if candidate.text.is_empty() {
                                "…"
                            } else {
                                candidate.text.as_str()
                            })
                            .color(pal.text)
                            .size(13.0),
                        );
                    });
            });
    }
    if ai.candidates.len() > 1 {
        ui.add_space(4.0);
        let options: Vec<(usize, bool)> = ai
            .candidates
            .iter()
            .enumerate()
            .map(|(index, candidate)| (index, candidate.complete))
            .collect();
        ui.horizontal_wrapped(|ui| {
            for (index, complete) in options {
                let label = format!("Answer {}", index + 1);
                let selected_now = ai.selected == Some(index);
                if ui
                    .add_enabled(complete, egui::Button::new(label).selected(selected_now))
                    .clicked()
                {
                    ai.select(index);
                    card.pinned = false;
                }
            }
        });
    }
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        let can_pin = ai.pin_text().is_some() && !card.pinned;
        if ui
            .add_enabled(can_pin, egui::Button::new("Pin answer"))
            .on_hover_text("Share only this completed answer to the board")
            .clicked()
            && let Some(text) = ai.pin_text()
        {
            actions.push(CardAction::PreviewPin(ai.local_id, text.to_string()));
        }
        if let CardState::Failed { message, partial } = &ai.state {
            ui.label(
                egui::RichText::new(if *partial {
                    format!("Failed after partial text: {message}")
                } else {
                    format!("Failed: {message}")
                })
                .size(11.0)
                .color(pal.danger),
            );
        }
    });
}
