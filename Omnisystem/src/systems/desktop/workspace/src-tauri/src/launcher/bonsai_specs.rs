//! Default component specs for Bonsai Workspace.
//! The supervisor probes these in dependency order before signalling the frontend.

use super::component::ComponentSpec;

/// Build the component specs from the actual bound ports.
/// Call this after the API server and buddy server have started so the ports are known.
pub fn bonsai_components(api_port: u16, buddy_port: u16) -> Vec<ComponentSpec> {
    vec![
        ComponentSpec {
            name: "api_server".to_string(),
            health_url: Some(format!("http://127.0.0.1:{api_port}/health")),
            health_port: api_port,
            dependencies: vec![],
            retries: 5,
            retry_delay_ms: 400,
            timeout_secs: 10,
            required: true,
        },
        ComponentSpec {
            name: "buddy_api".to_string(),
            health_url: None,
            health_port: buddy_port,
            dependencies: vec!["api_server".to_string()],
            retries: 3,
            retry_delay_ms: 500,
            timeout_secs: 8,
            required: false,
        },
    ]
}
