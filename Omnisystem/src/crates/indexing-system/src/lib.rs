//! indexing-system: a small hybrid (lexical + vector) search engine.
//!
//! - [`lexical`]: tokenizer and a real BM25 ranking implementation.
//! - [`vector`]: cosine-similarity nearest-neighbor search over embeddings.
//! - [`query`]: [`query::SearchEngine`] ties the tokenizer and BM25 index
//!   together into a single index/search API over plain-text documents.

pub mod lexical;
pub mod query;
pub mod vector;

pub use lexical::{Tokenizer, BM25};
pub use query::{SearchEngine, SearchResult};
pub use vector::{Embedding, HnswIndex};
