//! FreeLLMAPI HuggingFace provider adapter - translates OpenAI-shaped chat requests
//! into HuggingFace Inference API prompts and back.

pub mod huggingface;

pub use huggingface::HuggingFaceAdapter;
