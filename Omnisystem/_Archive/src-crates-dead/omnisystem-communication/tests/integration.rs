use omnisystem_communication::*;

#[test]
fn test_message_router() {
    let router = MessageRouter::new();
    router.register_route("s1".to_string(), vec!["t1".to_string()]).unwrap();

    let msg = Message::new(
        "m1".to_string(),
        "s1".to_string(),
        "t1".to_string(),
        vec![1, 2],
    );
    assert!(router.send(&msg).is_ok());
}

#[test]
fn test_unknown_route() {
    let router = MessageRouter::new();
    let msg = Message::new(
        "m1".to_string(),
        "unknown".to_string(),
        "t1".to_string(),
        vec![],
    );
    assert!(router.send(&msg).is_err());
}
