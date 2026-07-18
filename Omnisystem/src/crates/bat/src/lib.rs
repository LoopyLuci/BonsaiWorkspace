//! bat: a scalable transformer ("Big/Batched Adaptive Transformer") toy
//! inference engine. Real matrix-based forward pass (attention + FFN with
//! residuals) and greedy autoregressive decoding over randomly-initialized
//! (untrained) weights, plus a parameter-count-driven architecture scaling
//! heuristic. Not a production model -- there is no trained checkpoint
//! loading or tokenizer -- but the computation itself is genuine, not a
//! passthrough stub.

pub mod config;
pub mod inference;
pub mod layers;
pub mod scaling;

pub use config::BatConfig;
pub use inference::BatEngine;
pub use layers::TransformerBlock;
pub use scaling::ScaleMap;
