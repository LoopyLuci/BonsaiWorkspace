use usee_search::*;
use std::collections::HashMap;

#[test]
fn test_full_search_workflow() {
    let engine = core::SearchEngine::new();
    
    let doc1 = Document {
        id: "doc1".to_string(),
        title: "Rust Programming".to_string(),
        content: "Rust is a systems programming language that empowers developers".to_string(),
        metadata: HashMap::new(),
        score: 1.0,
    };

    let doc2 = Document {
        id: "doc2".to_string(),
        title: "Python Basics".to_string(),
        content: "Python is a high-level programming language for developers".to_string(),
        metadata: HashMap::new(),
        score: 1.0,
    };

    engine.index_document(doc1).unwrap();
    engine.index_document(doc2).unwrap();

    assert_eq!(engine.document_count(), 2);

    let query = Query {
        text: "programming language".to_string(),
        limit: 10,
        offset: 0,
        filters: HashMap::new(),
    };

    let results = engine.search(&query).unwrap();
    assert!(results.total > 0);
}

#[test]
fn test_indexer_with_documents() {
    let indexer = indexer::Indexer::new();
    
    let doc = Document {
        id: "doc1".to_string(),
        title: "Test Document".to_string(),
        content: "This is test content for the search indexer test".to_string(),
        metadata: HashMap::new(),
        score: 1.0,
    };

    assert!(indexer.build_index(&doc).is_ok());
    assert!(indexer.get_term_frequency("doc1").is_some());
}

#[test]
fn test_embeddings_similarity() {
    let store = embeddings::EmbeddingStore::new();
    
    let emb1 = Embedding {
        document_id: "doc1".to_string(),
        vector: vec![1.0, 0.0, 0.0],
        dimensions: 3,
    };

    let emb2 = Embedding {
        document_id: "doc2".to_string(),
        vector: vec![0.0, 1.0, 0.0],
        dimensions: 3,
    };

    store.store_embedding(emb1).unwrap();
    store.store_embedding(emb2).unwrap();

    assert_eq!(store.embedding_count(), 2);
    
    let v1 = vec![1.0, 0.0, 0.0];
    let v2 = vec![0.0, 1.0, 0.0];
    assert_eq!(store.similarity(&v1, &v2), 0.0);
}
