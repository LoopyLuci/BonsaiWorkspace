use event_bus::*;

#[tokio::test]
async fn test_event_system() {
    let bus = bus::EventBus::new();
    
    let event1 = event::Event::new(
        "order.created".to_string(),
        serde_json::json!({"order_id": "123"}),
    );
    
    let event2 = event::Event::new(
        "payment.processed".to_string(),
        serde_json::json!({"amount": 99.99}),
    );
    
    bus.publish(event1).await.unwrap();
    bus.publish(event2).await.unwrap();
    
    assert_eq!(bus.event_count(), 2);
}
