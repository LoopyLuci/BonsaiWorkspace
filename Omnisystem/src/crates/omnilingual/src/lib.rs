//! omnilingual: small building blocks for dictionary-based machine
//! translation -- sentence/word segmentation, a per-language-pair
//! [`dictionary::Dictionary`], a domain-scoped [`terminology::TerminologyStore`],
//! a monotonic [`alignment::WordAligner`] baseline, and a [`translator::Translator`]
//! that actually looks words up in the registered dictionary.

pub mod alignment;
pub mod dictionary;
pub mod error;
pub mod segmentation;
pub mod terminology;
pub mod translator;

pub use alignment::WordAligner;
pub use dictionary::Dictionary;
pub use error::{Error, Result};
pub use segmentation::Segmenter;
pub use terminology::{TermEntry, TerminologyStore};
pub use translator::Translator;
