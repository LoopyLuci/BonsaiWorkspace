use std::sync::Arc;

use skill_compiler::{
    compile_skill, compile_skill_from_str, compile_skill_with_override, distill::DistillationJob,
    load_compiled_skill, persist_compiled_skill, register_compiled_skill, verify_skill_integrity,
    CompiledSkill, SkillToolDef, ToolRegistryMut,
};
use serde_json::Value;
use tauri::State;

use crate::AppState;

// ── WasmSkillTool: bridges compiled skills into the ToolRegistry ──────────────

/// Public so `marketplace_commands` can construct it for peer-installed skills.
pub struct WasmSkillToolPublic {
    pub skill_name: String,
    pub skill_description: String,
    pub wasm_bytes: Vec<u8>,
    pub registry: Arc<crate::tool_registry::ToolRegistry>,
}

#[async_trait::async_trait]
impl crate::tool_registry::Tool for WasmSkillToolPublic {
    fn name(&self) -> &str {
        &self.skill_name
    }
    fn description(&self) -> &str {
        &self.skill_description
    }

    async fn run(&self, args: &Value) -> Result<crate::tool_registry::ToolResult, String> {
        let output = crate::plugin_host::execute_wasm_skill(
            &self.skill_name,
            &self.wasm_bytes,
            args,
            self.registry.clone(),
        )
        .await?;
        Ok(crate::tool_registry::ToolResult::text(output))
    }
}

// ── ToolRegistryMut adapter: wires WASM skills into the live registry ─────────

struct RegistryAdapter {
    registry: Arc<crate::tool_registry::ToolRegistry>,
}

impl ToolRegistryMut for RegistryAdapter {
    fn register_wasm_tool(&self, def: SkillToolDef, wasm_bytes: Vec<u8>) -> anyhow::Result<()> {
        let tool = WasmSkillToolPublic {
            skill_name: def.name.clone(),
            skill_description: def.description.clone(),
            wasm_bytes,
            registry: self.registry.clone(),
        };
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(async { self.registry.register(Box::new(tool)).await })
        });
        tracing::info!(
            name = %def.name,
            perms = ?def.requires_permissions,
            "skill registered as WASM tool"
        );
        Ok(())
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn make_adapter(state: &AppState) -> RegistryAdapter {
    RegistryAdapter {
        registry: state.tool_registry.registry.clone(),
    }
}

// ── Tauri commands ────────────────────────────────────────────────────────────

/// Compile a skill from a local directory containing `SKILL.md`.
#[tauri::command]
pub async fn compile_skill_from_path(
    state: State<'_, AppState>,
    path: String,
) -> Result<CompiledSkill, String> {
    let skill_dir = std::path::PathBuf::from(&path);
    let compiled = compile_skill(&skill_dir).await.map_err(|e| e.to_string())?;
    persist_compiled_skill(&compiled).map_err(|e| e.to_string())?;
    let adapter = make_adapter(&state);
    register_compiled_skill(&compiled, &adapter).map_err(|e| e.to_string())?;
    Ok(compiled)
}

/// Compile from raw SKILL.md content (Skills.sh network installs).
#[tauri::command]
pub async fn compile_skill_from_content(
    state: State<'_, AppState>,
    content: String,
    allow_security_concerns: Option<bool>,
) -> Result<CompiledSkill, String> {
    let compiled = compile_skill_from_str(&content, None)
        .await
        .map_err(|e| e.to_string())?;

    if !compiled.security_report.passed && allow_security_concerns != Some(true) {
        return Err(format!(
            "Security scan failed: {}",
            compiled.security_report.concerns.join("; ")
        ));
    }

    persist_compiled_skill(&compiled).map_err(|e| e.to_string())?;
    let adapter = make_adapter(&state);
    register_compiled_skill(&compiled, &adapter).map_err(|e| e.to_string())?;
    Ok(compiled)
}

/// Compile + register with optional security override.
#[tauri::command]
pub async fn compile_and_register_skill(
    state: State<'_, AppState>,
    path: String,
    allow_security_concerns: bool,
) -> Result<CompiledSkill, String> {
    let skill_dir = std::path::PathBuf::from(&path);
    let compiled = compile_skill_with_override(&skill_dir, allow_security_concerns)
        .await
        .map_err(|e| e.to_string())?;
    persist_compiled_skill(&compiled).map_err(|e| e.to_string())?;
    let adapter = make_adapter(&state);
    register_compiled_skill(&compiled, &adapter).map_err(|e| e.to_string())?;
    Ok(compiled)
}

