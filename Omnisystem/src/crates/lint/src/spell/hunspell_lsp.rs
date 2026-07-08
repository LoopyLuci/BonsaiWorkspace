/// Hunspell-based LSP server for spell checking.
/// This module bridges Hunspell with the LSP protocol for IDE integration.

use anyhow::Result;
use lsp_types::{Diagnostic as LspDiagnostic, DiagnosticSeverity, Position as LspPosition, Range as LspRange};
use std::path::Path;

/// Start a Hunspell-based spell checking LSP server.
pub async fn start_hunspell_lsp_server(port: u16) -> Result<()> {
    // TODO: Implement LSP server using lsp-server crate
    tracing::info!("Hunspell LSP server would start on port {}", port);
    Ok(())
}

/// Convert spell checking results to LSP diagnostics.
pub fn spell_check_to_lsp_diagnostic(
    word: &str,
    line: u32,
    column: u32,
    suggestions: Vec<String>,
) -> LspDiagnostic {
    LspDiagnostic {
        range: LspRange {
            start: LspPosition { line, character: column },
            end: LspPosition { line, character: column + word.len() as u32 },
        },
        severity: Some(DiagnosticSeverity::INFORMATION),
        code: Some(lsp_types::NumberOrString::String("spell-check".to_string())),
        source: Some("hunspell".to_string()),
        message: format!("Spelling error: '{}'. Suggestions: {}", word, suggestions.join(", ")),
        related_information: None,
        tags: None,
        code_description: None,
        data: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spell_check_to_lsp_diagnostic() {
        let diag = spell_check_to_lsp_diagnostic("teh", 0, 0, vec!["the".to_string()]);
        assert_eq!(diag.severity, Some(DiagnosticSeverity::INFORMATION));
        assert!(diag.message.contains("Suggestions: the"));
    }
}
