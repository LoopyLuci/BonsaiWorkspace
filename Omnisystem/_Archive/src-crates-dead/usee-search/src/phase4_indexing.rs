use dashmap::DashMap;
use std::sync::Arc;

pub struct InvertedIndex {
    terms: Arc<DashMap<String, TermEntry>>,
}

pub struct TermEntry {
    pub postings: Vec<Posting>,
    pub frequency: u64,
    pub df: u32,
}

pub struct Posting {
    pub doc_id: String,
    pub positions: Vec<u32>,
    pub tf: u32,
}

pub struct PhraseBitmap {
    phrase: String,
    docs: Vec<String>,
}

pub struct NgramIndex {
    ngrams: Arc<DashMap<String, Vec<String>>>,
}

impl InvertedIndex {
    pub fn new() -> Self {
        InvertedIndex {
            terms: Arc::new(DashMap::new()),
        }
    }

    pub fn index_term(&self, term: &str, doc_id: &str, position: u32) {
        self.terms
            .entry(term.to_string())
            .or_insert(TermEntry {
                postings: Vec::new(),
                frequency: 0,
                df: 0,
            })
            .postings
            .push(Posting {
                doc_id: doc_id.to_string(),
                positions: vec![position],
                tf: 1,
            });
    }

    pub fn search_term(&self, term: &str) -> Vec<Posting> {
        self.terms
            .get(term)
            .map(|entry| entry.postings.clone())
            .unwrap_or_default()
    }

    pub fn search_phrase(&self, phrase: &str) -> Vec<String> {
        let terms: Vec<&str> = phrase.split_whitespace().collect();
        if terms.is_empty() {
            return Vec::new();
        }

        let mut results = self.search_term(terms[0]).iter().map(|p| p.doc_id.clone()).collect::<Vec<_>>();
        for term in &terms[1..] {
            let term_results = self.search_term(term).iter().map(|p| p.doc_id.clone()).collect::<Vec<_>>();
            results.retain(|d| term_results.contains(d));
        }
        results
    }

    pub fn get_df(&self, term: &str) -> u32 {
        self.terms.get(term).map(|e| e.df).unwrap_or(0)
    }
}

impl NgramIndex {
    pub fn new() -> Self {
        NgramIndex {
            ngrams: Arc::new(DashMap::new()),
        }
    }

    pub fn index_text(&self, text: &str, doc_id: &str, n: usize) {
        let words: Vec<&str> = text.split_whitespace().collect();
        for i in 0..words.len().saturating_sub(n - 1) {
            let ngram = words[i..i + n].join(" ");
            self.ngrams
                .entry(ngram)
                .or_insert_with(Vec::new)
                .push(doc_id.to_string());
        }
    }

    pub fn search(&self, query: &str, n: usize) -> Vec<String> {
        self.ngrams
            .get(query)
            .map(|entry| entry.value().clone())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inverted_index() {
        let idx = InvertedIndex::new();
        idx.index_term("hello", "doc1", 0);
        idx.index_term("world", "doc1", 1);
        assert_eq!(idx.search_term("hello").len(), 1);
    }

    #[test]
    fn test_phrase_search() {
        let idx = InvertedIndex::new();
        idx.index_term("hello", "doc1", 0);
        idx.index_term("world", "doc1", 1);
        idx.index_term("hello", "doc2", 0);

        let results = idx.search_phrase("hello world");
        assert!(results.contains(&"doc1".to_string()));
    }

    #[test]
    fn test_ngram_index() {
        let ngram = NgramIndex::new();
        ngram.index_text("hello world test", "doc1", 2);
        let results = ngram.search("hello world", 2);
        assert!(results.contains(&"doc1".to_string()));
    }
}
