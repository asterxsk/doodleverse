use doodle_types::{MAX_AI_ALTERNATIVES, MAX_ANSWER_BYTES, Point};
use thiserror::Error;

use crate::provider::{
    AiProvider, GenerationHandle, GenerationRequest, ProviderError, ProviderEvent,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub text: String,
    pub complete: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CardState {
    Draft,
    Submitting,
    Streaming,
    Alternatives,
    Failed { message: String, partial: bool },
    Cancelled,
}

impl CardState {
    #[must_use]
    pub fn is_busy(&self) -> bool {
        matches!(self, Self::Submitting | Self::Streaming)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum CardError {
    #[error("question is empty")]
    EmptyQuestion,
    #[error("too many alternatives")]
    TooManyAlternatives,
    #[error("provider unavailable")]
    ProviderUnavailable,
}

impl From<ProviderError> for CardError {
    fn from(value: ProviderError) -> Self {
        match value {
            ProviderError::EmptyQuestion => Self::EmptyQuestion,
            ProviderError::Unavailable => Self::ProviderUnavailable,
            _ => Self::ProviderUnavailable,
        }
    }
}

/// Entirely local AI card. Position, question, and unfinished candidates never enter the shared
/// document, room messages, or portable files. Pinning copies the selected completed answer.
#[derive(Debug)]
pub struct AiCard {
    pub local_id: u64,
    pub world_pos: Point,
    pub question: String,
    pub state: CardState,
    pub candidates: Vec<Candidate>,
    pub selected: Option<usize>,
    active_index: Option<usize>,
    generation: Option<GenerationHandle>,
    generation_serial: u64,
}

impl AiCard {
    #[must_use]
    pub fn new(local_id: u64, world_pos: Point) -> Self {
        Self {
            local_id,
            world_pos,
            question: String::new(),
            state: CardState::Draft,
            candidates: Vec::new(),
            selected: None,
            active_index: None,
            generation: None,
            generation_serial: 0,
        }
    }

    pub fn set_question(&mut self, question: String) {
        if question == self.question {
            return;
        }
        self.cancel();
        self.question = question;
        self.candidates.clear();
        self.selected = None;
        self.active_index = None;
        self.state = CardState::Draft;
    }

    /// Explicit local generation. This is the only method that starts provider work.
    pub fn submit(
        &mut self,
        provider: &dyn AiProvider,
        instruction: Option<&str>,
    ) -> Result<(), CardError> {
        let mut request = GenerationRequest::new(self.question.clone())?;
        if let Some(instruction) = instruction {
            request = request.with_instruction(instruction);
        }
        if self.candidates.len() >= MAX_AI_ALTERNATIVES {
            return Err(CardError::TooManyAlternatives);
        }
        self.cancel();
        self.generation_serial += 1;
        self.candidates.push(Candidate {
            text: String::new(),
            complete: false,
        });
        let index = self.candidates.len() - 1;
        self.active_index = Some(index);
        self.selected = None;
        match provider.start(request) {
            Ok(handle) => {
                self.generation = Some(handle);
                self.state = CardState::Submitting;
                Ok(())
            }
            Err(error) => {
                self.candidates.pop();
                self.active_index = None;
                self.state = CardState::Failed {
                    message: error.to_string(),
                    partial: false,
                };
                Err(error.into())
            }
        }
    }

    /// Drains provider events. Returns true when the visible state changed.
    pub fn poll(&mut self) -> bool {
        let mut changed = false;
        while let Some(handle) = self.generation.as_mut() {
            let Some(event) = handle.poll() else {
                break;
            };
            changed = true;
            match event {
                ProviderEvent::Started => {
                    self.state = CardState::Streaming;
                }
                ProviderEvent::TextDelta(text) => {
                    let Some(index) = self.active_index else {
                        continue;
                    };
                    let Some(candidate) = self.candidates.get_mut(index) else {
                        continue;
                    };
                    if candidate.text.len() + text.len() <= MAX_ANSWER_BYTES {
                        candidate.text.push_str(&text);
                    }
                    self.state = CardState::Streaming;
                }
                ProviderEvent::Completed => {
                    let partial = self.finish_candidate(true);
                    self.generation = None;
                    self.state = if partial {
                        CardState::Failed {
                            message: "provider completed without an answer".into(),
                            partial: false,
                        }
                    } else {
                        CardState::Alternatives
                    };
                }
                ProviderEvent::Failed { message, partial } => {
                    self.finish_candidate(false);
                    self.generation = None;
                    self.state = CardState::Failed { message, partial };
                }
                ProviderEvent::Cancelled => {
                    self.finish_candidate(false);
                    self.generation = None;
                    self.state = CardState::Cancelled;
                }
            }
            if self.generation.is_none() {
                break;
            }
        }
        changed
    }

    fn finish_candidate(&mut self, complete: bool) -> bool {
        let Some(index) = self.active_index.take() else {
            return false;
        };
        match self.candidates.get_mut(index) {
            Some(candidate) => {
                candidate.complete = complete;
                let empty = candidate.text.trim().is_empty();
                if complete {
                    self.selected = Some(index);
                }
                empty
            }
            None => true,
        }
    }

    pub fn cancel(&mut self) {
        let was_busy = self.state.is_busy();
        if let Some(handle) = self.generation.as_ref() {
            handle.cancel();
        }
        if let Some(index) = self.active_index.take()
            && let Some(candidate) = self.candidates.get_mut(index)
        {
            candidate.complete = false;
        }
        self.generation = None;
        if was_busy {
            self.state = CardState::Cancelled;
        }
    }

    pub fn select(&mut self, index: usize) -> bool {
        if self
            .candidates
            .get(index)
            .is_some_and(|candidate| candidate.complete)
        {
            self.selected = Some(index);
            true
        } else {
            false
        }
    }

    /// Text eligible for an explicit pin. Only a completed selected candidate qualifies.
    #[must_use]
    pub fn pin_text(&self) -> Option<&str> {
        let index = self.selected?;
        let candidate = self.candidates.get(index)?;
        if candidate.complete && !candidate.text.trim().is_empty() {
            Some(candidate.text.as_str())
        } else {
            None
        }
    }

    #[must_use]
    pub fn selected_text(&self) -> Option<&str> {
        self.selected
            .and_then(|index| self.candidates.get(index))
            .map(|candidate| candidate.text.as_str())
    }

    #[must_use]
    pub fn is_busy(&self) -> bool {
        self.state.is_busy() || self.generation.is_some()
    }

    #[must_use]
    pub fn generation_serial(&self) -> u64 {
        self.generation_serial
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::MockProvider;
    use std::time::Duration;

    fn wait_for_idle(card: &mut AiCard) {
        for _ in 0..500 {
            card.poll();
            if !card.is_busy() {
                return;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        panic!("mock stream did not finish");
    }

    #[test]
    fn full_flow_pins_only_completed_selected_text() {
        let provider = MockProvider::instant();
        let mut card = AiCard::new(1, Point::default());
        card.set_question("Ideas for a paper robot".into());
        card.submit(&provider, None).expect("submit");
        wait_for_idle(&mut card);
        assert_eq!(card.state, CardState::Alternatives);
        assert_eq!(card.candidates.len(), 1);
        assert!(card.candidates[0].complete);
        let text = card.pin_text().expect("pin text").to_string();
        assert!(text.contains("paper robot"));
        assert!(!text.contains("Ideas for a paper robot?"));
    }

    #[test]
    fn changing_question_clears_old_candidates() {
        let provider = MockProvider::instant();
        let mut card = AiCard::new(1, Point::default());
        card.set_question("one".into());
        card.submit(&provider, None).expect("submit");
        wait_for_idle(&mut card);
        card.set_question("two".into());
        assert!(card.candidates.is_empty());
        assert_eq!(card.state, CardState::Draft);
        assert!(card.pin_text().is_none());
    }

    #[test]
    fn cancellation_keeps_partial_text_but_not_pinnable() {
        let provider = MockProvider {
            chunk_delay: Duration::from_millis(4),
            chunk_size: 3,
        };
        let mut card = AiCard::new(1, Point::default());
        card.set_question("cancel".into());
        card.submit(&provider, None).expect("submit");
        std::thread::sleep(Duration::from_millis(30));
        card.poll();
        let partial_len = card.candidates[0].text.len();
        card.cancel();
        assert_eq!(card.state, CardState::Cancelled);
        assert!(!card.candidates[0].complete);
        assert!(card.pin_text().is_none());
        assert!(card.candidates[0].text.len() >= partial_len);
    }

    #[test]
    fn alternatives_accumulate() {
        let provider = MockProvider::instant();
        let mut card = AiCard::new(1, Point::default());
        card.set_question("alternatives".into());
        card.submit(&provider, None).expect("submit");
        wait_for_idle(&mut card);
        card.submit(&provider, None).expect("second");
        wait_for_idle(&mut card);
        assert_eq!(card.candidates.len(), 2);
        assert_eq!(card.selected, Some(1));
    }
}
