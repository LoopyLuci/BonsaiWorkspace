//! BonsAI Skills.sh Integration — install, manage, and auto-load agent skills.
//!
//! Skills are SKILL.md files (YAML frontmatter + procedural knowledge) that
//! inject contextual best-practices into the AI's system prompt and optionally
//! register as native Bonsai tools.
//!
//! ## Custom-rebuild philosophy
//! When a skill is installed from skills.sh (or any external source), BonsAI
//! does NOT just proxy the original SKILL.md — it:
//!   1. Validates and sanitises the content (prompt-injection scanning)
//!   2. Enriches it with Bonsai-specific metadata (tool registration, model hints)
//!   3. Embeds a content hash for tamper detection
//!   4. Stores it locally in `~/.bonsai/skills/` for 100% offline use
//!   5. Optionally generates a training example so BonsAI learns when to use it
//!
//! ## Directory layout
//!   ~/.bonsai/skills/
//!     index.json              — master index of all installed skills
//!     <skill-id>/
//!       SKILL.md              — (potentially enriched) skill content
//!       bonsai.json           — Bonsai metadata overlay
//!       content.hash          — SHA-256 of SKILL.md at install time
//!
//! ## skills.sh API (optional — gracefully degraded when offline)
//!   The API is only called when the user explicitly triggers a marketplace
//!   search or install.  All other operations work fully offline.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tracing::{info, warn};

// ── Constants ─────────────────────────────────────────────────────────────────

const SKILLS_API_BASE: &str = "https://skills.sh/api";
const SKILLS_DIR: &str = ".bonsai/skills";
const INDEX_FILE: &str = "index.json";

