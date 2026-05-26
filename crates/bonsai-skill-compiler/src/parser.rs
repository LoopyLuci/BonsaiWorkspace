use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMetadata {
    pub name: String,
    pub description: String,
    pub owner: Option<String>,
    pub version: Option<String>,
    pub license: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub bonsai: Option<BonsaiExtension>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BonsaiExtension {
    pub tool: Option<bool>,
    pub category: Option<String>,
    pub sandbox_tier: Option<String>,
    pub auto_load: Option<bool>,
    pub training_eligible: Option<bool>,
}

pub fn parse_skill_md(skill_dir: &Path) -> Result<(SkillMetadata, String)> {
    let skill_md_path = skill_dir.join("SKILL.md");
    let content = fs::read_to_string(&skill_md_path)
        .with_context(|| format!("Failed to read {skill_md_path:?}"))?;
    let (frontmatter, body) = split_frontmatter(&content)?;
    let metadata: SkillMetadata = serde_yaml::from_str(&frontmatter)
        .context("Invalid YAML frontmatter in SKILL.md")?;
    Ok((metadata, body))
}

/// Parse a SKILL.md directly from a string (for tests and in-memory installs).
pub fn parse_skill_md_str(content: &str) -> Result<(SkillMetadata, String)> {
    let (frontmatter, body) = split_frontmatter(content)?;
    let metadata: SkillMetadata = serde_yaml::from_str(&frontmatter)
        .context("Invalid YAML frontmatter")?;
    Ok((metadata, body))
}

fn split_frontmatter(content: &str) -> Result<(String, String)> {
    let lines: Vec<&str> = content.lines().collect();
    anyhow::ensure!(
        lines.first().unwrap_or(&"").trim() == "---",
        "SKILL.md must start with YAML frontmatter delimited by ---"
    );
    let end_idx = lines[1..]
        .iter()
        .position(|l| l.trim() == "---")
        .ok_or_else(|| anyhow::anyhow!("No closing --- found for frontmatter"))?;
    let frontmatter = lines[1..=end_idx].join("\n");
    let body = lines[end_idx + 2..].join("\n");
    Ok((frontmatter, body))
}
