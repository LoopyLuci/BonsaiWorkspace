use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use dashmap::DashMap;
use parking_lot::RwLock;
use std::sync::Arc;
use tree_sitter::{Language, Parser, Tree};

/// Detects the programming language from a file path.
pub fn detect_language(path: &Path) -> String {
    match path.extension().and_then(|s| s.to_str()) {
        Some("rs") => "rust".to_string(),
        Some("py") => "python".to_string(),
        Some("ts" | "tsx") => "typescript".to_string(),
        Some("js" | "jsx") => "javascript".to_string(),
        Some("go") => "go".to_string(),
        Some("java") => "java".to_string(),
        Some("cs") => "csharp".to_string(),
        Some("cpp" | "cc" | "cxx" | "h" | "hpp") => "cpp".to_string(),
        Some("titan") => "titan".to_string(),
        Some("aether") => "aether".to_string(),
        Some("sylva") => "sylva".to_string(),
        Some("axiom") => "axiom".to_string(),
        _ => "plaintext".to_string(),
    }
}

/// Salsa-like database for incremental parsing and symbol indexing.
pub struct LintDb {
    root: PathBuf,
    /// Cache of parsed trees (path -> tree).
    parse_cache: Arc<DashMap<PathBuf, ParsedFile>>,
    /// Cache of previous file hashes (for change detection).
    file_hashes: Arc<RwLock<HashMap<PathBuf, u64>>>,
}

#[derive(Clone, Debug)]
pub struct ParsedFile {
    pub path: PathBuf,
    pub language: String,
    pub tree: Tree,
    pub source: String,
    pub hash: u64,
}

impl LintDb {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            parse_cache: Arc::new(DashMap::new()),
            file_hashes: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Parse a file and cache the result.
    pub fn parse_file(&self, path: &Path) -> Result<ParsedFile> {
        // Check cache
        if let Some(cached) = self.parse_cache.get(path) {
            let current_hash = self.file_hash(path)?;
            if cached.hash == current_hash {
                return Ok(cached.clone());
            }
        }

        let source = std::fs::read_to_string(path)?;
        let hash = blake3::hash(source.as_bytes()).as_u64().into();

        // Detect language and get parser
        let language = detect_language(path);
        let mut parser = self.get_parser(&language)?;

        // Parse
        let tree = parser.parse(&source, None)
            .ok_or_else(|| anyhow!("Failed to parse {:?}", path))?;

        let parsed = ParsedFile {
            path: path.to_path_buf(),
            language,
            tree,
            source,
            hash,
        };

        self.parse_cache.insert(path.to_path_buf(), parsed.clone());
        self.file_hashes.write().insert(path.to_path_buf(), hash);

        Ok(parsed)
    }

    /// Compute the blast radius (all affected files) of a change to one file.
    pub fn compute_blast_radius(&self, changed_file: &Path) -> Result<Vec<PathBuf>> {
        // In a real implementation, this would track import/reference relationships.
        // For now, return just the changed file.
        Ok(vec![changed_file.to_path_buf()])
    }

    fn get_parser(&self, language: &str) -> Result<Parser> {
        let mut parser = Parser::new();
        let lang = self.get_language(language)?;
        parser.set_language(lang)?;
        Ok(parser)
    }

    fn get_language(&self, language: &str) -> Result<Language> {
        match language {
            "rust" => Ok(tree_sitter_rust::LANGUAGE),
            "python" => Ok(tree_sitter_python::LANGUAGE),
            "typescript" => Ok(tree_sitter_javascript::LANGUAGE), // Note: both TS and JS use same grammar
            "javascript" => Ok(tree_sitter_javascript::LANGUAGE),
            "go" => Ok(tree_sitter_go::LANGUAGE),
            "java" => Ok(tree_sitter_java::LANGUAGE),
            "csharp" => Ok(tree_sitter_c_sharp::LANGUAGE),
            "cpp" => Ok(tree_sitter_cpp::LANGUAGE),
            #[cfg(feature = "omnisystem")]
            "titan" => {
                // Load Titan grammar (will be provided by bonsai-lint-treesitter-titan)
                todo!("Load Titan grammar")
            }
            #[cfg(feature = "omnisystem")]
            "aether" => todo!("Load Aether grammar"),
            #[cfg(feature = "omnisystem")]
            "sylva" => todo!("Load Sylva grammar"),
            #[cfg(feature = "omnisystem")]
            "axiom" => todo!("Load Axiom grammar"),
            _ => Err(anyhow!("Unsupported language: {}", language)),
        }
    }

    fn file_hash(&self, path: &Path) -> Result<u64> {
        let source = std::fs::read_to_string(path)?;
        Ok(blake3::hash(source.as_bytes()).as_u64().into())
    }

    /// Clear the cache (useful for tests).
    pub fn clear_cache(&self) {
        self.parse_cache.clear();
        self.file_hashes.write().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_detect_language() {
        assert_eq!(detect_language(Path::new("main.rs")), "rust");
        assert_eq!(detect_language(Path::new("script.py")), "python");
        assert_eq!(detect_language(Path::new("app.ts")), "typescript");
        assert_eq!(detect_language(Path::new("unknown.xyz")), "plaintext");
    }

    #[test]
    fn test_parse_caching() -> Result<()> {
        let temp = TempDir::new()?;
        let file_path = temp.path().join("test.rs");
        fs::write(&file_path, "fn main() {}")?;

        let db = LintDb::new(temp.path().to_path_buf());

        // First parse
        let parsed1 = db.parse_file(&file_path)?;
        assert_eq!(parsed1.language, "rust");

        // Second parse (should be cached)
        let parsed2 = db.parse_file(&file_path)?;
        assert_eq!(parsed1.hash, parsed2.hash);

        Ok(())
    }
}
