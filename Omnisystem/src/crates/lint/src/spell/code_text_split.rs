/// Extract natural language text from code (comments, strings, docs) for spell checking.
/// Uses Tree-sitter to identify comment and string nodes.

use anyhow::Result;
use std::path::Path;
use tree_sitter::{Tree, Node};

#[derive(Debug, Clone)]
pub struct TextSpan {
    pub text: String,
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
    pub kind: TextKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextKind {
    Comment,
    StringLiteral,
    Documentation,
}

/// Extract all natural language text spans from a parsed tree.
pub fn extract_text_spans(tree: &Tree, source: &str) -> Vec<TextSpan> {
    let mut spans = Vec::new();
    let root = tree.root_node();
    extract_text_spans_recursive(&root, source, &mut spans);
    spans
}

fn extract_text_spans_recursive(node: &Node, source: &str, spans: &mut Vec<TextSpan>) {
    match node.kind() {
        "comment" | "line_comment" | "block_comment" => {
            if let Ok(text) = node.utf8_text(source.as_bytes()) {
                let start = node.start_point();
                let end = node.end_point();
                spans.push(TextSpan {
                    text: text.to_string(),
                    start_line: start.row as u32,
                    start_col: start.column as u32,
                    end_line: end.row as u32,
                    end_col: end.column as u32,
                    kind: TextKind::Comment,
                });
            }
        }
        "string_literal" | "string" => {
            if let Ok(text) = node.utf8_text(source.as_bytes()) {
                let start = node.start_point();
                let end = node.end_point();
                spans.push(TextSpan {
                    text: text.to_string(),
                    start_line: start.row as u32,
                    start_col: start.column as u32,
                    end_line: end.row as u32,
                    end_col: end.column as u32,
                    kind: TextKind::StringLiteral,
                });
            }
        }
        _ => {
            // Recursively search children
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                extract_text_spans_recursive(&child, source, spans);
            }
        }
    }
}

/// Filter out code identifiers from spell checking.
pub fn is_code_identifier(word: &str) -> bool {
    // Heuristic: if word contains underscores or camelCase patterns, it's likely code
    word.contains('_') || has_camel_case(word)
}

fn has_camel_case(word: &str) -> bool {
    let mut has_lower = false;
    let mut has_upper = false;
    for ch in word.chars() {
        if ch.is_lowercase() {
            has_lower = true;
        } else if ch.is_uppercase() {
            has_upper = true;
        }
        if has_lower && has_upper {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_code_identifier() {
        assert!(is_code_identifier("myVariable"));
        assert!(is_code_identifier("snake_case"));
        assert!(is_code_identifier("CamelCase"));
        assert!(!is_code_identifier("the"));
        assert!(!is_code_identifier("hello"));
    }
}
