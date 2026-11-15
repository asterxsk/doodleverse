use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::Duration;

use doodle_types::{MAX_ANSWER_BYTES, MAX_QUESTION_BYTES, RequestId};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerationRequest {
    pub request_id: RequestId,
    pub question: String,
    /// Short brainstorming instruction. Never board content.
    pub instruction: Option<String>,
}

impl GenerationRequest {
    pub fn new(question: impl Into<String>) -> Result<Self, ProviderError> {
        let question = question.into();
        if question.trim().is_empty() {
            return Err(ProviderError::EmptyQuestion);
        }
        if question.len() > MAX_QUESTION_BYTES {
            return Err(ProviderError::QuestionTooLong);
        }
        Ok(Self {
            request_id: RequestId::new(),
            question,
            instruction: None,
        })
    }

    #[must_use]
    pub fn with_instruction(mut self, instruction: impl Into<String>) -> Self {
        let instruction = instruction.into();
        if !instruction.trim().is_empty() && instruction.len() <= 4096 {
            self.instruction = Some(instruction);
        }
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderEvent {
    Started,
    TextDelta(String),
    Completed,
    Failed { message: String, partial: bool },
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ProviderError {
    #[error("question is empty")]
    EmptyQuestion,
    #[error("question is too long")]
    QuestionTooLong,
    #[error("answer exceeded the size limit")]
    AnswerTooLong,
    #[error("provider unavailable")]
    Unavailable,
}

pub trait AiProvider: Send + Sync {
    /// Human-readable provider name shown in the card.
    fn name(&self) -> String;
    /// True when this provider never performs network inference.
    fn is_offline_demo(&self) -> bool;
    fn start(&self, request: GenerationRequest) -> Result<GenerationHandle, ProviderError>;
}

/// Owns one provider stream. Dropping the handle requests cancellation.
pub struct GenerationHandle {
    request_id: RequestId,
    receiver: Receiver<ProviderEvent>,
    cancel: Arc<AtomicBool>,
    finished: bool,
    total_bytes: usize,
}

impl std::fmt::Debug for GenerationHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GenerationHandle")
            .field("request_id", &self.request_id)
            .field("finished", &self.finished)
            .finish_non_exhaustive()
    }
}

impl GenerationHandle {
    #[must_use]
    pub fn request_id(&self) -> RequestId {
        self.request_id
    }

    #[must_use]
    pub fn is_finished(&self) -> bool {
        self.finished
    }

    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    /// Non-blocking poll. Returns `None` once the stream is fully drained.
    pub fn poll(&mut self) -> Option<ProviderEvent> {
        if self.finished {
            return None;
        }
        match self.receiver.try_recv() {
            Ok(event) => {
                if let ProviderEvent::TextDelta(text) = &event {
                    self.total_bytes = self.total_bytes.saturating_add(text.len());
                }
                if matches!(
                    event,
                    ProviderEvent::Completed
                        | ProviderEvent::Failed { .. }
                        | ProviderEvent::Cancelled
                ) {
                    self.finished = true;
                }
                Some(event)
            }
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                self.finished = true;
                Some(ProviderEvent::Failed {
                    message: "provider stream ended unexpectedly".into(),
                    partial: self.total_bytes > 0,
                })
            }
        }
    }
}

