//! FreeLLMAPI Groq provider adapter - translates OpenAI-shaped chat requests to
//! Groq's OpenAI-compatible chat completions API and back.

pub mod groq;

pub use groq::GroqAdapter;
