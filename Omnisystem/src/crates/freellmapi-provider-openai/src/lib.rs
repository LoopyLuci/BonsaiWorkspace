//! FreeLLMAPI OpenAI provider adapter - translates OpenAI-shaped chat requests to
//! the OpenAI chat completions API and back.

pub mod openai;

pub use openai::OpenAIAdapter;
