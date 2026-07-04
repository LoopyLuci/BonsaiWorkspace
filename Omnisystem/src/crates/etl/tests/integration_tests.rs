/// Comprehensive integration tests for EternalTrainingLoop
use etl::{
    EternalTrainingLoop, RuleConfidenceCalculator, RuleConfidenceAdjuster, RuleRefiner,
    ETLStorage, UniverseEventEmitter, FeedbackCollector, FeedbackEvent, FeedbackEventType,
};
use chrono::Utc;
use std::sync::Arc;

#[tokio::test]
async fn test_full_etl_cycle() {
    // Initialize components
    let storage = Arc::new(ETLStorage::new());
    let calculator = Arc::new(RuleConfidenceCalculator);
    let adjuster = Arc::new(RuleConfidenceAdjuster::new());
    let refiner = Arc::new(RuleRefiner::new());
    let event_emitter = Arc::new(UniverseEventEmitter::new());

    // Create ETL orchestrator
    let etl = EternalTrainingLoop::new(
        storage.clone(),
        calculator.clone(),
        adjuster.clone(),
        refiner.clone(),
        event_emitter.clone(),
    );

    // Run an empty cycle
    let result = etl.run_cycle().await.unwrap();
    assert_eq!(result.feedback_events_processed, 0);
    assert_eq!(result.rules_analyzed, 0);
}

#[tokio::test]
async fn test_feedback_collection_and_analysis() {
    let storage = Arc::new(ETLStorage::new());
    let collector = FeedbackCollector::new(storage.clone());

    // Simulate user fixing a rule multiple times
    for i in 0..10 {
        collector
            .on_fix_applied(
                "rule-1".to_string(),
                "test.rs".to_string(),
                42 + i,
                "user-1".to_string(),
                "success".to_string(),
            )
            .await
            .unwrap();
    }

    // Simulate some false positives
    for i in 0..2 {
        collector
            .on_false_positive_report(
                "rule-1".to_string(),
                "test.rs".to_string(),
                100 + i,
                "user-1".to_string(),
                "Not applicable here".to_string(),
            )
            .await
            .unwrap();
    }

    // Retrieve and verify feedback
    let since = Utc::now() - chrono::Duration::hours(1);
    let events = storage.get_feedback_events_since(since).await.unwrap();
    assert_eq!(events.len(), 12); // 10 fixes + 2 false positives

    // Verify counts by type
    let accepts = events
        .iter()
        .filter(|e| e.event_type == FeedbackEventType::DiagnosticAccepted)
        .count();
    assert_eq!(accepts, 10);

    let fp_reports = events
        .iter()
        .filter(|e| e.event_type == FeedbackEventType::FalsePositiveReported)
        .count();
    assert_eq!(fp_reports, 2);
}

#[tokio::test]
async fn test_confidence_calculation_pipeline() {
    let storage = Arc::new(ETLStorage::new());
    let collector = FeedbackCollector::new(storage.clone());
    let calculator = RuleConfidenceCalculator;

    // Simulate high-confidence rule (many accepts, few rejects)
    for i in 0..100 {
        collector
            .on_fix_applied(
                "good-rule".to_string(),
                "test.rs".to_string(),
                42 + i,
                "user-1".to_string(),
                "success".to_string(),
            )
            .await
            .unwrap();
    }

    for _ in 0..5 {
        collector
            .on_false_positive_report(
                "good-rule".to_string(),
                "test.rs".to_string(),
                200,
                "user-1".to_string(),
                "Not applicable".to_string(),
            )
            .await
            .unwrap();
    }

    // Retrieve events and calculate confidence
    let since = Utc::now() - chrono::Duration::hours(1);
    let events = storage.get_feedback_events_since(since).await.unwrap();
    let metrics = calculator.aggregate_metrics(&events).await.unwrap();

    let good_rule_metric = metrics.get("good-rule").unwrap();
    let confidence = calculator
        .calculate_confidence(good_rule_metric)
        .unwrap();

    // Should be high confidence: 100/(100+5) ≈ 0.95
    assert!(confidence > 0.90);

    let action = calculator.recommend_action(confidence).unwrap();
    assert_eq!(action, "promote_to_error");
}

#[tokio::test]
async fn test_low_confidence_rule_detection() {
    let storage = Arc::new(ETLStorage::new());
    let collector = FeedbackCollector::new(storage.clone());
    let calculator = RuleConfidenceCalculator;

    // Simulate low-confidence rule (many false positives)
    for i in 0..20 {
        collector
            .on_fix_applied(
                "noisy-rule".to_string(),
                "test.rs".to_string(),
                42 + i,
                "user-1".to_string(),
                "success".to_string(),
            )
            .await
            .unwrap();
    }

    for i in 0..80 {
        collector
            .on_false_positive_report(
                "noisy-rule".to_string(),
                "test.rs".to_string(),
                200 + i,
                "user-1".to_string(),
                "Not applicable".to_string(),
            )
            .await
            .unwrap();
    }

    // Retrieve events and calculate confidence
    let since = Utc::now() - chrono::Duration::hours(1);
    let events = storage.get_feedback_events_since(since).await.unwrap();
    let metrics = calculator.aggregate_metrics(&events).await.unwrap();

    let noisy_metric = metrics.get("noisy-rule").unwrap();
    let confidence = calculator.calculate_confidence(noisy_metric).unwrap();

    // Should be low confidence: 20/(20+80) = 0.2
    assert!(confidence < 0.30);

    let action = calculator.recommend_action(confidence).unwrap();
    assert_eq!(action, "disable");
}

