use omni_bot::*;

#[tokio::test]
async fn test_omnibot_full_workflow() {
    let bot = orchestrator::OmniBot::new("1.0.0".to_string());
    
    assert!(bot.initialize().await.is_ok());
    assert_eq!(bot.get_state(), orchestrator::BotState::Ready);
    
    let request = orchestrator::RequestCommand {
        service: "iot".to_string(),
        payload: "control_device".to_string(),
    };
    
    let response = bot.execute_request(request).await.unwrap();
    assert!(response.contains("IoT processed"));
    
    let task_id = bot.create_task("test_task".to_string());
    assert!(bot.update_task(&task_id, 1.0, "completed".to_string()).is_ok());
    assert_eq!(bot.active_task_count(), 1);
}

#[test]
fn test_request_handler() {
    assert!(request_handler::RequestHandler::validate_request("valid").is_ok());
    
    let (service, payload) = request_handler::RequestHandler::parse_command("search:query_db").unwrap();
    assert_eq!(service, "search");
    
    let response = request_handler::RequestHandler::build_response("network", "ok");
    assert!(response.contains("network"));
}

#[tokio::test]
async fn test_autonomous_decisions() {
    let engine = autonomous::AutonomousEngine::new();
    
    let decision = autonomous::Decision {
        id: "d1".to_string(),
        action_type: "coordinate_agents".to_string(),
        parameters: std::collections::HashMap::new(),
        confidence: 0.98,
    };
    
    engine.enqueue_decision(decision);
    let result = engine.execute_next().await.unwrap();
    assert!(result.is_some());
}

#[tokio::test]
async fn test_service_routing() {
    let bridge = service_bridge::ServiceBridge::new();
    bridge.connect_all().await.unwrap();
    
    let services = vec!["iot", "search", "fabrication", "agents", "network"];
    for service in services {
        let payload = format!("{}_request", service);
        match service {
            "iot" => {
                let _ = bridge.route_to_iot(&payload).await;
            }
            "search" => {
                let _ = bridge.route_to_search(&payload).await;
            }
            "fabrication" => {
                let _ = bridge.route_to_fabrication(&payload).await;
            }
            "agents" => {
                let _ = bridge.route_to_agents(&payload).await;
            }
            "network" => {
                let _ = bridge.route_to_network(&payload).await;
            }
            _ => {}
        }
    }
}
