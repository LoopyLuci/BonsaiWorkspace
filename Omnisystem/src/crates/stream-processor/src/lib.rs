mod error;
mod types;
mod processor;

pub use error::{StreamError, StreamResult};
pub use types::{StreamEvent, StreamWindow, WindowType, Aggregation, StreamState, ProcessedResult};
pub use processor::StreamProcessor;
