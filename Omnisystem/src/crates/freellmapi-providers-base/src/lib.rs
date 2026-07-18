//! FreeLLMAPI Providers Base - the shared `ProviderAdapter` trait and request/response
//! types every per-provider adapter crate (groq, openai, anthropic, mistral, gemini,
//! cohere, huggingface) implements against.

pub mod adapter;
pub mod errors;
pub mod types;

pub use adapter::ProviderAdapter;
pub use errors::ProviderError;
pub use types::{Choice, Message, ProviderConfig, ProviderRequest, ProviderResponse, Usage};
