//! Integration tests for mobile remote desktop functionality

#[cfg(test)]
mod tests {
    use mcp_server::mobile_session::{
        SessionRegistry, SessionStatus, PeerInfo, PeerStatus, SessionStats,
    };
    use chrono::Utc;

    #[tokio::test]
    async fn test_create_and_manage_session() {
        let registry = SessionRegistry::new();

        // Create session
        let session = registry.create_session("peer-123".to_string(), true).await;
        assert_eq!(session.peer_id, "peer-123");
        assert_eq!(session.status, SessionStatus::Connecting);
        assert!(session.encryption_enabled);

        // Get session
        let retrieved = registry.get_session(&session.session_id).await;
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.session_id, session.session_id);

        // Update status
        registry
            .update_status(&session.session_id, SessionStatus::Connected)
            .await;
        let updated = registry.get_session(&session.session_id).await.unwrap();
        assert_eq!(updated.status, SessionStatus::Connected);

        // Close session
        let closed = registry.close_session(&session.session_id).await;
        assert!(closed);
        assert!(registry.get_session(&session.session_id).await.is_none());
    }

    #[tokio::test]
    async fn test_session_stats_tracking() {
        let registry = SessionRegistry::new();
        let session = registry.create_session("peer-123".to_string(), true).await;

        // Update stats
        let stats = SessionStats {
            fps: 60.0,
            bitrate_mbps: 8.5,
            latency_ms: 2.3,
            bandwidth_usage_mb: 42.5,
            frames_decoded: 3600,
            frames_dropped: 2,
            connection_uptime_secs: 60,
            battery_drain_percent_per_hour: 10.2,
        };

        registry.update_stats(&session.session_id, stats.clone()).await;

        // Retrieve stats
        let retrieved = registry.get_stats(&session.session_id).await;
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.fps, 60.0);
        assert_eq!(retrieved.bitrate_mbps, 8.5);
        assert_eq!(retrieved.frames_decoded, 3600);
    }

    #[tokio::test]
    async fn test_peer_registration_and_discovery() {
        let registry = SessionRegistry::new();

        // Register peers
        let peer1 = PeerInfo {
            peer_id: "peer-1".to_string(),
            device_name: "Desktop #1".to_string(),
            device_model: "Windows 10".to_string(),
            last_seen: Utc::now(),
            status: PeerStatus::Online,
            local_ip: Some("192.168.1.100".to_string()),
            is_trusted: true,
        };

        let peer2 = PeerInfo {
            peer_id: "peer-2".to_string(),
            device_name: "Laptop".to_string(),
            device_model: "macOS 14".to_string(),
            last_seen: Utc::now(),
            status: PeerStatus::Offline,
            local_ip: None,
            is_trusted: true,
        };

        registry.register_peer(peer1.clone()).await;
        registry.register_peer(peer2.clone()).await;

        // List all peers
        let all_peers = registry.list_peers(None).await;
        assert_eq!(all_peers.len(), 2);

        // Filter online peers
        let online_peers = registry.list_peers(Some(PeerStatus::Online)).await;
        assert_eq!(online_peers.len(), 1);
        assert_eq!(online_peers[0].peer_id, "peer-1");

        // Get specific peer
        let peer = registry.get_peer("peer-1").await;
        assert!(peer.is_some());
        assert_eq!(peer.unwrap().device_name, "Desktop #1");
    }

    #[tokio::test]
    async fn test_multiple_concurrent_sessions() {
        let registry = SessionRegistry::new();

        // Create multiple sessions
        let session1 = registry.create_session("peer-1".to_string(), true).await;
        let session2 = registry.create_session("peer-2".to_string(), true).await;
        let session3 = registry.create_session("peer-3".to_string(), false).await;

        // Verify all sessions exist
        let sessions = registry.list_sessions().await;
        assert_eq!(sessions.len(), 3);

        // Verify session IDs are unique
        let ids: Vec<_> = sessions.iter().map(|s| s.session_id.clone()).collect();
        assert_eq!(ids[0], session1.session_id);
        assert_eq!(ids[1], session2.session_id);
        assert_eq!(ids[2], session3.session_id);
        assert_eq!(ids.len(), ids.iter().collect::<std::collections::HashSet<_>>().len());

        // Close some sessions
        registry.close_session(&session1.session_id).await;
        let remaining = registry.list_sessions().await;
        assert_eq!(remaining.len(), 2);
    }

    #[tokio::test]
    async fn test_session_activity_check() {
        let registry = SessionRegistry::new();
        let session = registry.create_session("peer-123".to_string(), true).await;

        // Initially connecting
        assert!(registry.is_session_active(&session.session_id).await);

        // Set to streaming
        registry
            .update_status(&session.session_id, SessionStatus::Streaming)
            .await;
        assert!(registry.is_session_active(&session.session_id).await);

        // Disconnect
        registry
            .update_status(&session.session_id, SessionStatus::Disconnected)
            .await;
        assert!(!registry.is_session_active(&session.session_id).await);
    }

    #[tokio::test]
    async fn test_nonexistent_session() {
        let registry = SessionRegistry::new();

        // Try to get non-existent session
        let session = registry.get_session("nonexistent").await;
        assert!(session.is_none());

        // Try to update non-existent session
        registry
            .update_status("nonexistent", SessionStatus::Connected)
            .await;
        // Should not panic, just silently fail

        // Try to close non-existent session
        let closed = registry.close_session("nonexistent").await;
        assert!(!closed);
    }

    #[tokio::test]
    async fn test_concurrent_session_operations() {
        let registry = std::sync::Arc::new(SessionRegistry::new());

        // Spawn multiple tasks creating sessions concurrently
        let mut handles = vec![];

        for i in 0..10 {
            let reg = registry.clone();
            let handle = tokio::spawn(async move {
                let session = reg.create_session(format!("peer-{}", i), true).await;
                session.session_id
            });
            handles.push(handle);
        }

        // Wait for all tasks
        let mut session_ids = vec![];
        for handle in handles {
            let id = handle.await.unwrap();
            session_ids.push(id);
        }

        // Verify all sessions created
        let sessions = registry.list_sessions().await;
        assert_eq!(sessions.len(), 10);

        // Verify all session IDs are unique
        let unique_ids: std::collections::HashSet<_> = session_ids.iter().cloned().collect();
        assert_eq!(unique_ids.len(), 10);
    }

    #[tokio::test]
    async fn test_peer_status_transitions() {
        let registry = SessionRegistry::new();

        let mut peer = PeerInfo {
            peer_id: "peer-1".to_string(),
            device_name: "Desktop".to_string(),
            device_model: "Windows".to_string(),
            last_seen: Utc::now(),
            status: PeerStatus::Offline,
            local_ip: None,
            is_trusted: true,
        };

        // Register offline
        registry.register_peer(peer.clone()).await;
        let offline_peers = registry.list_peers(Some(PeerStatus::Offline)).await;
        assert_eq!(offline_peers.len(), 1);

        // Update to online
        peer.status = PeerStatus::Online;
        registry.register_peer(peer.clone()).await;
        let online_peers = registry.list_peers(Some(PeerStatus::Online)).await;
        assert_eq!(online_peers.len(), 1);

        // Offline count should now be 0 (updated)
        let offline_peers = registry.list_peers(Some(PeerStatus::Offline)).await;
        assert_eq!(offline_peers.len(), 0);
    }

    #[tokio::test]
    async fn test_session_stats_default() {
        let stats = SessionStats::default();

        assert_eq!(stats.fps, 0.0);
        assert_eq!(stats.bitrate_mbps, 0.0);
        assert_eq!(stats.latency_ms, 0.0);
        assert_eq!(stats.bandwidth_usage_mb, 0.0);
        assert_eq!(stats.frames_decoded, 0);
        assert_eq!(stats.frames_dropped, 0);
        assert_eq!(stats.connection_uptime_secs, 0);
        assert_eq!(stats.battery_drain_percent_per_hour, 0.0);
    }

    #[tokio::test]
    async fn test_session_encryption_flag() {
        let registry = SessionRegistry::new();

        let encrypted = registry.create_session("peer-1".to_string(), true).await;
        assert!(encrypted.encryption_enabled);

        let unencrypted = registry.create_session("peer-2".to_string(), false).await;
        assert!(!unencrypted.encryption_enabled);

        let session1 = registry.get_session(&encrypted.session_id).await.unwrap();
        assert!(session1.encryption_enabled);

        let session2 = registry.get_session(&unencrypted.session_id).await.unwrap();
        assert!(!session2.encryption_enabled);
    }
}

