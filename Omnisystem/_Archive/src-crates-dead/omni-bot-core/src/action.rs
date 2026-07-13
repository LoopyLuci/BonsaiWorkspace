//! Action types representing commands to execute

use serde::{Deserialize, Serialize};
use crate::types::Metadata;

/// An action to be executed by the system
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum Action {
    // Service management
    StartService { name: String, config: Option<Metadata> },
    StopService { name: String, force: bool },
    RestartService { name: String },
    GetServiceStatus { name: String },
    ConfigureService { name: String, updates: Metadata },
    
    // Environment management
    CreateEnvironment { name: String, spec: Metadata },
    StartEnvironment { id: String },
    StopEnvironment { id: String, force: bool },
    DeleteEnvironment { id: String },
    SnapshotEnvironment { id: String, name: String },
    RestoreEnvironment { id: String, snapshot_id: String },
    MigrateEnvironment { id: String, target: String },
    
    // Module management
    InstallModule { name: String, version: String },
    UpdateModule { name: String, version: String },
    RemoveModule { name: String },
    
    // Asset management
    GenerateAsset { asset_type: String, description: String },
    PublishAsset { id: String },
    
    // Validation
    RunValidation { suite: String, matrix: Metadata },
    
    // Driver conversion
    ConvertDriver { dis: Metadata, target: String },
    
    // HDE management
    ToggleAIAdvisor { enabled: bool },
    
    // Custom/extensible
    Custom { name: String, params: Metadata },
}

impl Action {
    /// Get the required capability for this action
    pub fn required_capability(&self) -> String {
        match self {
            Action::StartService { .. } => "SERVICE:start".to_string(),
            Action::StopService { .. } => "SERVICE:stop".to_string(),
            Action::RestartService { .. } => "SERVICE:restart".to_string(),
            Action::GetServiceStatus { .. } => "SERVICE:view".to_string(),
            Action::ConfigureService { .. } => "SERVICE:configure".to_string(),
            
            Action::CreateEnvironment { .. } => "ENVIRONMENT:create".to_string(),
            Action::StartEnvironment { .. } => "ENVIRONMENT:start".to_string(),
            Action::StopEnvironment { .. } => "ENVIRONMENT:stop".to_string(),
            Action::DeleteEnvironment { .. } => "ENVIRONMENT:delete".to_string(),
            Action::SnapshotEnvironment { .. } => "ENVIRONMENT:snapshot".to_string(),
            Action::RestoreEnvironment { .. } => "ENVIRONMENT:restore".to_string(),
            Action::MigrateEnvironment { .. } => "ENVIRONMENT:migrate".to_string(),
            
            Action::InstallModule { .. } => "MODULE:install".to_string(),
            Action::UpdateModule { .. } => "MODULE:update".to_string(),
            Action::RemoveModule { .. } => "MODULE:remove".to_string(),
            
            Action::GenerateAsset { .. } => "ASSET:generate".to_string(),
            Action::PublishAsset { .. } => "ASSET:publish".to_string(),
            
            Action::RunValidation { .. } => "VALIDATION:run".to_string(),
            
            Action::ConvertDriver { .. } => "DRIVER:convert".to_string(),
            
            Action::ToggleAIAdvisor { .. } => "HDE:configure".to_string(),
            
            Action::Custom { name, .. } => format!("CUSTOM:{}", name),
        }
    }
}
