//! CLI that exercises the race detector and concurrency scheduler.

use bedf_concurrency::{ConcurrencyConfig, ConcurrencyScheduler, RaceDetector, ScheduleStrategy};

fn main() {
    let mut detector = RaceDetector::new();
    detector.record_access("shared_counter", 1, 100, true);
    detector.record_access("shared_counter", 2, 101, true);
    detector.record_access("shared_counter", 1, 102, false);

    let races = detector.detect_races();
    println!("Detected {} potential race(s):", races.len());
    for race in &races {
        println!(
            "  {} between threads {} and {} (write/write: {})",
            race.location, race.thread1, race.thread2, race.is_write_write
        );
    }

    let config = ConcurrencyConfig {
        strategy: ScheduleStrategy::Coverage,
        num_threads: 4,
        ..ConcurrencyConfig::default()
    };
    let scheduler = ConcurrencyScheduler::new(config);

    print!("Coverage-guided thread interleaving: ");
    for schedule_id in 0..8 {
        scheduler.set_schedule(schedule_id);
        print!("{} ", scheduler.next_thread_choice());
    }
    println!();
}
