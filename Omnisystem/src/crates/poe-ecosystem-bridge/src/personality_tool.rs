use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetNarrativeModeRequest {
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetNarrativeModeResponse {
    pub success: bool,
    pub message: String,
    pub narrative_mode_active: bool,
}

pub fn set_narrative_mode_tool_schema() -> serde_json::Value {
    serde_json::json!({
        "name": "set_narrative_mode",
        "description": "Toggle the AC Poe narrative personality mode on or off. When enabled, Poe adopts the gothic persona from Altered Carbon.",
        "parameters": {
            "type": "object",
            "properties": {
                "enabled": {
                    "type": "boolean",
                    "description": "true = AC Poe gothic persona, false = standard production mode"
                }
            },
            "required": ["enabled"]
        }
    })
}

pub async fn set_narrative_mode(enabled: bool) -> Result<SetNarrativeModeResponse, String> {
    Ok(SetNarrativeModeResponse {
        success: true,
        message: if enabled {
            "AC Poe personality engaged. The Gothic Core narrative is now active.".to_string()
        } else {
            "Returning to standard production persona. Narrative mode disengaged.".to_string()
        },
        narrative_mode_active: enabled,
    })
}
