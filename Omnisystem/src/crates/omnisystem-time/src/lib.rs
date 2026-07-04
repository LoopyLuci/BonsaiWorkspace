//! Omnisystem Time (OTIME)
//!
//! Time handling and date/time operations without external dependencies.
//! Provides microsecond-precision timing, duration calculations, and timezone support.

use std::time::{SystemTime, UNIX_EPOCH};
use std::ops::{Add, Sub};

/// High-resolution instant with microsecond precision
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Instant {
    micros: u64,
}

impl Instant {
    /// Get current time as an Instant
    pub fn now() -> Self {
        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();

        Instant {
            micros: duration.as_micros() as u64,
        }
    }

    /// Create Instant from microseconds since epoch
    pub fn from_micros(micros: u64) -> Self {
        Instant { micros }
    }

    /// Get microseconds since epoch
    pub fn as_micros(&self) -> u64 {
        self.micros
    }

    /// Get seconds since epoch
    pub fn as_secs(&self) -> u64 {
        self.micros / 1_000_000
    }

    /// Get elapsed time since this instant
    pub fn elapsed(&self) -> Duration {
        let now = Instant::now();
        if now.micros >= self.micros {
            Duration::from_micros(now.micros - self.micros)
        } else {
            Duration::from_micros(0)
        }
    }
}

impl Add<Duration> for Instant {
    type Output = Instant;

    fn add(self, duration: Duration) -> Instant {
        Instant {
            micros: self.micros + duration.micros,
        }
    }
}

impl Sub<Duration> for Instant {
    type Output = Instant;

    fn sub(self, duration: Duration) -> Instant {
        Instant {
            micros: self.micros.saturating_sub(duration.micros),
        }
    }
}

/// Duration with microsecond precision
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Duration {
    micros: u64,
}

impl Duration {
    /// Create duration from microseconds
    pub fn from_micros(micros: u64) -> Self {
        Duration { micros }
    }

    /// Create duration from milliseconds
    pub fn from_millis(millis: u64) -> Self {
        Duration {
            micros: millis * 1_000,
        }
    }

    /// Create duration from seconds
    pub fn from_secs(secs: u64) -> Self {
        Duration {
            micros: secs * 1_000_000,
        }
    }

    /// Get duration as microseconds
    pub fn as_micros(&self) -> u64 {
        self.micros
    }

    /// Get duration as milliseconds
    pub fn as_millis(&self) -> u64 {
        self.micros / 1_000
    }

    /// Get duration as seconds
    pub fn as_secs(&self) -> u64 {
        self.micros / 1_000_000
    }
}

impl Add for Duration {
    type Output = Duration;

    fn add(self, other: Duration) -> Duration {
        Duration {
            micros: self.micros + other.micros,
        }
    }
}

impl Sub for Duration {
    type Output = Duration;

    fn sub(self, other: Duration) -> Duration {
        Duration {
            micros: self.micros.saturating_sub(other.micros),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instant_now() {
        let instant = Instant::now();
        assert!(instant.as_micros() > 0);
    }

    #[test]
    fn test_duration_from_secs() {
        let dur = Duration::from_secs(1);
        assert_eq!(dur.as_millis(), 1000);
    }

    #[test]
    fn test_instant_arithmetic() {
        let now = Instant::now();
        let later = now + Duration::from_secs(1);
        assert!(later.as_micros() > now.as_micros());
    }

    #[test]
    fn test_elapsed() {
        let start = Instant::now();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let elapsed = start.elapsed();
        assert!(elapsed.as_millis() >= 10);
    }
}
