mod error;
mod types;
mod engine;

pub use error::{SemanticError, SemanticResult};
pub use types::{Embedding, SemanticIndex, SimilarityResult, RankedResult, SemanticQuery};
pub use engine::SemanticSearchEngine;
