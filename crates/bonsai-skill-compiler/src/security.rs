use anyhow::Result;
use sha2::Digest;
use std::path::Path;

use crate::SecurityReport;

// Patterns that raise concerns in skill content (checked case-insensitively).
const DANGEROUS_PATTERNS: &[(&str, &str)] = &[
    ("rm -rf", "Destructive recursive file deletion"),
    ("sudo ", "Elevated privilege execution"),
    ("eval(", "Dynamic code evaluation risk"),
    ("exec(", "Arbitrary execution risk"),
    ("subprocess.call", "Subprocess execution"),
    ("os.system", "System command execution"),
    ("curl.*|.*sh", "Remote code execution via pipe"),
    ("wget.*|.*sh", "Remote code execution via pipe"),
    ("__import__", "Dynamic Python import"),
    ("base64.decode", "Obfuscated payload risk"),
    ("process.exit", "Forced process termination"),
];

pub fn scan_skill(body: &str, _skill_dir: &Path) -> Result<SecurityReport> {
    let lower = body.to_lowercase();
    let mut concerns = Vec::new();

    for (pattern, desc) in DANGEROUS_PATTERNS {
        if lower.contains(pattern) {
            concerns.push((*desc).to_string());
        }
    }

    let content_hash = format!("{:x}", sha2::Sha256::digest(body.as_bytes()));

    Ok(SecurityReport {
        passed: concerns.is_empty(),
        concerns,
        content_hash,
    })
}
