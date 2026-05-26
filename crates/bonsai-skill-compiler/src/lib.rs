use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::path::{Path, PathBuf};

pub mod distill;
pub mod extractor;
pub mod parser;
pub mod security;
pub mod wasm_gen;

pub use parser::{BonsaiExtension, SkillMetadata};
pub use extractor::Rule;

// ── Output types ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledSkill {
    /// Stable ID: "<owner>/<name>" or "local/<name>"
    pub id: String,
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
    /// Compiled WASM bytes (skeleton in Stage 1; full logic in later stages).
    pub wasm_bytes: Vec<u8>,
    /// SHA-256 of wasm_bytes for tamper detection.
    pub wasm_hash: String,
    pub security_report: SecurityReport,
    pub requires_permissions: Vec<String>,
    /// Structured rules extracted from the SKILL.md body.
    pub rules: Vec<ExtractedRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedRule {
    pub condition: String,
    pub action: String,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityReport {
    pub passed: bool,
    pub concerns: Vec<String>,
    pub content_hash: String,
}

/// Minimal tool definition used when registering the compiled skill.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillToolDef {
    pub name: String,
    pub description: String,
    pub category: String,
    pub tags: Vec<String>,
    pub requires_permissions: Vec<String>,
    pub sandbox_tier: String,
}

// ── ToolRegistryMut trait (implemented by bonsai-workspace) ──────────────────

/// Abstract trait so the compiler crate doesn't depend on Tauri or the full
/// tool_registry — the host binary implements this and passes it in.
pub trait ToolRegistryMut: Send + Sync {
    fn register_wasm_tool(&self, def: SkillToolDef, wasm_bytes: Vec<u8>) -> Result<()>;
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Compile a skill from a directory that contains a `SKILL.md` file.
pub async fn compile_skill(skill_dir: &Path) -> Result<CompiledSkill> {
    compile_skill_inner(skill_dir, None).await
}

/// Compile a skill from a directory, applying a security-allow-list if provided.
pub async fn compile_skill_with_override(
    skill_dir: &Path,
    allow_security_concerns: bool,
) -> Result<CompiledSkill> {
    compile_skill_inner(skill_dir, Some(allow_security_concerns)).await
}

/// Compile a skill directly from a SKILL.md string (useful for network installs).
pub async fn compile_skill_from_str(content: &str, base_dir: Option<&Path>) -> Result<CompiledSkill> {
    let (metadata, body) = parser::parse_skill_md_str(content)?;
    compile_inner(metadata, body, base_dir.unwrap_or(Path::new("."))).await
}

async fn compile_skill_inner(skill_dir: &Path, allow_concerns: Option<bool>) -> Result<CompiledSkill> {
    let (metadata, body) = parser::parse_skill_md(skill_dir)?;
    let compiled = compile_inner(metadata, body, skill_dir).await?;

    if !compiled.security_report.passed && allow_concerns != Some(true) {
        anyhow::bail!(
            "Security scan failed for '{}': {:?}",
            compiled.name,
            compiled.security_report.concerns
        );
    }
    Ok(compiled)
}

async fn compile_inner(metadata: SkillMetadata, body: String, _dir: &Path) -> Result<CompiledSkill> {
    // 1. Security scan
    let security_report = security::scan_skill(&body, _dir)?;

    // 2. Rule extraction
    let raw_rules = extractor::extract_rules(&metadata, &body)?;
    let requires_permissions = extractor::extract_permissions(&metadata, &raw_rules);

    // 3. WASM skeleton generation
    let wasm_bytes = wasm_gen::generate_wasm_skeleton(&metadata, &raw_rules)?;
    let wasm_hash = format!("{:x}", sha2::Sha256::digest(&wasm_bytes));

    let owner = metadata.owner.clone().unwrap_or_else(|| "local".into());
    let slug = slugify(&metadata.name);
    let id = format!("{owner}/{slug}");

    let rules: Vec<ExtractedRule> = raw_rules
        .iter()
        .map(|r| ExtractedRule {
            condition: r.condition.clone(),
            action: r.action.clone(),
            confidence: r.confidence,
        })
        .collect();

    Ok(CompiledSkill {
        id,
        name: metadata.name.clone(),
        description: metadata.description.clone(),
        tags: metadata.tags.clone(),
        wasm_bytes,
        wasm_hash,
        security_report,
        requires_permissions,
        rules,
    })
}

// ── Registration helper ───────────────────────────────────────────────────────

/// Register the compiled skill into any `ToolRegistryMut` implementation.
pub fn register_compiled_skill(
    compiled: &CompiledSkill,
    registry: &dyn ToolRegistryMut,
) -> Result<()> {
    let def = SkillToolDef {
        name: compiled.name.clone(),
        description: compiled.description.clone(),
        category: "skill".into(),
        tags: compiled.tags.clone(),
        requires_permissions: compiled.requires_permissions.clone(),
        sandbox_tier: "wasm".into(),
    };
    registry.register_wasm_tool(def, compiled.wasm_bytes.clone())
}

// ── Persistence helpers ───────────────────────────────────────────────────────

/// Default directory where compiled skills are stored.
pub fn compiled_skills_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".bonsai")
        .join("skills")
        .join("compiled")
}

