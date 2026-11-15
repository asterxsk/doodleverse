//! AI-card core.
//!
//! This boundary is deliberately narrow: a request carries exactly one typed question and an
//! optional short brainstorming instruction. It never carries a board, document, peer list, file
//! path, tool, or conversation history. Only an explicit local Generate action invokes a provider.

pub mod card;
pub mod mascot;
pub mod provider;

pub use card::{AiCard, Candidate, CardError, CardState};
pub use mascot::{
    AtlasFrame, AtlasManifest, MascotInput, MascotMood, frame_for_mood, frame_uv, parse_manifest,
    resolve_mascot,
};
pub use provider::{
    AiProvider, GenerationHandle, GenerationRequest, MockProvider, ProviderError, ProviderEvent,
};
