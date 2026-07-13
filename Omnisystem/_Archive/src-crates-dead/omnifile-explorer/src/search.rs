use crate::Result;

pub struct FileSearchEngine;

impl FileSearchEngine {
    pub fn search_by_extension(files: &[String], ext: &str) -> Vec<String> {
        files
            .iter()
            .filter(|f| f.ends_with(ext))
            .cloned()
            .collect()
    }

    pub fn search_by_size_range(files: &[(String, u64)], min: u64, max: u64) -> Vec<String> {
        files
            .iter()
            .filter(|f| f.1 >= min && f.1 <= max)
            .map(|f| f.0.clone())
            .collect()
    }

    pub fn sort_by_name(files: &mut Vec<String>) {
        files.sort();
    }

    pub fn sort_by_size(files: &mut Vec<(String, u64)>) {
        files.sort_by_key(|f| f.1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extension_search() {
        let files = vec![
            "a.txt".to_string(),
            "b.pdf".to_string(),
            "c.txt".to_string(),
        ];
        let results = FileSearchEngine::search_by_extension(&files, ".txt");
        assert_eq!(results.len(), 2);
    }
}
