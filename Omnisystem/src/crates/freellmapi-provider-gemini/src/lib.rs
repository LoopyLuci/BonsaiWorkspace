//! FreeLLMAPI Gemini provider adapter - translates OpenAI-shaped chat requests to
//! Google's Gemini OpenAI-compatible endpoint and back.

pub mod gemini;

pub use gemini::GeminiAdapter;
