use std::time::Instant;

pub struct Profiler {
    start: Instant,
    name: String,
}

impl Profiler {
    pub fn new(name: String) -> Self {
        Self {
            start: Instant::now(),
            name,
        }
    }

    pub fn elapsed_ms(&self) -> u128 {
        self.start.elapsed().as_millis()
    }

    pub fn elapsed_µs(&self) -> u128 {
        self.start.elapsed().as_micros()
    }
}

impl Drop for Profiler {
    fn drop(&mut self) {
        tracing::info!("{}: {}ms", self.name, self.elapsed_ms());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_profiler() {
        let _prof = Profiler::new("test".to_string());
        std::thread::sleep(std::time::Duration::from_millis(1));
        assert!(_prof.elapsed_ms() >= 1);
    }
}
