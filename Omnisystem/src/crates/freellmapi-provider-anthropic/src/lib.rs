//! FreeLLMAPI Anthropic provider adapter - translates OpenAI-shaped chat requests
//! to Anthropic's Messages API and back.

pub mod anthropic;

pub use anthropic::AnthropicAdapter;
