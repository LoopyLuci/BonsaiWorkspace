/// Bonsai Terminal Interface (BTI) commands for mobile remote desktop
use serde_json::{json, Value};

/// BTI command handler for remote desktop operations
pub fn handle_remote_command(cmd: &str, args: Vec<&str>) -> Result<Value, String> {
    let parts: Vec<&str> = cmd.split_whitespace().collect();

    if parts.is_empty() {
        return Err("Empty command".to_string());
    }

    match parts[0] {
        "remote" => handle_remote_subcommand(&parts[1..], args),
        _ => Err(format!("Unknown command: {}", parts[0])),
    }
}

fn handle_remote_subcommand(subcommands: &[&str], _args: Vec<&str>) -> Result<Value, String> {
    if subcommands.is_empty() {
        return Err("Usage: :remote <connect|disconnect|list|stats|screenshot>".to_string());
    }

    match subcommands[0] {
        "connect" => remote_connect(&subcommands[1..]),
        "disconnect" => remote_disconnect(&subcommands[1..]),
        "list" => remote_list(),
        "stats" => remote_stats(&subcommands[1..]),
        "screenshot" => remote_screenshot(&subcommands[1..]),
        _ => Err(format!("Unknown remote command: {}", subcommands[0])),
    }
}

/// `:remote connect <peer_id>` — Connect to a desktop peer
///
/// Example:
/// ```
/// :remote connect desktop-uuid-here
/// ```
fn remote_connect(args: &[&str]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Usage: :remote connect <peer_id>".to_string());
    }

    let peer_id = args[0];

    Ok(json!({
        "type": "remote_connect",
        "peer_id": peer_id,
        "status": "initiated",
        "message": format!("Connecting to peer: {}", peer_id),
        "next_steps": [
            "Waiting for peer to accept connection...",
            "If this takes >30s, peer may be offline",
            "Use ':remote list' to see available peers"
        ]
    }))
}

/// `:remote disconnect <session_id>` — Disconnect from active session
///
/// Example:
/// ```
/// :remote disconnect session-uuid-here
/// ```
fn remote_disconnect(args: &[&str]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Usage: :remote disconnect <session_id>".to_string());
    }

    let session_id = args[0];

    Ok(json!({
        "type": "remote_disconnect",
        "session_id": session_id,
        "status": "disconnecting",
        "message": format!("Closing session: {}", session_id)
    }))
}

/// `:remote list` — List available peers for connection
///
/// Example output:
/// ```
/// Peer ID                          Device Name    Status    Last Seen
/// ================================ ============== ========= ==================
/// 550e8400-e29b-41d4-a716-...      Desktop #1     online    5 min ago
/// 6ba7b810-9dad-11d1-80b4-...      Laptop         offline   2 hours ago
/// 6ba7b811-9dad-11d1-80b4-...      Work PC        online    just now
/// ```
fn remote_list() -> Result<Value, String> {
    Ok(json!({
        "type": "remote_list",
        "command": "list_available_peers",
        "message": "Fetching available peers...",
        "filters": {
            "status": "all",
            "trusted_only": false
        }
    }))
}

/// `:remote stats <session_id>` — Show real-time session statistics
///
/// Example output:
/// ```
/// Session ID: session-uuid-here
/// Connected to: Desktop #1 (192.168.1.100:5900)
/// Connection Type: local
/// Uptime: 5m 32s
///
/// Performance:
///   FPS: 60 | Bitrate: 8.5 Mbps | Latency: 2.3ms
///   Bandwidth: 142 MB (session) | Battery: -8% per hour
///   Frames: 19,800 decoded | 0 dropped
/// ```
fn remote_stats(args: &[&str]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Usage: :remote stats <session_id>".to_string());
    }

    let session_id = args[0];

    Ok(json!({
        "type": "remote_stats",
        "session_id": session_id,
        "command": "get_session_stats",
        "message": format!("Fetching stats for session: {}", session_id),
        "display_format": "table",
        "fields": [
            "fps",
            "bitrate_mbps",
            "latency_ms",
            "bandwidth_usage_mb",
            "frames_decoded",
            "frames_dropped",
            "connection_uptime_secs",
            "battery_drain_percent_per_hour"
        ]
    }))
}

/// `:remote screenshot <session_id>` — Capture current screen from remote desktop
///
/// Example:
/// ```
/// :remote screenshot session-uuid-here --quality 85 --format jpeg
/// ```
fn remote_screenshot(args: &[&str]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Usage: :remote screenshot <session_id> [--quality 0-100] [--format jpeg|png]"
            .to_string());
    }

    let session_id = args[0];
    let mut quality = 85;
    let mut format = "jpeg";

    // Parse optional arguments
    let mut i = 1;
    while i < args.len() {
        match args[i] {
            "--quality" if i + 1 < args.len() => {
                quality = args[i + 1].parse().unwrap_or(85);
                i += 2;
            }
            "--format" if i + 1 < args.len() => {
                format = args[i + 1];
                i += 2;
            }
            _ => i += 1,
        }
    }

    Ok(json!({
        "type": "remote_screenshot",
        "session_id": session_id,
        "command": "take_screenshot",
        "message": format!("Capturing screenshot from session: {}", session_id),
        "quality": quality,
        "format": format,
        "expected_format": "base64-encoded image data"
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remote_connect_command() {
        let result = remote_connect(&["peer-123"]);
        assert!(result.is_ok());
        let json = result.unwrap();
        assert_eq!(json["type"], "remote_connect");
        assert_eq!(json["peer_id"], "peer-123");
    }

    #[test]
    fn test_remote_connect_missing_peer_id() {
        let result = remote_connect(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_remote_disconnect_command() {
        let result = remote_disconnect(&["session-123"]);
        assert!(result.is_ok());
        let json = result.unwrap();
        assert_eq!(json["type"], "remote_disconnect");
        assert_eq!(json["session_id"], "session-123");
    }

    #[test]
    fn test_remote_list_command() {
        let result = remote_list();
        assert!(result.is_ok());
        let json = result.unwrap();
        assert_eq!(json["type"], "remote_list");
    }

    #[test]
    fn test_remote_stats_command() {
        let result = remote_stats(&["session-123"]);
        assert!(result.is_ok());
        let json = result.unwrap();
        assert_eq!(json["type"], "remote_stats");
    }

    #[test]
    fn test_remote_screenshot_command() {
        let result = remote_screenshot(&["session-123"]);
        assert!(result.is_ok());
        let json = result.unwrap();
        assert_eq!(json["type"], "remote_screenshot");
    }

    #[test]
    fn test_handle_remote_command() {
        let result = handle_remote_command("remote connect peer-123", vec![]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_handle_remote_command_unknown() {
        let result = handle_remote_command("remote unknown", vec![]);
        assert!(result.is_err());
    }
}
