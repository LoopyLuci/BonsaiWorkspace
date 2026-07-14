use crate::Result;

pub struct TokenBucket {
    capacity: u32,
    current_tokens: u32,
    refill_rate: u32,
}

impl TokenBucket {
    pub fn new(capacity: u32, refill_rate: u32) -> Self {
        Self {
            capacity,
            current_tokens: capacity,
            refill_rate,
        }
    }

    pub fn allow_packet(&mut self, packet_size: u32) -> Result<bool> {
        if self.current_tokens >= packet_size {
            self.current_tokens -= packet_size;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn refill(&mut self) {
        self.current_tokens = (self.current_tokens + self.refill_rate).min(self.capacity);
    }

    pub fn get_tokens(&self) -> u32 {
        self.current_tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_bucket() {
        let mut bucket = TokenBucket::new(1000, 100);
        assert!(bucket.allow_packet(500).is_ok());
        bucket.refill();
        assert!(bucket.allow_packet(700).is_ok());
    }
}
