#[cfg(test)]
mod integration_tests {
    use crate::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_full_launcher_core_workflow() {
        // Create all managers
        let session_mgr = Arc::new(session::DefaultSessionManager::new());
        let app_registry = Arc::new(registry::DefaultAppRegistry::new());
        let launch_coord = Arc::new(coordinator::DefaultLaunchCoordinator::new());
        let lifecycle_mgr = Arc::new(lifecycle::DefaultLifecycleManager::new());

        // Create launcher core
        let core = LauncherCore::new(
            session_mgr.clone(),
            app_registry.clone(),
            launch_coord.clone(),
            lifecycle_mgr.clone(),
        )
        .await
        .unwrap();

        // Test session creation
        let session = session_mgr
            .create_session("testuser".to_string())
            .await
            .unwrap();
        assert_eq!(session.user_id, "testuser");

        // Test app registration
        let app = registry::AppMetadata {
            app_id: "test-app".to_string(),
            name: "Test App".to_string(),
            version: "1.0.0".to_string(),
            description: "Test Application".to_string(),
            executable: std::path::PathBuf::from("/usr/bin/test"),
            args: vec![],
            dependencies: vec![],
            tags: vec!["test".to_string()],
        };
        app_registry.register_app(app).await.unwrap();

        // Test app retrieval
        let retrieved = app_registry.get_app("test-app").await.unwrap();
        assert!(retrieved.is_some());

        // Test launch request
        let request = coordinator::LaunchRequest {
            request_id: uuid::Uuid::new_v4(),
            app_id: "test-app".to_string(),
            session_id: session.session_id,
            args: vec![],
            dependencies: vec![],
        };
        let request_id = launch_coord
            .submit_launch_request(request)
            .await
            .unwrap();
        assert!(launch_coord.get_launch_status(&request_id).await.unwrap().is_some());

        // Shutdown
        core.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_multiple_sessions() {
        let session_mgr = Arc::new(session::DefaultSessionManager::new());

        for i in 0..5 {
            let user_id = format!("user{}", i);
            session_mgr.create_session(user_id).await.unwrap();
        }

        let sessions = session_mgr.list_sessions().await.unwrap();
        assert_eq!(sessions.len(), 5);
    }

    #[tokio::test]
    async fn test_concurrent_operations() {
        let session_mgr = Arc::new(session::DefaultSessionManager::new());
        let mut handles = vec![];

        for i in 0..10 {
            let mgr = session_mgr.clone();
            let handle = tokio::spawn(async move {
                let user_id = format!("user{}", i);
                mgr.create_session(user_id).await.unwrap()
            });
            handles.push(handle);
        }

        for handle in handles {
            let _ = handle.await;
        }

        let sessions = session_mgr.list_sessions().await.unwrap();
        assert_eq!(sessions.len(), 10);
    }
}
