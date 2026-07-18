use bedf_concurrency::{ConcurrencyConfig, ConcurrencyScheduler, RaceDetector, ScheduleStrategy};

#[test]
fn test_race_detector_finds_write_write_conflict_across_threads() {
    let mut detector = RaceDetector::new();
    detector.record_access("counter", 1, 0, true);
    detector.record_access("counter", 2, 1, true);
    detector.record_access("counter", 1, 2, false);

    let races = detector.detect_races();
    // thread1/thread2 write-write conflict, plus each cross-thread pair
    // involving the earlier write counts as a race.
    assert!(!races.is_empty());
    assert!(races.iter().any(|r| r.is_write_write));
}

#[test]
fn test_deterministic_scheduler_is_reproducible() {
    let config = ConcurrencyConfig {
        strategy: ScheduleStrategy::Deterministic,
        num_threads: 6,
        ..ConcurrencyConfig::default()
    };
    let scheduler = ConcurrencyScheduler::new(config);

    scheduler.set_schedule(17);
    let first = scheduler.next_thread_choice();
    scheduler.set_schedule(17);
    let second = scheduler.next_thread_choice();

    assert_eq!(first, second, "same schedule id must yield same thread choice");
    assert_eq!(first, 17 % 6);
}

#[test]
fn test_coverage_scheduler_stays_in_bounds() {
    let config = ConcurrencyConfig {
        strategy: ScheduleStrategy::Coverage,
        num_threads: 3,
        ..ConcurrencyConfig::default()
    };
    let scheduler = ConcurrencyScheduler::new(config);

    for schedule_id in 0..50u32 {
        scheduler.set_schedule(schedule_id);
        let choice = scheduler.next_thread_choice();
        assert!(choice < 3);
    }
}
