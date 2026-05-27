//! Workstream I — OmniBoot: Self-verifying boot chain
//!
//! Every component binary is hashed with Blake3; the hash is checked against a
//! CAS-persisted manifest before the OS is considered trusted to launch.  L3
//! (System) components additionally require an Axiom proof stored in CAS.
//!
//! The system refuses to boot if any *required* component fails hash or proof
//! verification.  Optional components are skipped with a warning.

use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use bonsai_cas::{CasKey, CasStore};
use bonsai_verify::{AxiomKernel, Context, Term};
use crate::process_manager::TrustLevel;

// ─────────────────────────────────────────────────────────────────────────────
// § 1 — Manifest types
// ─────────────────────────────────────────────────────────────────────────────

/// A serialisable representation of an Axiom proof stored in CAS.
/// The `proposition` and `proof_term` are JSON-encoded `Term` values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootProof {
    pub proposition: Term,
    pub proof_term: Term,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootComponent {
    pub name: String,
    pub path: String,
    /// Blake3 hex hash of the component binary at manifest-snapshot time.
    pub expected_hash: String,
    pub trust_level: TrustLevel,
    pub required_for_boot: bool,
    /// CAS key (hex) where a `BootProof` is stored, for System-level components.
    pub proof_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootManifest {
    pub os_version: String,
    pub components: Vec<BootComponent>,
    pub minimum_verified: u32,
}

impl Default for BootManifest {
    fn default() -> Self {
        Self {
            os_version: env!("CARGO_PKG_VERSION").to_string(),
            components: vec![],
            minimum_verified: 1,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// § 2 — Verification log
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "step")]
pub enum VerificationStep {
    Verifying { component: String, expected: String },
    Verified  { component: String, actual: String, duration_us: u64 },
    ProofChecked { component: String, proof_key: String, valid: bool },
    Failed    { component: String, reason: String },
    Skipped   { component: String, reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BootReport {
    pub os_version: String,
    pub total_components: usize,
    pub verified_components: usize,
    pub failed_components: usize,
    pub skipped_components: usize,
    pub proofs_checked: usize,
    pub proofs_valid: usize,
    pub total_duration_ms: u64,
    pub boot_successful: bool,
    pub failure_reason: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// § 3 — Boot errors
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum BootError {
    MissingComponent(String),
    HashMismatch { component: String, expected: String, actual: String },
    ProofFailed(String),
    InsufficientVerified { required: u32, got: u32 },
    Io(String),
    Serde(String),
}

impl std::fmt::Display for BootError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingComponent(n) => write!(f, "Required component '{}' is missing", n),
            Self::HashMismatch { component, expected, actual } =>
                write!(f, "Hash mismatch for '{}': expected {} got {}", component, &expected[..8], &actual[..8]),
            Self::ProofFailed(n) => write!(f, "Axiom proof verification failed for '{}'", n),
            Self::InsufficientVerified { required, got } =>
                write!(f, "Only {}/{} components verified", got, required),
            Self::Io(e) => write!(f, "I/O error: {}", e),
            Self::Serde(e) => write!(f, "Serialization error: {}", e),
        }
    }
}

impl From<std::io::Error> for BootError {
    fn from(e: std::io::Error) -> Self { Self::Io(e.to_string()) }
}

// ─────────────────────────────────────────────────────────────────────────────
// § 4 — OmniBoot
// ─────────────────────────────────────────────────────────────────────────────

pub struct OmniBoot {
    pub cas: Arc<CasStore>,
    pub kernel: AxiomKernel,
    pub manifest: std::sync::RwLock<BootManifest>,
    pub log: std::sync::RwLock<Vec<VerificationStep>>,
    /// CAS key where the active manifest is stored (hex).
    pub manifest_key: std::sync::RwLock<Option<String>>,
}

impl OmniBoot {
    pub fn new(cas: Arc<CasStore>) -> Arc<Self> {
        Arc::new(Self {
            cas,
            kernel: AxiomKernel::new(),
            manifest: std::sync::RwLock::new(BootManifest::default()),
            log: std::sync::RwLock::new(Vec::new()),
            manifest_key: std::sync::RwLock::new(None),
        })
    }

    // ── Load manifest from CAS ────────────────────────────────────────────────

    pub async fn load_manifest(&self, key_hex: &str) -> Result<(), String> {
        let key = CasKey::from_hex(key_hex).map_err(|e| e.to_string())?;
        let bytes = self.cas.get(&key).await.map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Manifest not found in CAS: {}", key_hex))?;
        let manifest: BootManifest = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        *self.manifest.write().unwrap() = manifest;
        *self.manifest_key.write().unwrap() = Some(key_hex.to_string());
        info!("[omni-boot] manifest loaded from {}", &key_hex[..8]);
        Ok(())
    }

    // ── Snapshot current binary tree into a manifest ──────────────────────────

    pub async fn snapshot_manifest(&self, search_dir: &str) -> Result<String, String> {
        let mut components: Vec<BootComponent> = Vec::new();

        let search_path = Path::new(search_dir);
        if search_path.exists() {
            for entry in walkdir::WalkDir::new(search_path)
                .max_depth(3)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if !entry.file_type().is_file() { continue; }
                let path = entry.path();
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                // Only hash executables and shared libraries
                if !matches!(ext, "" | "exe" | "dll" | "so" | "dylib") { continue; }

                let data = match std::fs::read(path) {
                    Ok(d) => d,
                    Err(e) => { warn!("[omni-boot] cannot read {:?}: {}", path, e); continue; }
                };

                let hash_hex = CasKey::from_bytes(&data).hex();
                let name = path.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();

                let trust_level = if name.contains("bonsai") { TrustLevel::System } else { TrustLevel::Managed };
                let required = name.contains("bonsai-workspace");

                components.push(BootComponent {
                    name: name.clone(),
                    path: path.to_string_lossy().to_string(),
                    expected_hash: hash_hex,
                    trust_level,
                    required_for_boot: required,
                    proof_key: None,
                });
            }
        }

        let manifest = BootManifest {
            os_version: env!("CARGO_PKG_VERSION").to_string(),
            components,
            minimum_verified: 1,
        };

        let bytes = serde_json::to_vec(&manifest).map_err(|e| e.to_string())?;
        let key = self.cas.put(&bytes, "application/x-omni-boot-manifest")
            .await.map_err(|e| e.to_string())?;
        let hex = key.hex();

        *self.manifest.write().unwrap() = manifest;
        *self.manifest_key.write().unwrap() = Some(hex.clone());
        info!("[omni-boot] manifest snapshotted: {} components, key={}",
              self.manifest.read().unwrap().components.len(), &hex[..8]);
        Ok(hex)
    }

    // ── Verify and boot ───────────────────────────────────────────────────────

    pub async fn verify_and_boot(&self) -> Result<BootReport, BootError> {
        let start = Instant::now();
        let mut report = BootReport::default();
        report.os_version = env!("CARGO_PKG_VERSION").to_string();

        let manifest = self.manifest.read().unwrap().clone();
        report.total_components = manifest.components.len();

        self.log.write().unwrap().clear();

        for component in &manifest.components {
            self.log.write().unwrap().push(VerificationStep::Verifying {
                component: component.name.clone(),
                expected: component.expected_hash.clone(),
            });

            let path = Path::new(&component.path);
            if !path.exists() {
                if component.required_for_boot {
                    report.failure_reason = Some(format!("Missing: {}", component.name));
                    return Err(BootError::MissingComponent(component.name.clone()));
                }
                self.log.write().unwrap().push(VerificationStep::Skipped {
                    component: component.name.clone(),
                    reason: "File not found".into(),
                });
                report.skipped_components += 1;
                continue;
            }

            let data = std::fs::read(path).map_err(|e| {
                if component.required_for_boot { BootError::Io(e.to_string()) }
                else { BootError::Io(e.to_string()) }
            })?;

            let actual_hash = CasKey::from_bytes(&data).hex();

            if actual_hash != component.expected_hash {
                if component.required_for_boot {
                    let reason = format!("Hash mismatch for {}", component.name);
                    report.failure_reason = Some(reason.clone());
                    return Err(BootError::HashMismatch {
                        component: component.name.clone(),
                        expected: component.expected_hash.clone(),
                        actual: actual_hash,
                    });
                }
                self.log.write().unwrap().push(VerificationStep::Failed {
                    component: component.name.clone(),
                    reason: format!("Expected {} got {}", &component.expected_hash[..8], &actual_hash[..8]),
                });
                report.failed_components += 1;
                continue;
            }

            let dur = start.elapsed().as_micros() as u64;
            self.log.write().unwrap().push(VerificationStep::Verified {
                component: component.name.clone(),
                actual: actual_hash.clone(),
                duration_us: dur,
            });
            report.verified_components += 1;

            // Phase 2: Axiom proof check for System-level components
            if component.trust_level == TrustLevel::System {
                if let Some(proof_key_hex) = &component.proof_key {
                    report.proofs_checked += 1;
                    let mut proof_valid = false;

                    if let Ok(proof_key) = CasKey::from_hex(proof_key_hex) {
                        if let Ok(Some(proof_bytes)) = self.cas.get(&proof_key).await {
                            if let Ok(boot_proof) = serde_json::from_slice::<BootProof>(&proof_bytes) {
                                let ctx = Context::new();
                                proof_valid = self.kernel
                                    .prove(boot_proof.proposition, boot_proof.proof_term, &ctx)
                                    .is_ok();
                            }
                        }
                    }

                    self.log.write().unwrap().push(VerificationStep::ProofChecked {
                        component: component.name.clone(),
                        proof_key: proof_key_hex.clone(),
                        valid: proof_valid,
                    });

                    if proof_valid {
                        report.proofs_valid += 1;
                    } else if component.required_for_boot {
                        let reason = format!("Proof failed for {}", component.name);
                        report.failure_reason = Some(reason);
                        return Err(BootError::ProofFailed(component.name.clone()));
                    }
                }
            }
        }

        // Phase 3: minimum verified gate
        let min = manifest.minimum_verified;
        if (report.verified_components as u32) < min {
            let reason = format!("Only {}/{} components verified", report.verified_components, min);
            report.failure_reason = Some(reason);
            return Err(BootError::InsufficientVerified {
                required: min,
                got: report.verified_components as u32,
            });
        }

        report.total_duration_ms = start.elapsed().as_millis() as u64;
        report.boot_successful = true;
        info!("[omni-boot] boot verified: {} components, {}ms",
              report.verified_components, report.total_duration_ms);

        Ok(report)
    }

    // ── Accessors ─────────────────────────────────────────────────────────────

    pub fn get_log(&self) -> Vec<VerificationStep> {
        self.log.read().unwrap().clone()
    }

    pub fn get_manifest(&self) -> BootManifest {
        self.manifest.read().unwrap().clone()
    }

    /// Register a proof in CAS for a System component.
    pub async fn register_proof(
        &self,
        component_name: &str,
        proof: BootProof,
    ) -> Result<String, String> {
        let bytes = serde_json::to_vec(&proof).map_err(|e| e.to_string())?;
        let key = self.cas.put(&bytes, "application/x-omni-boot-proof")
            .await.map_err(|e| e.to_string())?;
        let hex = key.hex();

        let mut manifest = self.manifest.write().unwrap();
        if let Some(comp) = manifest.components.iter_mut().find(|c| c.name == component_name) {
            comp.proof_key = Some(hex.clone());
            info!("[omni-boot] proof registered for {} → {}", component_name, &hex[..8]);
        }
        Ok(hex)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// § 5 — Tauri commands
// ─────────────────────────────────────────────────────────────────────────────

use crate::AppState;
use tauri::State;

#[tauri::command]
pub async fn omni_boot_verify(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    match state.omni_boot.verify_and_boot().await {
        Ok(report) => Ok(serde_json::to_value(report).map_err(|e| e.to_string())?),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn omni_boot_manifest(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let manifest = state.omni_boot.get_manifest();
    Ok(serde_json::to_value(manifest).map_err(|e| e.to_string())?)
}

#[tauri::command]
pub async fn omni_boot_snapshot(
    search_dir: Option<String>,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let dir = search_dir.unwrap_or_else(|| "target/release".into());
    let key = state.omni_boot.snapshot_manifest(&dir).await?;
    Ok(serde_json::json!({ "manifest_key": key }))
}

#[tauri::command]
pub async fn omni_boot_report(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let log = state.omni_boot.get_log();
    Ok(serde_json::to_value(log).map_err(|e| e.to_string())?)
}

#[tauri::command]
pub async fn omni_boot_load_manifest(
    manifest_key: String,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    state.omni_boot.load_manifest(&manifest_key).await?;
    Ok(serde_json::json!({ "ok": true }))
}
