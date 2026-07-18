//! FreeLLMAPI Cohere provider adapter - translates OpenAI-shaped chat requests to
//! Cohere's chat API and back.

pub mod cohere;

pub use cohere::CohereAdapter;