/// Verify WASM integrity of a compiled skill by ID.
#[tauri::command]
pub async fn verify_compiled_skill(id: String) -> Result<bool, String> {
    let compiled = load_compiled_skill(&id).map_err(|e| e.to_string())?;
    Ok(verify_skill_integrity(&compiled))
}

/// List all compiled skills in `~/.bonsai/skills/compiled/`.
#[tauri::command]
pub async fn list_compiled_skills() -> Result<Vec<CompiledSkill>, String> {
    let dir = skill_compiler::compiled_skills_dir();
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut skills = Vec::new();
    for entry in std::fs::read_dir(&dir)
        .map_err(|e| e.to_string())?
        .flatten()
    {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            if let Ok(json) = std::fs::read_to_string(&path) {
                if let Ok(skill) = serde_json::from_str::<CompiledSkill>(&json) {
                    skills.push(skill);
                }
            }
        }
    }
    Ok(skills)
}

/// Invoke an installed skill by name.
#[tauri::command]
pub async fn invoke_skill(
    state: State<'_, AppState>,
    skill_name: String,
    args: Value,
) -> Result<String, String> {
    // Look up WASM bytes from disk
    let dir = skill_compiler::compiled_skills_dir();
    let mut found: Option<CompiledSkill> = None;
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Ok(json) = std::fs::read_to_string(&path) {
                    if let Ok(skill) = serde_json::from_str::<CompiledSkill>(&json) {
                        if skill.name == skill_name {
                            // Load wasm bytes
                            let wasm_path = path.with_extension("wasm");
                            if let Ok(bytes) = std::fs::read(&wasm_path) {
                                let mut full = skill;
                                full.wasm_bytes = bytes;
                                found = Some(full);
                            }
                            break;
                        }
                    }
                }
            }
        }
    }
    let compiled = found.ok_or_else(|| format!("Skill '{skill_name}' not found"))?;
    crate::plugin_host::execute_wasm_skill(
        &compiled.name,
        &compiled.wasm_bytes,
        &args,
        state.tool_registry.registry.clone(),
    )
    .await
}

/// Trigger LoRA distillation from an installed skill's rules.
#[tauri::command]
pub async fn distill_skill_to_lora(
    state: State<'_, AppState>,
    skill_id: String,
    base_model_path: Option<String>,
    output_adapter_dir: Option<String>,
) -> Result<DistillationJob, String> {
    let compiled = load_compiled_skill(&skill_id).map_err(|e| e.to_string())?;

    let rules: Vec<skill_compiler::extractor::Rule> = compiled
        .rules
        .iter()
        .map(|r| skill_compiler::extractor::Rule {
            condition: r.condition.clone(),
            action: r.action.clone(),
            confidence: r.confidence,
        })
        .collect();

    let metadata = skill_compiler::parser::SkillMetadata {
        name: compiled.name.clone(),
        description: compiled.description.clone(),
        owner: Some(compiled.id.split('/').next().unwrap_or("local").to_string()),
        version: None,
        license: None,
        tags: compiled.tags.clone(),
        bonsai: None,
    };

    let default_model = "bonsai-1.7b".to_string();

    let model_path = base_model_path.unwrap_or(default_model);
    let adapter_dir = output_adapter_dir.unwrap_or_else(|| {
        dirs::home_dir()
            .unwrap_or_default()
            .join(".bonsai")
            .join("adapters")
            .join(&compiled.id.replace('/', "__"))
            .to_string_lossy()
            .into_owned()
    });

    skill_compiler::distill::distill_skill(
        &metadata,
        &rules,
        &model_path,
        &adapter_dir,
        None,
    )
    .await
    .map_err(|e| e.to_string())
}

/// Uninstall (delete) a compiled skill from disk.
#[tauri::command]
pub async fn uninstall_compiled_skill(id: String) -> Result<(), String> {
    let dir = skill_compiler::compiled_skills_dir();
    let safe_id = id.replace('/', "__");
    let json_path = dir.join(format!("{safe_id}.json"));
    let wasm_path = dir.join(format!("{safe_id}.wasm"));
    for p in [&json_path, &wasm_path] {
        if p.exists() {
            std::fs::remove_file(p).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
