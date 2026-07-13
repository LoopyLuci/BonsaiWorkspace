/// Hunspell-based LSP spell checking server.
/// Implements the Language Server Protocol (LSP) for spell and grammar checking.

use lsp_server::{Connection, Message, Request, Response};
use lsp_types::*;
use serde_json::json;
use std::sync::Arc;
use parking_lot::Mutex;
use anyhow::Result;

/// Hunspell spell checker server state.
pub struct HunspellLspServer {
    connection: Arc<Mutex<Option<Connection>>>,
    documents: Arc<Mutex<std::collections::HashMap<String, String>>>,
}

impl HunspellLspServer {
    pub fn new() -> Self {
        Self {
            connection: Arc::new(Mutex::new(None)),
            documents: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }

    /// Start the LSP server on a given port.
    pub async fn run(&self, port: u16) -> Result<()> {
        // TODO: Implement full LSP server lifecycle
        // This is a placeholder showing the structure

        tracing::info!("Hunspell LSP server would start on port {}", port);

        // Initialize diagnostics for opened documents
        self.on_initialize().await?;

        // Listen for file changes and emit diagnostics
        self.on_did_open().await?;
        self.on_did_change().await?;

        Ok(())
    }

    async fn on_initialize(&self) -> Result<()> {
        // Server initialization
        tracing::info!("Hunspell LSP server initialized");
        Ok(())
    }

    async fn on_did_open(&self) -> Result<()> {
        // When a document is opened, scan it for spelling errors
        tracing::info!("Document opened");
        Ok(())
    }

    async fn on_did_change(&self) -> Result<()> {
        // When a document changes, re-scan for errors
        tracing::info!("Document changed");
        Ok(())
    }

    /// Check text for spelling errors.
    pub async fn check_text(&self, text: &str, language: &str) -> Result<Vec<Diagnostic>> {
        let mut diagnostics = Vec::new();

        // Split text into words and check each
        for (line_no, line) in text.lines().enumerate() {
            let words = line.split_whitespace();
            let mut col_offset = 0;

            for word in words {
                // Simple spell check: in production, use hunspell-rs crate
                if is_misspelled(word, language).await {
                    let start_col = line.find(word).unwrap_or(col_offset) as u32;
                    let end_col = start_col + word.len() as u32;

                    diagnostics.push(Diagnostic {
                        range: Range {
                            start: Position {
                                line: line_no as u32,
                                character: start_col,
                            },
                            end: Position {
                                line: line_no as u32,
                                character: end_col,
                            },
                        },
                        severity: Some(DiagnosticSeverity::INFORMATION),
                        code: Some(NumberOrString::String("spell-check".to_string())),
                        source: Some("hunspell".to_string()),
                        message: format!("Spelling: '{}'. Suggestions: {}", word, suggest_corrections(word).join(", ")),
                        related_information: None,
                        tags: None,
                        code_description: None,
                        data: None,
                    });
                }

                col_offset = start_col as usize + word.len();
            }
        }

        Ok(diagnostics)
    }
}

impl Default for HunspellLspServer {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if a word is misspelled in the given language.
async fn is_misspelled(word: &str, _language: &str) -> bool {
    // TODO: Integrate with hunspell-rs for actual checking
    // For now, use a simple heuristic

    // Ignore:
    // - Code identifiers (camelCase, snake_case)
    // - Numbers
    // - Very short words
    if word.len() < 3 || word.contains('_') || word.chars().all(|c| c.is_numeric()) {
        return false;
    }

    // Common programming words to ignore
    let ignore_list = vec![
        "async", "await", "fn", "let", "mut", "pub", "struct", "impl", "trait",
        "where", "for", "in", "match", "if", "else", "loop", "while", "return",
        "self", "super", "crate", "use", "mod", "const", "type", "enum",
    ];

    if ignore_list.contains(&word.to_lowercase().as_str()) {
        return false;
    }

    // Placeholder: in production, check against dictionary
    false
}

/// Get spelling suggestions for a misspelled word.
fn suggest_corrections(word: &str) -> Vec<String> {
    // TODO: Integrate with hunspell-rs for actual suggestions
    // For now, return placeholder suggestions

    let suggestions = match word {
        "teh" => vec!["the"],
        "recieve" => vec!["receive"],
        "occured" => vec!["occurred"],
        "seperate" => vec!["separate"],
        _ => vec![],
    };

    suggestions.iter().map(|s| s.to_string()).collect()
}

/// LSP Diagnostic builder for spell checking.
pub fn spell_check_diagnostic(
    word: &str,
    line: u32,
    start_col: u32,
    end_col: u32,
    suggestions: Vec<String>,
) -> Diagnostic {
    Diagnostic {
        range: Range {
            start: Position { line, character: start_col },
            end: Position { line, character: end_col },
        },
        severity: Some(DiagnosticSeverity::HINT),
        code: Some(NumberOrString::String("spell-check".to_string())),
        source: Some("hunspell".to_string()),
        message: if suggestions.is_empty() {
            format!("Spelling: '{}'", word)
        } else {
            format!("Spelling: '{}'. Did you mean: {}?", word, suggestions.join(" or "))
        },
        related_information: None,
        tags: None,
        code_description: None,
        data: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_spell_checker_creation() {
        let server = HunspellLspServer::new();
        assert_eq!(server.documents.lock().len(), 0);
    }

    #[tokio::test]
    async fn test_check_text() -> Result<()> {
        let server = HunspellLspServer::new();
        let diagnostics = server.check_text("This is a test", "en").await?;
        // Should have no errors for correct text
        assert!(diagnostics.is_empty());
        Ok(())
    }

    #[test]
    fn test_spell_check_diagnostic() {
        let diag = spell_check_diagnostic("teh", 0, 0, 3, vec!["the".to_string()]);
        assert_eq!(diag.source, Some("hunspell".to_string()));
        assert!(diag.message.contains("teh"));
    }
}
