use health_checker::*;

#[tokio::test]
async fn test_full_health_check() {
    let checker = checker::HealthChecker::new();
    
    checker.check_service("api".to_string()).await.unwrap();
    checker.check_service("database".to_string()).await.unwrap();
    checker.check_service("cache".to_string()).await.unwrap();
    
    assert_eq!(checker.check_count(), 3);
    assert!(checker.all_healthy());
    assert_eq!(checker.get_status("api"), Some(HealthStatus::Healthy));
}
