use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::sync::RwLock;

static FEATURES: Lazy<RwLock<FeatureFlags>> =
    Lazy::new(|| RwLock::new(FeatureFlags::load().unwrap_or_default()));

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlags {
    pub swarm_enabled:               bool,
    pub bot_enabled:                 bool,
    pub browser_extension_enabled:   bool,
    pub android_enabled:             bool,
    pub sandbox_system_enabled:      bool,
    pub mobile_automation_enabled:   bool,
    pub mcp_bridge_enabled:          bool,
    pub cluster_orchestrator_enabled: bool,
    pub tts_enabled:                 bool,
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self {
            swarm_enabled:               false,
            bot_enabled:                 false,
            browser_extension_enabled:   false,
            android_enabled:             false,
            sandbox_system_enabled:      false,
            mobile_automation_enabled:   false,
            mcp_bridge_enabled:          false,
            cluster_orchestrator_enabled: false,
            tts_enabled:                 false,
        }
    }
}

impl FeatureFlags {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let path = std::path::Path::new("features.yaml");
        if path.exists() {
            Ok(serde_yaml::from_str(&std::fs::read_to_string(path)?)?)
        } else {
            Ok(Self::default())
        }
    }

    pub fn global() -> FeatureFlags {
        FEATURES.read().unwrap().clone()
    }

    pub fn set_global(flags: FeatureFlags) {
        *FEATURES.write().unwrap() = flags;
        if let Ok(yaml) = serde_yaml::to_string(&*FEATURES.read().unwrap()) {
            let _ = crate::atomic_write(std::path::Path::new("features.yaml"), yaml.as_bytes());
        }
    }

    pub fn is_enabled(flag: &str) -> bool {
        let f = FEATURES.read().unwrap();
        match flag {
            "swarm"               => f.swarm_enabled,
            "bot"                 => f.bot_enabled,
            "browser_extension"   => f.browser_extension_enabled,
            "android"             => f.android_enabled,
            "sandbox_system"      => f.sandbox_system_enabled,
            "mobile_automation"   => f.mobile_automation_enabled,
            "mcp_bridge"          => f.mcp_bridge_enabled,
            "cluster_orchestrator" => f.cluster_orchestrator_enabled,
            "tts"                 => f.tts_enabled,
            _                     => false,
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
