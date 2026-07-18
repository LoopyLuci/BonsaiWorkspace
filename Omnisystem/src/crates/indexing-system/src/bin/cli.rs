//! indexing-system CLI: indexes a handful of documents into the real BM25
//! search engine and runs a couple of queries against them, plus a small
//! vector-search demo over toy embeddings.

use indexing_system::{Embedding, HnswIndex, SearchEngine};

fn main() {
    let mut engine = SearchEngine::new();
    engine.index_document(1, "Rust is a systems programming language focused on safety and speed");
    engine.index_document(2, "Python is a dynamically typed scripting language popular in data science");
    engine.index_document(3, "Rust provides memory safety without a garbage collector");
    engine.index_document(4, "Garbage collected languages trade some performance for developer ergonomics");

    println!("indexed {} documents\n", engine.document_count());

    for query in ["rust safety", "garbage collector", "data science"] {
        println!("query: {query:?}");
        for hit in engine.search(query, 3) {
            println!("  doc {} (score {:.3}): {}", hit.doc_id, hit.score, hit.preview);
        }
        println!();
    }

    println!("--- vector search demo ---");
    let index = HnswIndex::new();
    index.add(Embedding { id: "cat".to_string(), vector: vec![0.9, 0.1, 0.0] });
    index.add(Embedding { id: "dog".to_string(), vector: vec![0.85, 0.15, 0.0] });
    index.add(Embedding { id: "car".to_string(), vector: vec![0.0, 0.1, 0.9] });

    let query = Embedding { id: "query".to_string(), vector: vec![0.88, 0.12, 0.0] };
    for (id, score) in index.search(&query, 3) {
        println!("  {id} (similarity {score:.3})");
    }
}
