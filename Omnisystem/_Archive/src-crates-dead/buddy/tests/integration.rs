use buddy::*;

#[tokio::test]
async fn test_buddy_full_workflow() {
    let buddy = assistant::Buddy::new("Buddy".to_string());
    
    buddy.register_capability("greet".to_string(), "Greet users".to_string()).unwrap();
    buddy.register_capability("help".to_string(), "Provide help".to_string()).unwrap();
    
    buddy.set_context("user_name".to_string(), "Alice".to_string());
    
    let response = buddy.interact("Hi Buddy".to_string()).await.unwrap();
    assert!(!response.is_empty());
    
    let caps = buddy.list_capabilities();
    assert!(caps.len() >= 8);
}

#[test]
fn test_capability_registry() {
    let registry = CapabilityRegistry::new();
    
    let all_caps = registry.list();
    assert!(all_caps.iter().any(|(name, _)| name == "iot_control"));
    assert!(all_caps.iter().any(|(name, _)| name == "search"));
    assert!(all_caps.iter().any(|(name, _)| name == "fabrication"));
    assert!(all_caps.iter().any(|(name, _)| name == "agents"));
    assert!(all_caps.iter().any(|(name, _)| name == "network"));
}

#[test]
fn test_conversation_context() {
    let context = context::ConversationContext::new();
    context.set_user_property("timezone".to_string(), "EST".to_string());
    context.store_session_data("session_id".to_string(), vec![1, 2, 3]);
    
    assert_eq!(context.get_user_property("timezone"), Some("EST".to_string()));
    assert_eq!(context.retrieve_session_data("session_id"), Some(vec![1, 2, 3]));
}
