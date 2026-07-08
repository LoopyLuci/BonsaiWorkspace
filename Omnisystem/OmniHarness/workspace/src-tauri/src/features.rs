use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::sync::RwLock;

static FEATURES: Lazy<RwLock<FeatureFlags>> =
    Lazy::new(|| RwLock::new(FeatureFlags::load().unwrap_or_default()));

/// See `config::CONFIG_SCHEMA_VERSION` for the same pattern applied to
/// `workspace-config.json` — bumped whenever a fix changes what a *correct*
/// persisted value should be, so `load()` can reset exactly the values known
/// to be stale from a specific past bug instead of them overriding a fixed
/// default forever.
pub const FEATURES_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlags {
    #[serde(default)]
    pub schema_version: u32,
    pub swarm_enabled: bool,
    pub bot_enabled: bool,
    pub browser_extension_enabled: bool,
    pub android_enabled: bool,
    pub sandbox_system_enabled: bool,
    pub mobile_automation_enabled: bool,
    pub mcp_bridge_enabled: bool,
    pub cluster_orchestrator_enabled: bool,
    pub tts_enabled: bool,
    #[serde(default)]
    pub hybrid_engine_enabled: bool,
    /// WORKSPACE.md — inject the self-evolving system prompt into every chat turn.
    #[serde(default)]
    pub project_context_md_enabled: bool,
    /// Undercover Mode — strip internal product names from outputs and commits.
    #[serde(default)]
    pub undercover_mode: bool,
    /// Plan Gate — require human approval for high-risk tool calls.
    #[serde(default)]
    pub plan_gate_enabled: bool,
    /// Trusted Web Router — whitelist-based documentation fetcher.
    #[serde(default)]
    pub web_router_enabled: bool,
    /// EternalWorkshop — background memory consolidation daemon.
    #[serde(default)]
    pub eternal_workshop_enabled: bool,
    /// Model Trainer GUI — in-app training control panel.
    #[serde(default = "default_true")]
    pub model_trainer_enabled: bool,
    /// Self-Build — lets the self-upgrade agent (`agents::self_upgrader`)
    /// propose and, for low-risk sandboxed-and-tested changes, apply
    /// changes to Omnisystem's own source. Off by default; enabling this
    /// is a deliberate, explicit decision made in the Self-Build panel, not
    /// a generic flags-grid checkbox.
    #[serde(default)]
    pub self_upgrade_enabled: bool,
    /// Survival System (`survival`) — background discovery of compile
    /// errors, test failures, lints, fuzzing/sandbox findings, and runtime
    /// crashes into the Bug Database. On by default: unlike Self-Build,
    /// scanning only reads and catalogs, it never changes code (submitting
    /// a discovered bug to the self-upgrade agent still separately requires
    /// `self_upgrade_enabled`).
    #[serde(default = "default_true")]
    pub survival_enabled: bool,
}

fn default_true() -> bool {
    true
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self {
            schema_version: FEATURES_SCHEMA_VERSION,
            // Unlike the other experimental/opt-in flags below, Custom
            // Swarm has a fully-built, always-visible frontend (AgentsPanel,
            // ModelSelector's "Custom Swarm" entry) with its own real gate —
            // it only actually runs once a user has deliberately configured
            // 2+ enabled agents, and resource fit is separately checked by
            // `estimate_swarm_resources`/the RAM gate before every run. A
            // second, hidden gate behind this flag (defaulting off, with no
            // dedicated toggle — only the generic Advanced flags grid) meant
            // a user could fully set up agents, pick "Custom Swarm" from the
            // model picker, and still hit an undiscoverable "Swarm feature is
            // disabled" error with no visible path to fix it.
            swarm_enabled: true,
            bot_enabled: false,
            browser_extension_enabled: false,
            android_enabled: false,
            sandbox_system_enabled: false,
            mobile_automation_enabled: false,
            mcp_bridge_enabled: false,
            cluster_orchestrator_enabled: false,
            tts_enabled: false,
            hybrid_engine_enabled: false,
            project_context_md_enabled: true,
            undercover_mode: false,
            plan_gate_enabled: false,
            web_router_enabled: true,
            eternal_workshop_enabled: true,
            model_trainer_enabled: true,
            self_upgrade_enabled: false,
            survival_enabled: true,
        }
    }
}

fn features_path() -> std::path::PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("com.omnisystem.workspace")
        .join("features.yaml")
}

