use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct AccessRecord {
    pub thread_id: u64,
    pub timestamp: u64,
    pub is_write: bool,
    pub location: String,
}

pub struct RaceDetector {
    access_history: HashMap<String, Vec<AccessRecord>>,
}

impl RaceDetector {
    pub fn new() -> Self {
        Self {
            access_history: HashMap::new(),
        }
    }

    pub fn record_access(&mut self, location: &str, thread_id: u64, timestamp: u64, is_write: bool) {
        let record = AccessRecord {
            thread_id,
            timestamp,
            is_write,
            location: location.to_string(),
        };

        self.access_history
            .entry(location.to_string())
            .or_insert_with(Vec::new)
            .push(record);
    }

    pub fn detect_races(&self) -> Vec<RaceInfo> {
        let mut races = Vec::new();

        for (location, accesses) in &self.access_history {
            if accesses.len() < 2 {
                continue;
            }

            for i in 0..accesses.len() {
                for j in (i + 1)..accesses.len() {
                    let access1 = &accesses[i];
                    let access2 = &accesses[j];

                    if access1.thread_id != access2.thread_id
                        && (access1.is_write || access2.is_write)
                    {
                        races.push(RaceInfo {
                            location: location.clone(),
                            thread1: access1.thread_id,
                            thread2: access2.thread_id,
                            is_write_write: access1.is_write && access2.is_write,
                            timestamp1: access1.timestamp,
                            timestamp2: access2.timestamp,
                        });
                    }
                }
            }
        }

        races
    }
}

impl Default for RaceDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct RaceInfo {
    pub location: String,
    pub thread1: u64,
    pub thread2: u64,
    pub is_write_write: bool,
    pub timestamp1: u64,
    pub timestamp2: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_race_detector_creation() {
        let detector = RaceDetector::new();
        assert_eq!(detector.access_history.len(), 0);
    }

    #[test]
    fn test_record_access() {
        let mut detector = RaceDetector::new();
        detector.record_access("var_x", 1, 100, true);
        detector.record_access("var_x", 2, 101, false);
        assert_eq!(detector.access_history.len(), 1);
    }

    #[test]
    fn test_detect_races() {
        let mut detector = RaceDetector::new();
        detector.record_access("var_x", 1, 100, true);
        detector.record_access("var_x", 2, 101, true);

        let races = detector.detect_races();
        assert_eq!(races.len(), 1);
        assert!(races[0].is_write_write);
    }

    #[test]
    fn test_no_race_same_thread() {
        let mut detector = RaceDetector::new();
        detector.record_access("var_x", 1, 100, true);
        detector.record_access("var_x", 1, 101, false);

        let races = detector.detect_races();
        assert_eq!(races.len(), 0);
    }

    #[test]
    fn test_no_race_read_read() {
        let mut detector = RaceDetector::new();
        detector.record_access("var_x", 1, 100, false);
        detector.record_access("var_x", 2, 101, false);

        let races = detector.detect_races();
        assert_eq!(races.len(), 0);
    }
}
