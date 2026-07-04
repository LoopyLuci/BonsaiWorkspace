//! Omnisystem ID Generation (OID)
//!
//! High-performance ID generation without external dependencies.
//! Supports UUID v4, Snowflake IDs, and ULID generation.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// UUID v4 identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UuidV4 {
    high: u64,
    low: u64,
}

impl UuidV4 {
    /// Generate a new random UUID v4
    pub fn new() -> Self {
        UuidV4 {
            high: random_u64(),
            low: random_u64(),
        }
    }

    /// Convert to string representation
    pub fn to_string(&self) -> String {
        format!(
            "{:08x}-{:04x}-4{:03x}-{:04x}-{:012x}",
            (self.high >> 32) as u32,
            ((self.high >> 16) & 0xffff) as u16,
            (self.high & 0x0fff) as u16,
            ((self.low >> 48) & 0x3fff) as u16 | 0x8000,
            self.low & 0xffffffffffff,
        )
    }
}

impl Default for UuidV4 {
    fn default() -> Self {
        Self::new()
    }
}

/// Snowflake ID generator (Twitter-style distributed IDs)
pub struct SnowflakeGen {
    epoch: u64,
    machine_id: u32,
    sequence: Arc<AtomicU64>,
    last_timestamp: Arc<AtomicU64>,
}

impl SnowflakeGen {
    /// Create new Snowflake generator with machine ID
    pub fn new(machine_id: u32) -> Self {
        SnowflakeGen {
            epoch: 1288834974657, // Twitter epoch
            machine_id: machine_id & 0x3ff,
            sequence: Arc::new(AtomicU64::new(0)),
            last_timestamp: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Generate next Snowflake ID
    pub fn next(&self) -> u64 {
        loop {
            let timestamp = current_millis() - self.epoch;
            let last = self.last_timestamp.load(Ordering::Relaxed);

            if timestamp >= last {
                let seq = self.sequence.fetch_add(1, Ordering::Relaxed) & 0xfff;

                if seq == 0 {
                    self.last_timestamp.store(timestamp, Ordering::Relaxed);
                }

                return (timestamp << 22) | ((self.machine_id as u64) << 12) | seq;
            }
        }
    }
}

/// ULID (Universally Unique Lexicographically Sortable Identifier)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ulid {
    timestamp: u64,
    randomness: u128,
}

impl Ulid {
    /// Generate new ULID
    pub fn new() -> Self {
        Ulid {
            timestamp: current_millis(),
            randomness: random_u128(),
        }
    }

    /// Get timestamp component (milliseconds since epoch)
    pub fn timestamp(&self) -> u64 {
        self.timestamp
    }

    /// Convert to string (base32 encoding)
    pub fn to_string(&self) -> String {
        let mut result = String::with_capacity(26);

        // Encode timestamp (10 chars)
        let ts = self.timestamp;
        result.push(BASE32[(ts >> 45) as usize & 0x1f]);
        result.push(BASE32[(ts >> 40) as usize & 0x1f]);
        result.push(BASE32[(ts >> 35) as usize & 0x1f]);
        result.push(BASE32[(ts >> 30) as usize & 0x1f]);
        result.push(BASE32[(ts >> 25) as usize & 0x1f]);
        result.push(BASE32[(ts >> 20) as usize & 0x1f]);
        result.push(BASE32[(ts >> 15) as usize & 0x1f]);
        result.push(BASE32[(ts >> 10) as usize & 0x1f]);
        result.push(BASE32[(ts >> 5) as usize & 0x1f]);
        result.push(BASE32[(ts & 0x1f) as usize]);

        // Encode randomness (16 chars)
        let rand = self.randomness;
        result.push(BASE32[((rand >> 120) & 0x1f) as usize]);
        result.push(BASE32[((rand >> 115) & 0x1f) as usize]);
        result.push(BASE32[((rand >> 110) & 0x1f) as usize]);
        result.push(BASE32[((rand >> 105) & 0x1f) as usize]);
        result.push(BASE32[((rand >> 100) & 0x1f) as usize]);
        result.push(BASE32[((rand >> 95) & 0x1f) as usize]);
        result.push(BASE32[((rand >> 90) & 0x1f) as usize]);
        result.push(BASE32[((rand >> 85) & 0x1f) as usize]);
        result.push(BASE32[((rand >> 80) & 0x1f) as usize]);
        result.push(BASE32[((rand >> 75) & 0x1f) as usize]);
        result.push(BASE32[((rand >> 70) & 0x1f) as usize]);
        result.push(BASE32[((rand >> 65) & 0x1f) as usize]);
        result.push(BASE32[((rand >> 60) & 0x1f) as usize]);
        result.push(BASE32[((rand >> 55) & 0x1f) as usize]);
        result.push(BASE32[((rand >> 50) & 0x1f) as usize]);
        result.push(BASE32[((rand >> 45) & 0x1f) as usize]);

        result
    }
}

impl Default for Ulid {
    fn default() -> Self {
        Self::new()
    }
}

const BASE32: &[char] = &[
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'J',
    'K', 'M', 'N', 'P', 'Q', 'R', 'S', 'T', 'V', 'W', 'X', 'Y', 'Z',
];

fn current_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn random_u64() -> u64 {
    use std::time::SystemTime;
    let nanos = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as u64;

    let mut seed = nanos.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    for _ in 0..4 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    }
    seed
}

fn random_u128() -> u128 {
    ((random_u64() as u128) << 64) | (random_u64() as u128)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uuid_v4_unique() {
        let uuid1 = UuidV4::new();
        let uuid2 = UuidV4::new();
        assert_ne!(uuid1, uuid2);
    }

    #[test]
    fn test_uuid_v4_string_format() {
        let uuid = UuidV4::new();
        let s = uuid.to_string();
        assert_eq!(s.len(), 36);
    }

    #[test]
    fn test_snowflake_monotonic() {
        let gen = SnowflakeGen::new(1);
        let id1 = gen.next();
        let id2 = gen.next();
        assert!(id2 > id1);
    }

    #[test]
    fn test_ulid_sortable() {
        let ulid1 = Ulid::new();
        std::thread::sleep(std::time::Duration::from_millis(1));
        let ulid2 = Ulid::new();
        assert!(ulid2 > ulid1);
    }

    #[test]
    fn test_ulid_string_format() {
        let ulid = Ulid::new();
        let s = ulid.to_string();
        assert_eq!(s.len(), 26);
    }
}