impl FeatureFlags {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let path = features_path();
        if path.exists() {
            let mut flags: Self = serde_yaml::from_str(&std::fs::read_to_string(&path)?)?;
            if flags.schema_version < FEATURES_SCHEMA_VERSION {
                let from = flags.schema_version;
                Self::migrate(&mut flags);
                flags.schema_version = FEATURES_SCHEMA_VERSION;
                tracing::info!("[features-migration] migrated features.yaml v{} -> v{}", from, FEATURES_SCHEMA_VERSION);
                if let Ok(yaml) = serde_yaml::to_string(&flags) {
                    let _ = crate::atomic_write(&path, yaml.as_bytes());
                }
            }
            return Ok(flags);
        }
        // One-time migration: copy legacy repo-root features.yaml into app data dir.
        let legacy = std::path::Path::new("config/features.yaml");
        if legacy.exists() {
            if let Ok(yaml) = std::fs::read_to_string(legacy) {
                if let Ok(flags) = serde_yaml::from_str::<Self>(&yaml) {
                    let _ = crate::atomic_write(&path, yaml.as_bytes());
                    return Ok(flags);
                }
            }
        }
        Ok(Self::default())
    }

    /// Reset exactly the values known stale from a specific past bug,
    /// gated by the version the file was last saved at (same pattern as
    /// `config::migrate_config` — see that function's doc comment).
    fn migrate(flags: &mut Self) {
        if flags.schema_version < 1 {
            // v0 -> v1: `swarm_enabled` used to default to `false` with no
            // dedicated, discoverable toggle (only the generic Advanced
            // "Feature Flags" grid) even though Custom Swarm has a
            // fully-built, always-visible frontend (AgentsPanel,
            // ModelSelector) gated by its own real precondition (2+ enabled
            // agents) plus a separate RAM-fit check before every run. A
            // persisted `false` from that era is far more likely to be the
            // stale old default than a deliberate opt-out — there was no
            // realistic way to have found and used a dedicated toggle to
            // turn it off, since none existed yet.
            if !flags.swarm_enabled {
                tracing::info!("[features-migration] resetting stale swarm_enabled=false -> true");
                flags.swarm_enabled = true;
            }
        }
    }

    pub fn global() -> FeatureFlags {
        FEATURES.read().unwrap().clone()
    }

    pub fn set_global(flags: FeatureFlags) {
        *FEATURES.write().unwrap() = flags;
        if let Ok(yaml) = serde_yaml::to_string(&*FEATURES.read().unwrap()) {
            let _ = crate::atomic_write(&features_path(), yaml.as_bytes());
        }
    }

    pub fn is_enabled(flag: &str) -> bool {
        let f = FEATURES.read().unwrap();
        match flag {
            "swarm" => f.swarm_enabled,
            "bot" => f.bot_enabled,
            "browser_extension" => f.browser_extension_enabled,
            "android" => f.android_enabled,
            "sandbox_system" => f.sandbox_system_enabled,
            "mobile_automation" => f.mobile_automation_enabled,
            "mcp_bridge" => f.mcp_bridge_enabled,
            "cluster_orchestrator" => f.cluster_orchestrator_enabled,
            "tts" => f.tts_enabled,
            "hybrid_engine_enabled" => f.hybrid_engine_enabled,
            "self_upgrade" => f.self_upgrade_enabled,
            "survival" => f.survival_enabled,
            _ => false,
        }
    }
}

#[tauri::command]
#[specta::specta]
pub fn get_feature_flags() -> FeatureFlags {
    FeatureFlags::global()
}

#[tauri::command]
#[specta::specta]
pub fn set_feature_flags(flags: FeatureFlags) {
    FeatureFlags::set_global(flags);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrates_stale_swarm_disabled_default() {
        let mut flags = FeatureFlags { schema_version: 0, swarm_enabled: false, ..FeatureFlags::default() };
        FeatureFlags::migrate(&mut flags);
        assert!(flags.swarm_enabled);
    }

    #[test]
    fn already_enabled_swarm_is_untouched() {
        let mut flags = FeatureFlags { schema_version: 0, swarm_enabled: true, ..FeatureFlags::default() };
        FeatureFlags::migrate(&mut flags);
        assert!(flags.swarm_enabled);
    }

    #[test]
    fn current_schema_version_default_has_swarm_enabled() {
        assert!(FeatureFlags::default().swarm_enabled);
        assert_eq!(FeatureFlags::default().schema_version, FEATURES_SCHEMA_VERSION);
    }
}
