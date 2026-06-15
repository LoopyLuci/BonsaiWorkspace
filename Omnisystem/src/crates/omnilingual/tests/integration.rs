use omnilingual::*;

#[test]
fn test_full_translation_workflow() {
    let translator = translator::Translator::new();
    let dict = dictionary::Dictionary::new("en".to_string(), "es".to_string());
    
    translator.register_dictionary("en→es".to_string(), dict).unwrap();
    
    let result = translator.translate("en", "es", "Hello world").unwrap();
    assert!(!result.is_empty());
}

#[test]
fn test_segmentation() {
    let text = "Hello. This is a test. How are you?";
    let sentences = segmentation::Segmenter::segment_sentences(text);
    assert_eq!(sentences.len(), 3);
}

#[test]
fn test_terminology_workflow() {
    let store = terminology::TerminologyStore::new();
    
    let entries = vec![
        ("algorithm", "algoritmo", "computer_science"),
        ("variable", "variable", "programming"),
        ("function", "función", "mathematics"),
    ];
    
    for (src, tgt, domain) in entries {
        let entry = terminology::TermEntry {
            source: src.to_string(),
            target: tgt.to_string(),
            domain: domain.to_string(),
            frequency: 50,
        };
        assert!(store.add_term(entry).is_ok());
    }
    
    assert_eq!(store.term_count(), 3);
    assert!(store.lookup_term("computer_science", "algorithm").is_some());
}