/// Integration tests for UACS mobile events
#[cfg(test)]
mod uacs_tests {
    use mcp_server::uacs::{ApprovalCategory, UacsEvent};

    #[test]
    fn test_approval_category_descriptions() {
        assert_eq!(
            ApprovalCategory::RemoteFileTransfer.description(),
            "Remote file transfers via mobile session"
        );
        assert_eq!(
            ApprovalCategory::RemoteClipboardAccess.description(),
            "Clipboard access on remote desktop"
        );
        assert_eq!(
            ApprovalCategory::RemoteTunnelCreation.description(),
            "Creating tunnels for remote connections"
        );
    }

    #[test]
    fn test_remote_session_event_serialization() {
        let event = UacsEvent::RemoteSessionStarted {
            timestamp: "2024-06-30T10:30:45Z".to_string(),
            session_id: "session-123".to_string(),
            peer_id: "peer-456".to_string(),
            connection_type: "local".to_string(),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("RemoteSessionStarted"));
        assert!(json.contains("session-123"));
        assert!(json.contains("peer-456"));
    }

    #[test]
    fn test_remote_file_transfer_event() {
        let event = UacsEvent::RemoteFileTransferRequest {
            timestamp: "2024-06-30T10:30:45Z".to_string(),
            session_id: "session-123".to_string(),
            request_id: "request-789".to_string(),
            file_path: "/path/to/file.txt".to_string(),
            direction: "upload".to_string(),
            size_bytes: 1024,
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("RemoteFileTransferRequest"));
        assert!(json.contains("upload"));
        assert!(json.contains("1024"));
    }

