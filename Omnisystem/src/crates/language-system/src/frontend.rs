//! The LanguageFrontend trait — the single integration point for all languages

use async_trait::async_trait;
use core_ir::LairModule;
use std::path::Path;
use crate::errors::Result;

/// Trait that every language frontend must implement
///
/// This is the single integration point for all languages in the Bonsai Ecosystem.
/// When you want to add a new language, you implement this trait and register it
/// via the language registry.
#[async_trait]
pub trait LanguageFrontend: Send + Sync {
    /// Human-readable language name (e.g., "Titan", "Python", "SQL")
    fn language_name(&self) -> &str;

    /// File extensions handled by this frontend (e.g., ["titan", "ti"])
    fn file_extensions(&self) -> &[&str];

    /// Parse source code into a LAIR module
    ///
    /// This is the primary job of a language frontend: parse source text
    /// and lower it to the Language-Agnostic Intermediate Representation (LAIR).
    /// LAIR provides a typed, effect-tracked, hot-reload-aware representation
    /// that all languages share.
    async fn parse(&self, source: &str, file_path: &Path) -> Result<LairModule>;

    /// (Optional) Provide a Language Server Protocol handler
    ///
    /// Implementing this allows IDE features like hover, completion, diagnostics
    /// to work seamlessly across all languages in the Bonsai ecosystem.
    fn lsp_server(&self) -> Option<Box<dyn LspHandler>> {
        None
    }

    /// (Optional) Provide an interpreter for Tier-1 execution
    ///
    /// Some languages (like Sylva, Python) benefit from an interpreter that can
    /// execute code immediately. Compiled languages may not provide this.
    fn interpreter(&self) -> Option<Box<dyn LairInterpreter>> {
        None
    }

    /// (Optional) Format source code according to language conventions
    async fn format(&self, source: &str) -> Result<String> {
        Ok(source.to_string())
    }

    /// (Optional) Lint source code and return diagnostics
    async fn lint(&self, _source: &str) -> Result<Vec<Diagnostic>> {
        Ok(Vec::new())
    }

    /// (Optional) Provide code completion suggestions
    async fn complete(
        &self,
        _source: &str,
        _line: usize,
        _column: usize,
    ) -> Result<Vec<CompletionItem>> {
        Ok(Vec::new())
    }
}

/// Language Server Protocol handler
///
/// Implementations provide IDE support (hover, definition lookup, etc.)
pub trait LspHandler: Send + Sync {
    /// Handle a hover request at a specific position
    fn hover(&self, source: &str, line: usize, column: usize) -> Option<String>;

    /// Find the definition of a symbol
    fn goto_definition(&self, source: &str, line: usize, column: usize) -> Option<(String, usize, usize)>;

    /// Find all references to a symbol
    fn find_references(&self, source: &str, line: usize, column: usize) -> Vec<(String, usize, usize)>;
}

/// Interpreter for LAIR modules
///
/// Allows executing LAIR code without compiling to native code.
/// Useful for scripting languages and REPL environments.
pub trait LairInterpreter: Send + Sync {
    /// Execute a LAIR module and return a result
    fn execute(&self, module: &LairModule) -> Result<serde_json::Value>;

    /// Execute a single function by name
    fn call_function(
        &self,
        module: &LairModule,
        name: &str,
        args: &[serde_json::Value],
    ) -> Result<serde_json::Value>;
}

/// A diagnostic message (error, warning, info)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Diagnostic {
    pub level: DiagnosticLevel,
    pub line: usize,
    pub column: usize,
    pub message: String,
    pub code: Option<String>,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum DiagnosticLevel {
    Error,
    Warning,
    Info,
    Hint,
}

/// A code completion item
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CompletionItem {
    pub label: String,
    pub kind: CompletionKind,
    pub detail: Option<String>,
    pub documentation: Option<String>,
    pub insert_text: Option<String>,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum CompletionKind {
    Function,
    Variable,
    Keyword,
    Type,
    Module,
    Constant,
}