#[tokio::test]
async fn test_dismissal_factor() {
    let storage = Arc::new(ETLStorage::new());
    let collector = FeedbackCollector::new(storage.clone());
    let calculator = RuleConfidenceCalculator;

    // Create a rule that's correct but frequently dismissed
    for i in 0..50 {
        collector
            .on_fix_applied(
                "dismissed-rule".to_string(),
                "test.rs".to_string(),
                42 + i,
                "user-1".to_string(),
                "success".to_string(),
            )
            .await
            .unwrap();
    }

    // Many dismissals reduce confidence even though fixes work
    for i in 0..50 {
        collector
            .on_diagnostic_dismissed(
                "dismissed-rule".to_string(),
                "test.rs".to_string(),
                100 + i,
                "user-1".to_string(),
                1,
            )
            .await
            .unwrap();
    }

    let since = Utc::now() - chrono::Duration::hours(1);
    let events = storage.get_feedback_events_since(since).await.unwrap();
    let metrics = calculator.aggregate_metrics(&events).await.unwrap();

    let metric = metrics.get("dismissed-rule").unwrap();
    let confidence = calculator.calculate_confidence(metric).unwrap();

    // Dismissal factor (50/100 = 0.5) should reduce confidence
    // (50/100) * (1 - 0.5 * 0.5) = 0.5 * 0.75 = 0.375
    assert!(confidence < 0.50);
}

#[tokio::test]
async fn test_mutation_proposal_for_noisy_rules() {
    let storage = Arc::new(ETLStorage::new());
    let collector = FeedbackCollector::new(storage.clone());
    let refiner = Arc::new(RuleRefiner::new());

    // Simulate a noisy rule with sufficient data
    for i in 0..60 {
        collector
            .on_fix_applied(
                "maybe-rule".to_string(),
                "test.rs".to_string(),
                42 + i,
                "user-1".to_string(),
                "success".to_string(),
            )
            .await
            .unwrap();
    }

    for i in 0..30 {
        collector
            .on_false_positive_report(
                "maybe-rule".to_string(),
                "test.rs".to_string(),
                200 + i,
                "user-1".to_string(),
                "Not applicable".to_string(),
            )
            .await
            .unwrap();
    }

    let since = Utc::now() - chrono::Duration::hours(1);
    let events = storage.get_feedback_events_since(since).await.unwrap();
    let calculator = RuleConfidenceCalculator;
    let metrics = calculator.aggregate_metrics(&events).await.unwrap();

    // Should propose refinements for noisy rules
    let proposals = refiner.propose_refinements(&metrics).await.unwrap();
    assert!(!proposals.is_empty());
    assert_eq!(proposals[0].rule_id, "maybe-rule");
    assert!(proposals[0].expected_improvement > 0.0);
}

#[tokio::test]
async fn test_event_emission() {
    let emitter = UniverseEventEmitter::new();

    // Test various event emissions don't panic
    let feedback = FeedbackEvent {
        event_id: "test".to_string(),
        event_type: FeedbackEventType::DiagnosticAccepted,
        rule_id: "rule-1".to_string(),
        file: "test.rs".to_string(),
        line: 42,
        timestamp: Utc::now(),
        user_id: "user-1".to_string(),
        action: Some("apply".to_string()),
        outcome: Some("success".to_string()),
        explanation: None,
        dismissal_count: None,
    };

    let result = emitter.emit_feedback_event(&feedback).await;
    assert!(result.is_ok());

    let cycle_result = emitter.emit_cycle_complete(100, 50, 10).await;
    assert!(cycle_result.is_ok());
}

#[tokio::test]
async fn test_storage_cleanup() {
    let storage = Arc::new(ETLStorage::new());
    let collector = FeedbackCollector::new(storage.clone());

    // Add some recent events
    collector
        .on_fix_applied(
            "rule-1".to_string(),
            "test.rs".to_string(),
            42,
            "user-1".to_string(),
            "success".to_string(),
        )
        .await
        .unwrap();

    // Retrieve all events
    let since = Utc::now() - chrono::Duration::days(200);
    let events_before = storage
        .get_feedback_events_since(since)
        .await
        .unwrap();
    assert!(!events_before.is_empty());

    // Cleanup old events (100+ days old) shouldn't remove recent ones
    let removed = storage.cleanup_old_events(100).await.unwrap();
    assert_eq!(removed, 0);

    // Recent events should still be there
    let events_after = storage
        .get_feedback_events_since(since)
        .await
        .unwrap();
    assert_eq!(events_before.len(), events_after.len());
}