    #[test]
    fn test_remote_session_stats_event() {
        let event = UacsEvent::RemoteSessionStats {
            timestamp: "2024-06-30T10:30:45Z".to_string(),
            session_id: "session-123".to_string(),
            fps: 59.8,
            bitrate_mbps: 8.4,
            latency_ms: 2.3,
            bandwidth_usage_mb: 42.5,
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("RemoteSessionStats"));
        assert!(json.contains("59.8"));
        assert!(json.contains("8.4"));
    }
}

/// Integration tests for BTI commands
#[cfg(test)]
mod bti_tests {
    use mcp_server::bti_commands;

    #[test]
    fn test_remote_connect_command_formatting() {
        let result = bti_commands::handle_remote_command("remote connect peer-123", vec![]);
        assert!(result.is_ok());

        let json = result.unwrap();
        assert_eq!(json["type"], "remote_connect");
        assert_eq!(json["peer_id"], "peer-123");
        assert_eq!(json["status"], "initiated");
    }

    #[test]
    fn test_remote_disconnect_command() {
        let result = bti_commands::handle_remote_command("remote disconnect session-456", vec![]);
        assert!(result.is_ok());

        let json = result.unwrap();
        assert_eq!(json["type"], "remote_disconnect");
        assert_eq!(json["session_id"], "session-456");
    }

    #[test]
    fn test_remote_list_command() {
        let result = bti_commands::handle_remote_command("remote list", vec![]);
        assert!(result.is_ok());

        let json = result.unwrap();
        assert_eq!(json["type"], "remote_list");
        assert_eq!(json["command"], "list_available_peers");
    }

    #[test]
    fn test_remote_stats_command() {
        let result = bti_commands::handle_remote_command("remote stats session-789", vec![]);
        assert!(result.is_ok());

        let json = result.unwrap();
        assert_eq!(json["type"], "remote_stats");
        assert_eq!(json["session_id"], "session-789");
    }

    #[test]
    fn test_remote_screenshot_command() {
        let result =
            bti_commands::handle_remote_command("remote screenshot session-789", vec![]);
        assert!(result.is_ok());

        let json = result.unwrap();
        assert_eq!(json["type"], "remote_screenshot");
        assert_eq!(json["format"], "jpeg");
        assert_eq!(json["quality"], 85);
    }

    #[test]
    fn test_invalid_remote_command() {
        let result = bti_commands::handle_remote_command("remote invalid", vec![]);
        assert!(result.is_err());
    }

    #[test]
    fn test_remote_command_missing_arguments() {
        let result = bti_commands::handle_remote_command("remote connect", vec![]);
        assert!(result.is_err());
    }
}