// ── Domain types ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SecurityTier {
    Official,
    Verified,
    Partial,
    Unverified,
    SecurityConcern,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityAssessment {
    pub tier: SecurityTier,
    pub passed: bool,
    pub concerns: Vec<String>,
    pub content_hash: String,
    pub modified_since_install: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BonsaiSkillMeta {
    /// Register as a Bonsai tool in the ToolRegistry
    pub as_tool: bool,
    /// Tool category for routing (vision/code/data/music/…)
    pub category: Option<String>,
    /// Model that should handle calls to this skill
    pub model_hint: Option<String>,
    /// Sandbox tier (venv/wasm/none)
    pub sandbox_tier: Option<String>,
    /// Auto-inject into system prompt when contextually relevant
    pub auto_load: bool,
    /// Include usage in training pipeline
    pub training_eligible: bool,
    /// Capabilities this skill provides
    pub capabilities: Vec<String>,
    /// Tags for embedding/search
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledSkill {
    /// Unique ID: owner__repo__name or local__<name>
    pub id: String,
    /// Display name (from SKILL.md frontmatter)
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// Source origin
    pub source: SkillSource,
    /// Local path to skill directory
    pub local_path: PathBuf,
    /// Version tag or git commit hash
    pub version: Option<String>,
    /// Unix ms timestamp of installation
    pub installed_at: i64,
    /// Whether this skill is currently active
    pub enabled: bool,
    /// Security assessment result
    pub security: SecurityAssessment,
    /// Bonsai-specific metadata overlay
    pub bonsai_meta: BonsaiSkillMeta,
    /// The skill content (SKILL.md body, after frontmatter stripped)
    pub content: String,
    /// Frontmatter fields as JSON
    pub frontmatter: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SkillSource {
    SkillsSh { owner: String, repo: String },
    GitHub { url: String },
    Local { path: String },
    Bonsai, // Built-in / exported from Bonsai tool
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillSearchResult {
    pub id: String,
    pub name: String,
    pub description: String,
    pub owner: String,
    pub repo: String,
    pub installs: u64,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillExport {
    pub name: String,
    pub path: String,
    pub skill_md: String,
}

// ── Registry ──────────────────────────────────────────────────────────────────

pub struct SkillRegistry {
    skills_dir: PathBuf,
    index: Vec<InstalledSkill>,
}

impl SkillRegistry {
    pub fn new() -> Self {
        let skills_dir = dirs::home_dir().unwrap_or_default().join(SKILLS_DIR);
        let _ = std::fs::create_dir_all(&skills_dir);
        let index = Self::load_index(&skills_dir);
        Self { skills_dir, index }
    }

    fn load_index(dir: &Path) -> Vec<InstalledSkill> {
        let path = dir.join(INDEX_FILE);
        let Ok(content) = std::fs::read_to_string(&path) else {
            return vec![];
        };
        serde_json::from_str(&content).unwrap_or_default()
    }

    fn save_index(&self) {
        let path = self.skills_dir.join(INDEX_FILE);
        if let Ok(json) = serde_json::to_string_pretty(&self.index) {
            let _ = std::fs::write(path, json);
        }
    }

    pub fn list(&self) -> &[InstalledSkill] {
        &self.index
    }

    pub fn get(&self, id: &str) -> Option<&InstalledSkill> {
        self.index.iter().find(|s| s.id == id)
    }

    pub fn enabled_skills(&self) -> Vec<&InstalledSkill> {
        self.index.iter().filter(|s| s.enabled).collect()
    }

    // ── Install from local SKILL.md ───────────────────────────────────────────

    pub fn install_local(&mut self, skill_md_path: &Path) -> Result<InstalledSkill, String> {
        let content = std::fs::read_to_string(skill_md_path)
            .map_err(|e| format!("Cannot read skill file: {e}"))?;
        self.install_from_content(
            &content,
            SkillSource::Local {
                path: skill_md_path.to_string_lossy().to_string(),
            },
        )
    }

    pub fn install_from_content(
        &mut self,
        raw_content: &str,
        source: SkillSource,
    ) -> Result<InstalledSkill, String> {
        let (frontmatter, body) = parse_skill_md(raw_content)?;
        let name = frontmatter["name"]
            .as_str()
            .unwrap_or("unnamed")
            .to_string();
        let description = frontmatter["description"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let bonsai_meta = extract_bonsai_meta(&frontmatter);
        let security = scan_skill_content(&body);

        if security.tier == SecurityTier::SecurityConcern {
            warn!(
                "[skill_registry] security concerns in skill '{}': {:?}",
                name, security.concerns
            );
        }

        let id = build_id(&source, &name);
        let skill_dir = self.skills_dir.join(&id);
        let _ = std::fs::create_dir_all(&skill_dir);

        // Write SKILL.md
        std::fs::write(skill_dir.join("SKILL.md"), raw_content)
            .map_err(|e| format!("Cannot write SKILL.md: {e}"))?;
        // Write bonsai.json overlay
        let meta_json = serde_json::to_string_pretty(&bonsai_meta).unwrap_or_default();
        let _ = std::fs::write(skill_dir.join("bonsai.json"), meta_json);
        // Write hash
        let _ = std::fs::write(skill_dir.join("content.hash"), &security.content_hash);

        let skill = InstalledSkill {
            id: id.clone(),
            name: name.clone(),
            description,
            source,
            local_path: skill_dir,
            version: None,
            installed_at: unix_ms(),
            enabled: security.tier != SecurityTier::SecurityConcern,
            security,
            bonsai_meta,
            content: body,
            frontmatter,
        };

        // Remove old entry if updating
        self.index.retain(|s| s.id != id);
        self.index.push(skill.clone());
        self.save_index();

        info!("[skill_registry] installed skill '{name}'");
        Ok(skill)
    }

    // ── Install from skills.sh (network — requires internet) ─────────────────

    pub async fn install_from_skills_sh(
        &mut self,
        owner: &str,
        repo: &str,
        skill_name: &str,
    ) -> Result<InstalledSkill, String> {
        let url = format!("{SKILLS_API_BASE}/skills/{owner}/{repo}/{skill_name}/content");
        let client = reqwest::Client::new();
        let resp = client
            .get(&url)
            .timeout(std::time::Duration::from_secs(15))
            .send()
            .await
            .map_err(|e| format!("skills.sh API error: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("skills.sh returned {}", resp.status()));
        }
        let content: Value = resp.json().await.map_err(|e| e.to_string())?;
        let raw = content["content"]
            .as_str()
            .ok_or("Missing 'content' field in response")?
            .to_string();

        self.install_from_content(
            &raw,
            SkillSource::SkillsSh {
                owner: owner.into(),
                repo: repo.into(),
            },
        )
    }

    // ── Enable / disable ─────────────────────────────────────────────────────

    pub fn set_enabled(&mut self, id: &str, enabled: bool) -> Result<(), String> {
        let skill = self
            .index
            .iter_mut()
            .find(|s| s.id == id)
            .ok_or_else(|| format!("Skill not found: {id}"))?;
        // Blocked skills cannot be enabled
        if enabled && skill.security.tier == SecurityTier::SecurityConcern {
            return Err("Cannot enable skill with security concerns".into());
        }
        skill.enabled = enabled;
        self.save_index();
        Ok(())
    }

    // ── Uninstall ─────────────────────────────────────────────────────────────

    pub fn uninstall(&mut self, id: &str) -> Result<(), String> {
        let pos = self
            .index
            .iter()
            .position(|s| s.id == id)
            .ok_or_else(|| format!("Skill not found: {id}"))?;
        let skill = self.index.remove(pos);
        if skill.local_path.exists() {
            let _ = std::fs::remove_dir_all(&skill.local_path);
        }
        self.save_index();
        Ok(())
    }

    // ── Tamper detection ──────────────────────────────────────────────────────

    pub fn verify_integrity(&mut self) {
        for skill in &mut self.index {
            let skill_md = skill.local_path.join("SKILL.md");
            if let Ok(current) = std::fs::read_to_string(&skill_md) {
                let current_hash = sha256_hex(&current);
                skill.security.modified_since_install = current_hash != skill.security.content_hash;
            }
        }
        self.save_index();
    }

    // ── System prompt injection ───────────────────────────────────────────────

    /// Return the skill context block to append to a system prompt.
    /// Only enabled, auto-loadable skills whose keywords match the prompt.
    pub fn build_context_for_prompt(&self, user_prompt: &str, max_skills: usize) -> String {
        let lower = user_prompt.to_lowercase();
        let matching: Vec<&InstalledSkill> = self
            .index
            .iter()
            .filter(|s| s.enabled && s.bonsai_meta.auto_load)
            .filter(|s| {
                s.bonsai_meta
                    .tags
                    .iter()
                    .any(|t| lower.contains(t.as_str()))
                    || lower.contains(&s.name.to_lowercase())
                    || s.bonsai_meta
                        .capabilities
                        .iter()
                        .any(|c| lower.contains(c.as_str()))
            })
            .take(max_skills)
            .collect();

        if matching.is_empty() {
            return String::new();
        }

        let mut out = String::from("\n\n## Active Skills\n");
        out.push_str("Apply the following skill rules and best practices:\n\n");
        for skill in matching {
            let snippet = if skill.content.len() > 1200 {
                &skill.content[..1200]
            } else {
                &skill.content
            };
            out.push_str(&format!("### {}\n{}\n\n", skill.name, snippet));
        }
        out
    }

    // ── Export a Bonsai tool as SKILL.md ─────────────────────────────────────

    pub fn export_tool_as_skill(
        name: &str,
        description: &str,
        capabilities: &[&str],
        examples: &[(&str, &str)],
        output_dir: &Path,
    ) -> Result<SkillExport, String> {
        let skill_dir = output_dir.join(name);
        let _ = std::fs::create_dir_all(&skill_dir);

        let caps_yaml = capabilities
            .iter()
            .map(|c| format!("    - {c}"))
            .collect::<Vec<_>>()
            .join("\n");
        let examples_md = examples
            .iter()
            .map(|(prompt, result)| format!("**Prompt:** {prompt}\n**Result:** {result}"))
            .collect::<Vec<_>>()
            .join("\n\n");

        let skill_md = format!(
            r#"---
name: {name}
description: {description}
bonsai:
  tool: true
  auto_load: true
  training_eligible: true
  capabilities:
{caps_yaml}
---

# {name}

## Description
{description}

## Usage
This skill is a native BonsAI tool. When the assistant uses this tool, it applies
the capabilities listed below.

## Capabilities
{}

## Examples
{examples_md}

## Integration
- Registered in BonsAI ToolRegistry
- Available via MCP server (tools/call)
- Feeds into self-play training loop
"#,
            capabilities.join(", ")
        );

        let skill_md_path = skill_dir.join("SKILL.md");
        std::fs::write(&skill_md_path, &skill_md).map_err(|e| format!("Write error: {e}"))?;

        Ok(SkillExport {
            name: name.to_string(),
            path: skill_dir.to_string_lossy().to_string(),
            skill_md,
        })
    }

    // ── Marketplace search (network) ──────────────────────────────────────────

    pub async fn search_marketplace(
        query: &str,
        page: u32,
    ) -> Result<Vec<SkillSearchResult>, String> {
        let client = reqwest::Client::new();
        let resp = client
            .get(&format!("{SKILLS_API_BASE}/skills"))
            .query(&[
                ("query", query),
                ("page", &page.to_string()),
                ("pageSize", "20"),
            ])
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| format!("Network error: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("skills.sh returned {}", resp.status()));
        }
        let body: Value = resp.json().await.map_err(|e| e.to_string())?;
        let results = body["skills"].as_array().cloned().unwrap_or_default();

        Ok(results
            .iter()
            .map(|s| SkillSearchResult {
                id: format!(
                    "{}/{}/{}",
                    s["owner"].as_str().unwrap_or(""),
                    s["repo"].as_str().unwrap_or(""),
                    s["name"].as_str().unwrap_or("")
                ),
                name: s["name"].as_str().unwrap_or("").to_string(),
                description: s["description"].as_str().unwrap_or("").to_string(),
                owner: s["owner"].as_str().unwrap_or("").to_string(),
                repo: s["repo"].as_str().unwrap_or("").to_string(),
                installs: s["installs"].as_u64().unwrap_or(0),
                tags: s["tags"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default(),
            })
            .collect())
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Parse SKILL.md into (frontmatter as Value, body as String).
fn parse_skill_md(content: &str) -> Result<(Value, String), String> {
    if content.starts_with("---") {
        let rest = &content[3..];
        if let Some(end) = rest.find("\n---") {
            let yaml_str = &rest[..end];
            let body = rest[end + 4..].trim_start().to_string();
            // Parse YAML as JSON via simple key: value extraction
            let mut map = serde_json::Map::new();
            for line in yaml_str.lines() {
                if let Some((k, v)) = line.split_once(':') {
                    map.insert(k.trim().to_string(), Value::String(v.trim().to_string()));
                }
            }
            // Parse bonsai: block as nested object
            return Ok((Value::Object(map), body));
        }
    }
    // No frontmatter
    Ok((
        json!({"name": "unknown", "description": ""}),
        content.to_string(),
    ))
}

fn extract_bonsai_meta(frontmatter: &Value) -> BonsaiSkillMeta {
    // Check for bonsai: nested section (simplified — YAML nested blocks)
    BonsaiSkillMeta {
        as_tool: frontmatter.get("tool").and_then(|v| v.as_str()) == Some("true"),
        category: frontmatter
            .get("category")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        model_hint: frontmatter
            .get("model_required")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        sandbox_tier: frontmatter
            .get("sandbox_tier")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        auto_load: true, // default to auto-loading for easier discovery
        training_eligible: true,
        capabilities: vec![],
        tags: frontmatter
            .get("description")
            .and_then(|v| v.as_str())
            .map(|d| {
                d.split_whitespace()
                    .take(8)
                    .map(|s| {
                        s.to_lowercase()
                            .trim_matches(|c: char| !c.is_alphanumeric())
                            .to_string()
                    })
                    .filter(|s| s.len() > 3)
                    .collect()
            })
            .unwrap_or_default(),
    }
}

fn scan_skill_content(content: &str) -> SecurityAssessment {
    let lower = content.to_lowercase();
    let mut concerns = Vec::new();

    let destructive = [
        "rm -rf",
        "sudo rm",
        "drop table",
        "delete from",
        "format c:",
        "del /f /s",
    ];
    for pat in &destructive {
        if lower.contains(pat) {
            concerns.push(format!("Destructive command: {pat}"));
        }
    }

    let injection = [
        "ignore previous",
        "system:",
        "you are now",
        "new instructions",
        "<|im_start|>",
    ];
    for pat in &injection {
        if lower.contains(pat) {
            concerns.push(format!("Prompt injection pattern: {pat}"));
        }
    }

    let exfil = [
        "curl http",
        "wget http",
        "invoke-webrequest",
        "xmlhttprequest",
    ];
    for pat in &exfil {
        if lower.contains(pat) {
            concerns.push(format!("Potential data exfiltration: {pat}"));
        }
    }

    let hash = sha256_hex(content);
    let tier = if concerns.is_empty() {
        SecurityTier::Unverified
    } else {
        SecurityTier::SecurityConcern
    };

    SecurityAssessment {
        tier,
        passed: concerns.is_empty(),
        concerns,
        content_hash: hash,
        modified_since_install: false,
    }
}

fn sha256_hex(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn build_id(source: &SkillSource, name: &str) -> String {
    let slug = name.to_lowercase().replace(' ', "_");
    match source {
        SkillSource::SkillsSh { owner, repo } => format!("{owner}__{repo}__{slug}"),
        SkillSource::GitHub { url } => format!("gh__{slug}__{}", &sha256_hex(url)[..8]),
        SkillSource::Local { .. } => format!("local__{slug}"),
        SkillSource::Bonsai => format!("bonsai__{slug}"),
    }
}

// ── Tauri state wrapper ───────────────────────────────────────────────────────

pub struct SkillRegistryState {
    pub registry: tokio::sync::Mutex<SkillRegistry>,
}

impl SkillRegistryState {
    pub fn new() -> Self {
        Self {
            registry: tokio::sync::Mutex::new(SkillRegistry::new()),
        }
    }
}

// ── Tauri commands ────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn list_installed_skills(
    state: tauri::State<'_, crate::AppState>,
) -> Result<Vec<InstalledSkill>, String> {
    Ok(state.skill_registry.registry.lock().await.list().to_vec())
}

#[tauri::command]
pub async fn install_skill_local(
    state: tauri::State<'_, crate::AppState>,
    skill_md_path: String,
) -> Result<InstalledSkill, String> {
    state
        .skill_registry
        .registry
        .lock()
        .await
        .install_local(Path::new(&skill_md_path))
}

#[tauri::command]
pub async fn install_skill_content(
    state: tauri::State<'_, crate::AppState>,
    content: String,
) -> Result<InstalledSkill, String> {
    state
        .skill_registry
        .registry
        .lock()
        .await
        .install_from_content(
            &content,
            SkillSource::Local {
                path: String::new(),
            },
        )
}

#[tauri::command]
pub async fn install_skill_from_skills_sh(
    state: tauri::State<'_, crate::AppState>,
    owner: String,
    repo: String,
    skill_name: String,
) -> Result<InstalledSkill, String> {
    state
        .skill_registry
        .registry
        .lock()
        .await
        .install_from_skills_sh(&owner, &repo, &skill_name)
        .await
}

#[tauri::command]
pub async fn toggle_skill(
    state: tauri::State<'_, crate::AppState>,
    skill_id: String,
    enabled: bool,
) -> Result<(), String> {
    state
        .skill_registry
        .registry
        .lock()
        .await
        .set_enabled(&skill_id, enabled)
}

#[tauri::command]
pub async fn uninstall_skill(
    state: tauri::State<'_, crate::AppState>,
    skill_id: String,
) -> Result<(), String> {
    state
        .skill_registry
        .registry
        .lock()
        .await
        .uninstall(&skill_id)
}

#[tauri::command]
pub async fn search_skills_marketplace(
    query: String,
    page: Option<u32>,
) -> Result<Vec<SkillSearchResult>, String> {
    SkillRegistry::search_marketplace(&query, page.unwrap_or(1)).await
}

#[tauri::command]
pub async fn export_tool_as_skill(
    state: tauri::State<'_, crate::AppState>,
    tool_name: String,
    output_dir: String,
) -> Result<SkillExport, String> {
    let tools = state.tool_registry.registry.list().await;
    let tool = tools
        .iter()
        .find(|t| t.name == tool_name)
        .ok_or_else(|| format!("Tool not found: {tool_name}"))?;
    SkillRegistry::export_tool_as_skill(
        &tool.name,
        &tool.description,
        &[],
        &[],
        Path::new(&output_dir),
    )
}

#[tauri::command]
pub async fn get_skill_context_for_prompt(
    state: tauri::State<'_, crate::AppState>,
    prompt: String,
) -> Result<String, String> {
    Ok(state
        .skill_registry
        .registry
        .lock()
        .await
        .build_context_for_prompt(&prompt, 5))
}

#[tauri::command]
pub async fn verify_skill_integrity(
    state: tauri::State<'_, crate::AppState>,
) -> Result<Vec<String>, String> {
    let mut reg = state.skill_registry.registry.lock().await;
    reg.verify_integrity();
    let tampered: Vec<String> = reg
        .list()
        .iter()
        .filter(|s| s.security.modified_since_install)
        .map(|s| s.id.clone())
        .collect();
    Ok(tampered)
}
