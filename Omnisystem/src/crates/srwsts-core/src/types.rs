//! Core type definitions used throughout SRWSTS
//!
//! Provides common types for durations, timestamps, and other fundamental values.

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// A duration in seconds with nanosecond precision
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Duration {
    /// Seconds component
    pub secs: u64,
    /// Nanoseconds component (0-999,999,999)
    pub nanos: u32,
}

impl Duration {
    /// Create a duration from seconds
    pub fn from_secs(secs: u64) -> Self {
        Self { secs, nanos: 0 }
    }

    /// Create a duration from milliseconds
    pub fn from_millis(millis: u64) -> Self {
        let secs = millis / 1000;
        let nanos = ((millis % 1000) * 1_000_000) as u32;
        Self { secs, nanos }
    }

    /// Convert to total milliseconds
    pub fn as_millis(&self) -> u64 {
        self.secs * 1000 + (self.nanos as u64 / 1_000_000)
    }

    /// Convert to total seconds as f64
    pub fn as_secs_f64(&self) -> f64 {
        self.secs as f64 + (self.nanos as f64 / 1_000_000_000.0)
    }

    /// Convert to std::time::Duration
    pub fn to_std_duration(&self) -> std::time::Duration {
        std::time::Duration::new(self.secs, self.nanos)
    }

    /// Convert from std::time::Duration
    pub fn from_std_duration(d: std::time::Duration) -> Self {
        Self {
            secs: d.as_secs(),
            nanos: d.subsec_nanos(),
        }
    }

    /// Check if duration is zero
    pub fn is_zero(&self) -> bool {
        self.secs == 0 && self.nanos == 0
    }

    /// Add two durations
    pub fn add(&self, other: &Duration) -> Option<Duration> {
        let mut secs = self.secs.checked_add(other.secs)?;
        let mut nanos = self.nanos + other.nanos;
        if nanos >= 1_000_000_000 {
            secs = secs.checked_add(1)?;
            nanos -= 1_000_000_000;
        }
        Some(Duration { secs, nanos })
    }

    /// Subtract two durations
    pub fn sub(&self, other: &Duration) -> Option<Duration> {
        if self < other {
            return None;
        }
        let (mut secs, mut nanos) = if self.nanos >= other.nanos {
            (self.secs - other.secs, self.nanos - other.nanos)
        } else {
            (
                self.secs.checked_sub(other.secs)?.checked_sub(1)?,
                self.nanos + 1_000_000_000 - other.nanos,
            )
        };
        if nanos >= 1_000_000_000 {
            secs = secs.checked_add(1)?;
            nanos -= 1_000_000_000;
        }
        Some(Duration { secs, nanos })
    }

    /// Multiply duration by a scalar
    pub fn mul_f64(&self, factor: f64) -> Option<Duration> {
        let total_secs = self.as_secs_f64() * factor;
        if !total_secs.is_finite() || total_secs < 0.0 {
            return None;
        }
        let secs = total_secs.trunc() as u64;
        let nanos = ((total_secs.fract()) * 1_000_000_000.0) as u32;
        Some(Duration { secs, nanos })
    }
}

impl std::fmt::Display for Duration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.secs > 0 {
            write!(f, "{}s", self.as_secs_f64())
        } else {
            write!(f, "{}ms", self.nanos / 1_000_000)
        }
    }
}

impl Default for Duration {
    fn default() -> Self {
        Self::from_secs(0)
    }
}

/// A timestamp representing seconds since Unix epoch
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Timestamp {
    /// Seconds since Unix epoch
    pub secs: u64,
    /// Nanoseconds component
    pub nanos: u32,
}

impl Timestamp {
    /// Get the current system time as a timestamp
    pub fn now() -> Self {
        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        Self {
            secs: duration.as_secs(),
            nanos: duration.subsec_nanos(),
        }
    }

    /// Create a timestamp from seconds since epoch
    pub fn from_secs(secs: u64) -> Self {
        Self { secs, nanos: 0 }
    }

    /// Convert to std::time::SystemTime
    pub fn to_system_time(&self) -> SystemTime {
        UNIX_EPOCH + std::time::Duration::new(self.secs, self.nanos)
    }

