//! FreeLLMAPI Mistral provider adapter - translates OpenAI-shaped chat requests to
//! Mistral's chat completions API and back.

pub mod mistral;

pub use mistral::MistralAdapter;
