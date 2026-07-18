//! Regex-based secret scanning over source text and files. Detects a set of
//! common high-signal secret formats (AWS access keys, private key blocks,
//! GitHub tokens, generic API-key assignments).

use crate::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SecretFinding {
    pub secret_type: String,
    pub line_number: usize,
    pub snippet: String,
    pub severity: String,
}

struct Pattern {
    secret_type: &'static str,
    severity: &'static str,
    regex: Regex,
}

fn patterns() -> &'static Vec<Pattern> {
    static PATTERNS: OnceLock<Vec<Pattern>> = OnceLock::new();
    PATTERNS.get_or_init(|| {
        vec![
            Pattern {
                secret_type: "aws_access_key_id",
                severity: "critical",
                regex: Regex::new(r"AKIA[0-9A-Z]{16}").unwrap(),
            },
            Pattern {
                secret_type: "private_key_block",
                severity: "critical",
                regex: Regex::new(r"-----BEGIN (RSA |EC |OPENSSH |DSA )?PRIVATE KEY-----").unwrap(),
            },
            Pattern {
                secret_type: "github_token",
                severity: "critical",
                regex: Regex::new(r"gh[pousr]_[A-Za-z0-9]{20,}").unwrap(),
            },
            Pattern {
                secret_type: "slack_token",
                severity: "high",
                regex: Regex::new(r"xox[baprs]-[A-Za-z0-9-]{10,}").unwrap(),
            },
            Pattern {
                secret_type: "generic_api_key_assignment",
                severity: "medium",
                regex: Regex::new(
                    r#"(?i)(api[_-]?key|secret|token|password)\s*[:=]\s*['"][A-Za-z0-9/+_\-]{16,}['"]"#,
                )
                .unwrap(),
            },
        ]
    })
}

pub struct SecretScanner;

impl SecretScanner {
    pub fn new() -> Self {
        Self
    }

    /// Scan the contents of a file on disk for secrets.
    pub async fn scan_file(&self, path: &str) -> Result<Vec<SecretFinding>> {
        let content = tokio::fs::read_to_string(path).await?;
        self.scan_text(&content).await
    }

    /// Scan arbitrary text for secrets, line by line, returning one finding
    /// per match with a redacted snippet.
    pub async fn scan_text(&self, content: &str) -> Result<Vec<SecretFinding>> {
        let mut findings = Vec::new();

        for (idx, line) in content.lines().enumerate() {
            for pattern in patterns() {
                if let Some(m) = pattern.regex.find(line) {
                    findings.push(SecretFinding {
                        secret_type: pattern.secret_type.to_string(),
                        line_number: idx + 1,
                        snippet: redact(m.as_str()),
                        severity: pattern.severity.to_string(),
                    });
                }
            }
        }

        Ok(findings)
    }
}

impl Default for SecretScanner {
    fn default() -> Self {
        Self::new()
    }
}

/// Redact all but the first 4 and last 4 characters of a matched secret so
/// findings can be reported without leaking the full value.
fn redact(matched: &str) -> String {
    let chars: Vec<char> = matched.chars().collect();
    if chars.len() <= 8 {
        return "*".repeat(chars.len());
    }
    let head: String = chars[..4].iter().collect();
    let tail: String = chars[chars.len() - 4..].iter().collect();
    format!("{}...{}", head, tail)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_detects_aws_key() {
        let scanner = SecretScanner::new();
        let content = "let key = \"AKIAIOSFODNN7EXAMPLE\";";
        let findings = scanner.scan_text(content).await.unwrap();
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].secret_type, "aws_access_key_id");
        assert_eq!(findings[0].line_number, 1);
    }

    #[tokio::test]
    async fn test_detects_private_key_block() {
        let scanner = SecretScanner::new();
        let content = "header\n-----BEGIN RSA PRIVATE KEY-----\nMIIEow...\n-----END RSA PRIVATE KEY-----";
        let findings = scanner.scan_text(content).await.unwrap();
        assert!(findings.iter().any(|f| f.secret_type == "private_key_block"));
    }

    #[tokio::test]
    async fn test_detects_generic_api_key_assignment() {
        let scanner = SecretScanner::new();
        let content = r#"api_key: "sk_live_abcdefghijklmnopqrstuv""#;
        let findings = scanner.scan_text(content).await.unwrap();
        assert!(findings.iter().any(|f| f.secret_type == "generic_api_key_assignment"));
    }

    #[tokio::test]
    async fn test_clean_text_has_no_findings() {
        let scanner = SecretScanner::new();
        let content = "fn main() {\n    println!(\"hello world\");\n}";
        let findings = scanner.scan_text(content).await.unwrap();
        assert!(findings.is_empty());
    }

    #[tokio::test]
    async fn test_redaction_hides_middle_of_secret() {
        let scanner = SecretScanner::new();
        let content = "AKIAIOSFODNN7EXAMPLE";
        let findings = scanner.scan_text(content).await.unwrap();
        assert_eq!(findings[0].snippet, "AKIA...MPLE");
    }
}