    /// Convert from std::time::SystemTime
    pub fn from_system_time(time: SystemTime) -> Self {
        let duration = time.duration_since(UNIX_EPOCH).unwrap_or_default();
        Self {
            secs: duration.as_secs(),
            nanos: duration.subsec_nanos(),
        }
    }

    /// Calculate elapsed time since this timestamp
    pub fn elapsed(&self) -> Option<Duration> {
        let now = Timestamp::now();
        if now < *self {
            return None;
        }
        let mut secs = now.secs - self.secs;
        let mut nanos = if now.nanos >= self.nanos {
            now.nanos - self.nanos
        } else {
            secs -= 1;
            now.nanos + 1_000_000_000 - self.nanos
        };
        if nanos >= 1_000_000_000 {
            secs += 1;
            nanos -= 1_000_000_000;
        }
        Some(Duration { secs, nanos })
    }

    /// Add a duration to this timestamp
    pub fn add_duration(&self, duration: &Duration) -> Option<Timestamp> {
        let mut secs = self.secs.checked_add(duration.secs)?;
        let mut nanos = self.nanos + duration.nanos;
        if nanos >= 1_000_000_000 {
            secs = secs.checked_add(1)?;
            nanos -= 1_000_000_000;
        }
        Some(Timestamp { secs, nanos })
    }

    /// Subtract a duration from this timestamp
    pub fn sub_duration(&self, duration: &Duration) -> Option<Timestamp> {
        let (mut secs, mut nanos) = if self.nanos >= duration.nanos {
            (self.secs, self.nanos - duration.nanos)
        } else {
            (
                self.secs.checked_sub(duration.secs)?.checked_sub(1)?,
                self.nanos + 1_000_000_000 - duration.nanos,
            )
        };
        secs = secs.checked_sub(duration.secs)?;
        if nanos >= 1_000_000_000 {
            secs = secs.checked_add(1)?;
            nanos -= 1_000_000_000;
        }
        Some(Timestamp { secs, nanos })
    }
}

impl std::fmt::Display for Timestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{:09}", self.secs, self.nanos)
    }
}

impl Default for Timestamp {
    fn default() -> Self {
        Self::from_secs(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_duration_from_secs() {
        let d = Duration::from_secs(5);
        assert_eq!(d.secs, 5);
        assert_eq!(d.nanos, 0);
    }

    #[test]
    fn test_duration_from_millis() {
        let d = Duration::from_millis(1500);
        assert_eq!(d.secs, 1);
        assert_eq!(d.nanos, 500_000_000);
    }

    #[test]
    fn test_duration_as_millis() {
        let d = Duration::from_millis(2500);
        assert_eq!(d.as_millis(), 2500);
    }

    #[test]
    fn test_duration_add() {
        let d1 = Duration::from_secs(5);
        let d2 = Duration::from_millis(500);
        let result = d1.add(&d2).unwrap();
        assert_eq!(result.secs, 5);
        assert_eq!(result.nanos, 500_000_000);
    }

    #[test]
    fn test_duration_sub() {
        let d1 = Duration::from_secs(10);
        let d2 = Duration::from_secs(3);
        let result = d1.sub(&d2).unwrap();
        assert_eq!(result.secs, 7);
    }

    #[test]
    fn test_duration_mul() {
        let d = Duration::from_secs(5);
        let result = d.mul_f64(2.5).unwrap();
        assert_eq!(result.secs, 12);
    }

    #[test]
    fn test_timestamp_now() {
        let ts = Timestamp::now();
        assert!(ts.secs > 0);
    }

    #[test]
    fn test_timestamp_elapsed() {
        let ts1 = Timestamp::now();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let ts2 = Timestamp::now();
        let elapsed = ts1.elapsed().unwrap();
        assert!(elapsed.as_millis() >= 10);
    }

    #[test]
    fn test_timestamp_add_duration() {
        let ts = Timestamp::from_secs(1000);
        let dur = Duration::from_secs(500);
        let result = ts.add_duration(&dur).unwrap();
        assert_eq!(result.secs, 1500);
    }
}
