pub mod core;

#[derive(Debug, Clone)]
pub struct Document { pub id: String, pub content: String }

#[derive(Debug, Clone)]
pub struct Query { pub text: String }

#[derive(Debug, Clone)]
pub struct SearchResult { pub doc: Document, pub score: f64 }

#[derive(Debug)]
pub enum SearchError { NotFound }

pub type Result<T> = std::result::Result<T, SearchError>;

pub use core::*;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_basic() {
        let _doc = Document { id: "1".into(), content: "test".into() };
    }
    
    #[test]
    fn test_query() {
        let _query = Query { text: "search".into() };
    }
}
