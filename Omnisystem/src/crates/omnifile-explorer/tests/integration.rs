use omnifile_explorer::*;

#[test]
fn test_full_explorer_workflow() {
    let explorer = explorer::Explorer::new();
    
    explorer.create_file("/file1.txt".to_string(), 1024).unwrap();
    explorer.create_file("/file2.pdf".to_string(), 5000).unwrap();
    assert_eq!(explorer.file_count(), 2);
    
    explorer.navigate("/home".to_string());
    assert_eq!(explorer.get_current_path(), "/home");
    
    explorer.add_bookmark("home".to_string(), "/home".to_string()).unwrap();
    assert_eq!(explorer.bookmark_count(), 1);
}

#[test]
fn test_indexing_and_search() {
    let indexer = indexer::FileIndexer::new();
    
    let entries = vec![
        indexer::IndexEntry {
            path: "/doc1.txt".to_string(),
            name: "doc1.txt".to_string(),
            size: 2000,
            tags: vec!["document".to_string()],
        },
        indexer::IndexEntry {
            path: "/img1.png".to_string(),
            name: "img1.png".to_string(),
            size: 50000,
            tags: vec!["image".to_string()],
        },
    ];
    
    for entry in entries {
        indexer.index_file(entry).unwrap();
    }
    
    let doc_results = indexer.search_by_name("doc");
    assert_eq!(doc_results.len(), 1);
    
    let size_results = indexer.search_by_size(1000, 3000);
    assert_eq!(size_results.len(), 1);
}
