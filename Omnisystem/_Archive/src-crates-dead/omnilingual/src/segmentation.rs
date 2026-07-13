pub struct Segmenter;

impl Segmenter {
    pub fn segment_sentences(text: &str) -> Vec<String> {
        text.split(|c| c == '.' || c == '!' || c == '?')
            .filter(|s| !s.trim().is_empty())
            .map(|s| s.trim().to_string())
            .collect()
    }

    pub fn segment_words(text: &str) -> Vec<String> {
        text.split_whitespace()
            .map(|w| w.to_string())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sentence_segmentation() {
        let sentences = Segmenter::segment_sentences("Hello. How are you?");
        assert_eq!(sentences.len(), 2);
    }

    #[test]
    fn test_word_segmentation() {
        let words = Segmenter::segment_words("hello world test");
        assert_eq!(words.len(), 3);
    }
}