impl Drop for GenerationHandle {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

/// Deterministic, fully offline brainstorming stream. Clearly labeled as a demo in the UI.
#[derive(Debug, Clone)]
pub struct MockProvider {
    pub chunk_delay: Duration,
    pub chunk_size: usize,
}

/// Splits text into byte-sized chunks without breaking a UTF-8 character.
fn chunk_text(text: &str, chunk_size: usize) -> Vec<String> {
    let limit = chunk_size.max(1);
    let mut chunks = Vec::new();
    let mut current = String::new();
    for ch in text.chars() {
        if !current.is_empty() && current.len() + ch.len_utf8() > limit {
            chunks.push(std::mem::take(&mut current));
        }
        current.push(ch);
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    chunks
}

impl Default for MockProvider {
    fn default() -> Self {
        Self {
            chunk_delay: Duration::from_millis(45),
            chunk_size: 18,
        }
    }
}

impl MockProvider {
    #[must_use]
    pub fn instant() -> Self {
        Self {
            chunk_delay: Duration::ZERO,
            chunk_size: 4096,
        }
    }

    fn response(question: &str) -> String {
        let topic = question.trim().trim_end_matches(['?', '.', '!']);
        format!(
            "A few doodle directions for “{topic}”:\n\n1. Draw the simplest version first — one bold outline, no shading.\n2. Add a tiny surprising detail in the corner so the idea feels yours.\n3. Try a second pass in a different color to show movement or contrast.\n\nThis is an offline demo response from Dibby. It never leaves your machine."
        )
    }
}

impl AiProvider for MockProvider {
    fn name(&self) -> String {
        "Dibby demo (offline)".into()
    }

    fn is_offline_demo(&self) -> bool {
        true
    }

    fn start(&self, request: GenerationRequest) -> Result<GenerationHandle, ProviderError> {
        let cancel = Arc::new(AtomicBool::new(false));
        let cancel_thread = Arc::clone(&cancel);
        let (sender, receiver) = mpsc::channel();
        let response = Self::response(&request.question);
        let delay = self.chunk_delay;
        let chunk_size = self.chunk_size.max(1);
        thread::Builder::new()
            .name("dibby-demo-stream".into())
            .spawn(move || {
                let _ = sender.send(ProviderEvent::Started);
                let mut emitted = 0usize;
                for chunk in chunk_text(&response, chunk_size) {
                    if cancel_thread.load(Ordering::Relaxed) {
                        let _ = sender.send(ProviderEvent::Cancelled);
                        return;
                    }
                    if !delay.is_zero() {
                        thread::sleep(delay);
                    }
                    emitted += chunk.len();
                    if emitted > MAX_ANSWER_BYTES {
                        let _ = sender.send(ProviderEvent::Failed {
                            message: "answer exceeded the size limit".into(),
                            partial: true,
                        });
                        return;
                    }
                    if sender.send(ProviderEvent::TextDelta(chunk)).is_err() {
                        return;
                    }
                }
                if cancel_thread.load(Ordering::Relaxed) {
                    let _ = sender.send(ProviderEvent::Cancelled);
                } else {
                    let _ = sender.send(ProviderEvent::Completed);
                }
            })
            .map_err(|_| ProviderError::Unavailable)?;
        Ok(GenerationHandle {
            request_id: request.request_id,
            receiver,
            cancel,
            finished: false,
            total_bytes: 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drain(handle: &mut GenerationHandle) -> Vec<ProviderEvent> {
        let mut events = Vec::new();
        for _ in 0..1000 {
            while let Some(event) = handle.poll() {
                events.push(event);
            }
            if handle.is_finished() {
                break;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        events
    }

    #[test]
    fn mock_stream_completes() {
        let provider = MockProvider::instant();
        let request = GenerationRequest::new("How should I draw a cat?").expect("request");
        let mut handle = provider.start(request).expect("start");
        let events = drain(&mut handle);
        assert!(matches!(events.first(), Some(ProviderEvent::Started)));
        assert!(matches!(events.last(), Some(ProviderEvent::Completed)));
        assert!(
            events
                .iter()
                .any(|event| matches!(event, ProviderEvent::TextDelta(_)))
        );
    }

    #[test]
    fn mock_stream_never_splits_multibyte_characters() {
        let provider = MockProvider {
            chunk_delay: Duration::ZERO,
            chunk_size: 1,
        };
        let question = "How do I draw café “quotes”?";
        let request = GenerationRequest::new(question).expect("request");
        let mut handle = provider.start(request).expect("start");
        let events = drain(&mut handle);
        assert!(
            events
                .iter()
                .all(|event| !matches!(event, ProviderEvent::Failed { .. })),
            "unexpected failure: {events:?}"
        );
        let text: String = events
            .iter()
            .filter_map(|event| match event {
                ProviderEvent::TextDelta(text) => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(text, MockProvider::response(question));
        assert!(matches!(events.last(), Some(ProviderEvent::Completed)));
    }

    #[test]
    fn empty_question_is_rejected() {
        assert_eq!(
            GenerationRequest::new("   "),
            Err(ProviderError::EmptyQuestion)
        );
    }

    #[test]
    fn cancel_reports_cancelled() {
        let provider = MockProvider {
            chunk_delay: Duration::from_millis(5),
            chunk_size: 4,
        };
        let request = GenerationRequest::new("cancel me").expect("request");
        let mut handle = provider.start(request).expect("start");
        handle.cancel();
        let mut last = None;
        for _ in 0..200 {
            if let Some(event) = handle.poll() {
                last = Some(event);
                if handle.is_finished() {
                    break;
                }
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        assert_eq!(last, Some(ProviderEvent::Cancelled));
    }
}