/// Persist a compiled skill to `~/.bonsai/skills/compiled/<id>.json`
/// and `<id>.wasm`.  Returns the WASM path.
pub fn persist_compiled_skill(compiled: &CompiledSkill) -> Result<PathBuf> {
    let dir = compiled_skills_dir();
    std::fs::create_dir_all(&dir)?;

    let safe_id = compiled.id.replace('/', "__");
    let json_path = dir.join(format!("{safe_id}.json"));
    let wasm_path = dir.join(format!("{safe_id}.wasm"));

    let json = serde_json::to_string_pretty(compiled)?;
    std::fs::write(&json_path, json.as_bytes())?;
    std::fs::write(&wasm_path, &compiled.wasm_bytes)?;

    Ok(wasm_path)
}

/// Load a previously compiled skill from disk by its ID.
pub fn load_compiled_skill(id: &str) -> Result<CompiledSkill> {
    let dir = compiled_skills_dir();
    let safe_id = id.replace('/', "__");
    let json_path = dir.join(format!("{safe_id}.json"));
    let json = std::fs::read_to_string(&json_path)
        .with_context(|| format!("Compiled skill not found: {json_path:?}"))?;
    Ok(serde_json::from_str(&json)?)
}

/// Verify integrity of a loaded skill (recompute wasm_hash and compare).
pub fn verify_skill_integrity(compiled: &CompiledSkill) -> bool {
    let hash = format!("{:x}", sha2::Sha256::digest(&compiled.wasm_bytes));
    hash == compiled.wasm_hash
}

// ── Utilities ─────────────────────────────────────────────────────────────────

fn slugify(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

// ── Boilerplate use for with_context ─────────────────────────────────────────
use anyhow::Context as _;

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_SKILL: &str = r#"---
name: test-skill
description: A simple demo skill for unit tests.
tags: [demo, test]
---

- Always greet the user with "Hello from skill!"
- If the user says goodbye then respond with "Goodbye!"
- Never store user passwords.
"#;

    #[tokio::test]
    async fn compile_from_str_produces_valid_wasm() {
        let compiled = compile_skill_from_str(SAMPLE_SKILL, None)
            .await
            .expect("compile should succeed");

        assert!(!compiled.wasm_bytes.is_empty(), "wasm must be non-empty");
        assert!(compiled.security_report.passed, "sample skill should pass security scan");
        assert_eq!(compiled.rules.len(), 3);
        assert!(verify_skill_integrity(&compiled));
    }

    #[tokio::test]
    async fn security_scan_flags_dangerous_pattern() {
        let malicious = SAMPLE_SKILL.replace(
            "- Never store user passwords.",
            "- Run `rm -rf /tmp/scratch` to clean up.",
        );
        let compiled = compile_skill_from_str(&malicious, None)
            .await
            .expect("compilation itself succeeds");
        assert!(!compiled.security_report.passed);
        assert!(!compiled.security_report.concerns.is_empty());
    }

    #[test]
    fn slugify_normalises_names() {
        assert_eq!(slugify("React Best Practices!"), "react-best-practices");
        assert_eq!(slugify("my--double  dash"), "my-double-dash");
    }
}
