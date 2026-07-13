use anyhow::{anyhow, Context, Result};
use root::installer::{ensure_install_root, install_path};
use root::installer::transaction::Transaction;
use root::manifest::Manifest;
use root::planner::{build_install_plan, InstallPlan};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::capability::require_capability;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteReport {
    pub operation_id: String,
    pub installed_components: Vec<String>,
    pub installed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateItem {
    pub id: String,
    pub from_version: Option<String>,
    pub to_version: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateReport {
    pub operation_id: String,
    pub updated_components: Vec<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepairIssue {
    pub component_id: String,
    pub issue: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepairReport {
    pub operation_id: String,
    pub healed_components: Vec<String>,
    pub issues: Vec<RepairIssue>,
    pub repaired_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    pub healthy: bool,
    pub issues: Vec<RepairIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEvent {
    pub id: String,
    pub timestamp: String,
    pub kind: String,
    pub summary: String,
    pub components: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootSettings {
    pub bonsai_advisor_enabled: bool,
    pub p2p_lan_enabled: bool,
    pub p2p_wan_enabled: bool,
    pub survival_warnings_enabled: bool,
    pub universe_history_enabled: bool,
    pub kdb_sharing_enabled: bool,
}

impl Default for RootSettings {
    fn default() -> Self {
        Self {
            bonsai_advisor_enabled: true,
            p2p_lan_enabled: true,
            p2p_wan_enabled: false,
            survival_warnings_enabled: true,
            universe_history_enabled: true,
            kdb_sharing_enabled: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct InstalledState {
    components: HashMap<String, InstalledComponent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct InstalledComponent {
    version: String,
    manifest_hash: String,
    dir_hash: String,
    installed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ManifestEnvelope {
    manifest: Manifest,
}

#[tauri::command]
pub async fn fetch_manifest() -> std::result::Result<Manifest, String> {
    map_err_to_string(async {
        fetch_manifest_remote().await
    }
    .await)
}

#[tauri::command]
pub fn check_installation() -> std::result::Result<bool, String> {
    map_err_to_string((|| -> Result<bool> {
        let root = install_path()?;
        Ok(root.exists() && installed_state_path(&root).exists())
    })())
}

#[tauri::command]
pub fn verify_manifest(manifest_json: String, public_key_hex: String) -> std::result::Result<bool, String> {
    map_err_to_string((|| -> Result<bool> {
        let manifest: Manifest = serde_json::from_str(&manifest_json)
            .context("invalid manifest json")?;

        let key_bytes = hex::decode(public_key_hex).context("public_key_hex is invalid hex")?;
        let key_arr: [u8; 32] = key_bytes
            .as_slice()
            .try_into()
            .map_err(|_| anyhow!("public key must be 32 bytes"))?;

        manifest.verify(&key_arr)?;
        Ok(true)
    })())
}

#[tauri::command]
pub fn plan_install(manifest_json: String, components: Vec<String>) -> std::result::Result<InstallPlan, String> {
    map_err_to_string((|| -> Result<InstallPlan> {
        let manifest: Manifest = serde_json::from_str(&manifest_json)
            .context("invalid manifest json")?;
        let plan = build_install_plan(&components, &manifest.components)?;
        Ok(plan)
    })())
}

#[tauri::command]
pub async fn execute_install(manifest_json: String, components: Vec<String>) -> std::result::Result<ExecuteReport, String> {
    map_err_to_string(async {
        require_capability("DevKitCap:install")?;

        let manifest: Manifest = serde_json::from_str(&manifest_json)
            .context("invalid manifest json")?;
        let plan = build_install_plan(&components, &manifest.components)?;

        let root = ensure_install_root()?;
        let mut tx = Transaction::new()?;

        for id in &plan.component_ids {
            let component = manifest
                .components
                .iter()
                .find(|c| c.id == *id)
                .ok_or_else(|| anyhow!("component not found in manifest: {id}"))?;

            if let Err(e) = tx.download_and_stage(component, &root).await {
                tx.rollback(&root)?;
                return Err(e);
            }
        }

        tx.commit()?;
        refresh_installed_state(&root, &manifest, &plan.component_ids)?;
        write_transaction_record(&root, &plan.component_ids)?;
        append_timeline_event(&root, "install", "Installed selected components", &plan.component_ids)?;

        Ok(ExecuteReport {
            operation_id: format!("install-{}", Utc::now().timestamp_millis()),
            installed_components: plan.component_ids,
            installed_at: Utc::now().to_rfc3339(),
        })
    }.await)
}

#[tauri::command]
pub fn rollback_latest() -> std::result::Result<bool, String> {
    map_err_to_string((|| -> Result<bool> {
        require_capability("DevKitCap:repair")?;

        let root = install_path()?;
        let rollback_root = root::utils::rollback_dir()?;
        if !rollback_root.exists() {
            return Ok(false);
        }

        let latest = latest_snapshot_dir(&rollback_root)?;
        if let Some(dir) = latest {
            for entry in fs::read_dir(&dir)? {
                let entry = entry?;
                let target = root.join(entry.file_name());
                if target.exists() {
                    fs::remove_dir_all(&target)?;
                }
                fs::rename(entry.path(), target)?;
            }
            return Ok(true);
        }

        Ok(false)
    })())
}

#[tauri::command]
pub async fn update_components() -> std::result::Result<UpdateReport, String> {
    map_err_to_string(async {
        require_capability("DevKitCap:update")?;

        let root = ensure_install_root()?;
        let manifest = fetch_manifest_remote().await?;
        let state = read_installed_state(&root)?;
        let diff = compute_update_diff(&manifest, &state);

        if diff.is_empty() {
            return Ok(UpdateReport {
                operation_id: format!("update-{}", Utc::now().timestamp_millis()),
                updated_components: vec![],
                updated_at: Utc::now().to_rfc3339(),
            });
        }

        let mut tx = Transaction::new()?;
        for item in &diff {
            let component = manifest
                .components
                .iter()
                .find(|c| c.id == item.id)
                .ok_or_else(|| anyhow!("component missing in manifest: {}", item.id))?;

            if let Err(e) = tx.download_and_stage(component, &root).await {
                tx.rollback(&root)?;
                return Err(e);
            }
        }

        tx.commit()?;
        let ids = diff.iter().map(|d| d.id.clone()).collect::<Vec<_>>();
        refresh_installed_state(&root, &manifest, &ids)?;
        append_timeline_event(&root, "update", "Updated components from manifest diff", &ids)?;

        let report = UpdateReport {
            operation_id: format!("update-{}", Utc::now().timestamp_millis()),
            updated_components: ids,
            updated_at: Utc::now().to_rfc3339(),
        };
        Ok(report)
    }
    .await)
}

#[tauri::command]
    pub async fn repair_installation() -> std::result::Result<RepairReport, String> {
    map_err_to_string(async {
        require_capability("DevKitCap:repair")?;

        let root = ensure_install_root()?;
        let manifest = fetch_manifest_remote().await?;
        let state = read_installed_state(&root)?;

        let mut issues = Vec::new();
        let mut heal_ids = Vec::new();

        for (id, info) in &state.components {
            let dir = root.join(id);
            if !dir.exists() {
                issues.push(RepairIssue {
                    component_id: id.clone(),
                    issue: "component directory missing".to_string(),
                });
                heal_ids.push(id.clone());
                continue;
            }

            let current_dir_hash = compute_dir_hash(&dir)?;
            if current_dir_hash != info.dir_hash {
                issues.push(RepairIssue {
                    component_id: id.clone(),
                    issue: "component integrity mismatch".to_string(),
                });
                heal_ids.push(id.clone());
            }
        }

        if heal_ids.is_empty() {
            let report = RepairReport {
                operation_id: format!("repair-{}", Utc::now().timestamp_millis()),
                healed_components: vec![],
                issues,
                repaired_at: Utc::now().to_rfc3339(),
            };
            return Ok(report);
        }

        let mut tx = Transaction::new()?;
        for id in &heal_ids {
            let component = manifest
                .components
                .iter()
                .find(|c| c.id == *id)
                .ok_or_else(|| anyhow!("component cannot be healed; missing in manifest: {id}"))?;

            if let Err(e) = tx.download_and_stage(component, &root).await {
                tx.rollback(&root)?;
                return Err(e);
            }
        }

        tx.commit()?;
        refresh_installed_state(&root, &manifest, &heal_ids)?;
        append_timeline_event(&root, "repair", "Auto-healed corrupted/missing components", &heal_ids)?;

        let report = RepairReport {
            operation_id: format!("repair-{}", Utc::now().timestamp_millis()),
            healed_components: heal_ids,
            issues,
            repaired_at: Utc::now().to_rfc3339(),
        };
        Ok(report)
    }
    .await)
}

#[tauri::command]
pub async fn check_for_updates() -> std::result::Result<Vec<UpdateItem>, String> {
    map_err_to_string(async {
        let root = ensure_install_root()?;
        let manifest = fetch_manifest_remote().await?;
        let state = read_installed_state(&root)?;
        Ok(compute_update_diff(&manifest, &state))
    }
    .await)
}

#[tauri::command]
pub fn get_install_history() -> std::result::Result<Vec<TimelineEvent>, String> {
    map_err_to_string((|| -> Result<Vec<TimelineEvent>> {
        let root = ensure_install_root()?;
        read_timeline_events(&root)
    })())
}

#[tauri::command]
pub fn universe_rollback(event_id: String) -> std::result::Result<bool, String> {
    map_err_to_string((|| -> Result<bool> {
        require_capability("DevKitCap:repair")?;

        let root = ensure_install_root()?;
        let rollback_root = root::utils::rollback_dir()?;
        if !rollback_root.exists() {
            return Ok(false);
        }

        let target = rollback_root.join(event_id);
        if !target.exists() {
            return Ok(false);
        }

        for entry in fs::read_dir(&target)? {
            let entry = entry?;
            let out = root.join(entry.file_name());
            if out.exists() {
                fs::remove_dir_all(&out)?;
            }
            fs::rename(entry.path(), out)?;
        }
        append_timeline_event(&root, "rollback", "Restored previous snapshot", &[])?;
        Ok(true)
    })())
}

#[tauri::command]
pub fn get_health_report() -> std::result::Result<HealthReport, String> {
    map_err_to_string((|| -> Result<HealthReport> {
        let root = ensure_install_root()?;
        let state = read_installed_state(&root)?;
        let mut issues = Vec::new();
        for (id, info) in &state.components {
            let dir = root.join(id);
            if !dir.exists() {
                issues.push(RepairIssue {
                    component_id: id.clone(),
                    issue: "component directory missing".to_string(),
                });
                continue;
            }
            let current_hash = compute_dir_hash(&dir)?;
            if current_hash != info.dir_hash {
                issues.push(RepairIssue {
                    component_id: id.clone(),
                    issue: "component integrity mismatch".to_string(),
                });
            }
        }
        Ok(HealthReport {
            healthy: issues.is_empty(),
            issues,
        })
    })())
}

#[tauri::command]
pub fn get_settings() -> std::result::Result<RootSettings, String> {
    map_err_to_string((|| -> Result<RootSettings> {
        let path = settings_path()?;
        if !path.exists() {
            return Ok(RootSettings::default());
        }
        let bytes = fs::read(path)?;
        Ok(serde_json::from_slice(&bytes)?)
    })())
}

#[tauri::command]
pub fn update_settings(settings: RootSettings) -> std::result::Result<bool, String> {
    map_err_to_string((|| -> Result<bool> {
        let path = settings_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, serde_json::to_vec_pretty(&settings)?)?;
        Ok(true)
    })())
}

fn write_transaction_record(root: &PathBuf, component_ids: &[String]) -> Result<()> {
    let tx_dir = root.join(".transactions");
    fs::create_dir_all(&tx_dir)?;
    let tx_file = tx_dir.join(format!("{}.json", Utc::now().timestamp_millis()));

    let payload = serde_json::json!({
        "at": Utc::now().to_rfc3339(),
        "components": component_ids,
    });

    fs::write(tx_file, serde_json::to_vec_pretty(&payload)?)?;
    Ok(())
}

fn latest_snapshot_dir(rollback_root: &PathBuf) -> Result<Option<PathBuf>> {
    let mut dirs: Vec<PathBuf> = fs::read_dir(rollback_root)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();

    dirs.sort();
    Ok(dirs.pop())
}

fn fetch_manifest_url() -> String {
    std::env::var("BONSAI_ROOT_MANIFEST_URL")
        .unwrap_or_else(|_| "https://releases.bonsai.sh/manifest.json".to_string())
}

async fn fetch_manifest_remote() -> Result<Manifest> {
    let url = fetch_manifest_url();
    let value: serde_json::Value = reqwest::get(&url)
        .await
        .with_context(|| format!("failed to GET manifest from {url}"))?
        .error_for_status()
        .with_context(|| format!("manifest request failed for {url}"))?
        .json()
        .await
        .with_context(|| format!("failed to decode manifest json from {url}"))?;

    if value.get("manifest").is_some() {
        let env: ManifestEnvelope = serde_json::from_value(value)?;
        return Ok(env.manifest);
    }
    Ok(serde_json::from_value(value)?)
}

fn installed_state_path(root: &Path) -> PathBuf {
    root.join(".installed-state.json")
}

fn read_installed_state(root: &Path) -> Result<InstalledState> {
    let path = installed_state_path(root);
    if !path.exists() {
        return Ok(InstalledState::default());
    }
    let bytes = fs::read(path)?;
    Ok(serde_json::from_slice(&bytes)?)
}

fn write_installed_state(root: &Path, state: &InstalledState) -> Result<()> {
    let path = installed_state_path(root);
    fs::write(path, serde_json::to_vec_pretty(state)?)?;
    Ok(())
}

fn refresh_installed_state(root: &Path, manifest: &Manifest, component_ids: &[String]) -> Result<()> {
    let mut state = read_installed_state(root)?;
    for id in component_ids {
        let component = manifest
            .components
            .iter()
            .find(|c| c.id == *id)
            .ok_or_else(|| anyhow!("component missing in manifest: {id}"))?;

        let dir = root.join(id);
        let dir_hash = if dir.exists() {
            compute_dir_hash(&dir)?
        } else {
            String::new()
        };
        state.components.insert(
            id.clone(),
            InstalledComponent {
                version: component.version.clone(),
                manifest_hash: component.hash.clone(),
                dir_hash,
                installed_at: Utc::now().to_rfc3339(),
            },
        );
    }
    write_installed_state(root, &state)
}

fn compute_update_diff(manifest: &Manifest, state: &InstalledState) -> Vec<UpdateItem> {
    let mut diff = Vec::new();
    for comp in &manifest.components {
        match state.components.get(&comp.id) {
            None => diff.push(UpdateItem {
                id: comp.id.clone(),
                from_version: None,
                to_version: comp.version.clone(),
                reason: "not installed yet".to_string(),
            }),
            Some(inst) if inst.version != comp.version => diff.push(UpdateItem {
                id: comp.id.clone(),
                from_version: Some(inst.version.clone()),
                to_version: comp.version.clone(),
                reason: "version changed".to_string(),
            }),
            Some(inst) if inst.manifest_hash.to_lowercase() != comp.hash.to_lowercase() => diff.push(UpdateItem {
                id: comp.id.clone(),
                from_version: Some(inst.version.clone()),
                to_version: comp.version.clone(),
                reason: "artifact hash changed".to_string(),
            }),
            _ => {}
        }
    }
    diff
}

fn compute_dir_hash(dir: &Path) -> Result<String> {
    let mut files = Vec::new();
    gather_files_rec(dir, &mut files)?;
    files.sort();

    let mut hasher = Sha256::new();
    for path in files {
        let rel = path.strip_prefix(dir).unwrap_or(&path);
        hasher.update(rel.to_string_lossy().as_bytes());

        let mut f = fs::File::open(&path)?;
        let mut buf = [0u8; 8192];
        loop {
            let n = f.read(&mut buf)?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
        }
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn gather_files_rec(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            gather_files_rec(&path, out)?;
        } else if path.is_file() {
            out.push(path);
        }
    }
    Ok(())
}

fn history_file_path(root: &Path) -> PathBuf {
    root.join(".history.jsonl")
}

fn append_timeline_event(root: &Path, kind: &str, summary: &str, components: &[String]) -> Result<()> {
    let file = history_file_path(root);
    let event = TimelineEvent {
        id: format!("{}-{}", Utc::now().timestamp(), kind),
        timestamp: Utc::now().to_rfc3339(),
        kind: kind.to_string(),
        summary: summary.to_string(),
        components: components.to_vec(),
    };
    let mut existing = String::new();
    if file.exists() {
        existing = fs::read_to_string(&file)?;
    }
    let mut line = serde_json::to_string(&event)?;
    line.push('\n');
    existing.push_str(&line);
    fs::write(file, existing)?;
    Ok(())
}

fn read_timeline_events(root: &Path) -> Result<Vec<TimelineEvent>> {
    let file = history_file_path(root);
    if !file.exists() {
        return Ok(vec![]);
    }
    let content = fs::read_to_string(file)?;
    let mut out = Vec::new();
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let parsed: TimelineEvent = serde_json::from_str(line)?;
        out.push(parsed);
    }
    Ok(out)
}

fn settings_path() -> Result<PathBuf> {
    Ok(root::utils::state_dir()?.join("settings.json"))
}

fn map_err_to_string<T>(res: Result<T>) -> std::result::Result<T, String> {
    res.map_err(|e| e.to_string())
}
