//! CLI that exercises dictionary-based translation end to end.

use omnilingual::{Dictionary, Segmenter, TermEntry, TerminologyStore, Translator, WordAligner};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dict = Dictionary::new("en".to_string(), "es".to_string());
    dict.add_entry("hello".to_string(), vec!["hola".to_string()])?;
    dict.add_entry("world".to_string(), vec!["mundo".to_string()])?;

    let translator = Translator::new();
    translator.register_dictionary("en→es".to_string(), dict)?;

    let source = "Hello world";
    let translated = translator.translate("en", "es", source)?;
    println!("{source} -> {translated}");

    let words = Segmenter::segment_words(source);
    let alignment = WordAligner::align(&words, &Segmenter::segment_words(&translated))?;
    println!("Alignment: {alignment:?}");

    let terminology = TerminologyStore::new();
    terminology.add_term(TermEntry {
        source: "algorithm".to_string(),
        target: "algoritmo".to_string(),
        domain: "computer_science".to_string(),
        frequency: 1,
    })?;
    if let Some(term) = terminology.lookup_term("computer_science", "algorithm") {
        println!("Term lookup: {} -> {}", term.source, term.target);
    }

    Ok(())
}
