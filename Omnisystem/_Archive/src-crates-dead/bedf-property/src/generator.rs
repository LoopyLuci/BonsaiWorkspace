use rand::{thread_rng, Rng};

pub struct InputGenerator;

impl InputGenerator {
    pub fn new() -> Self {
        Self
    }

    pub fn generate(&self, input_type: &str) -> Vec<u8> {
        match input_type {
            "bytes" => self.generate_bytes(),
            "numbers" => self.generate_numbers(),
            "strings" => self.generate_strings(),
            _ => self.generate_bytes(),
        }
    }

    fn generate_bytes(&self) -> Vec<u8> {
        let mut rng = thread_rng();
        let size = rng.gen_range(0..100);
        (0..size).map(|_| rng.gen::<u8>()).collect()
    }

    fn generate_numbers(&self) -> Vec<u8> {
        let mut rng = thread_rng();
        let size = rng.gen_range(1..20);
        (0..size)
            .map(|_| (rng.gen::<u32>() % 256) as u8)
            .collect()
    }

    fn generate_strings(&self) -> Vec<u8> {
        let mut rng = thread_rng();
        let size = rng.gen_range(1..50);
        (0..size)
            .map(|_| {
                let c = rng.gen_range(32u8..127u8);
                c
            })
            .collect()
    }

    pub fn shrink(&self, input: &[u8]) -> Option<Vec<u8>> {
        if input.is_empty() {
            return None;
        }

        // Simple shrinking: remove last byte
        Some(input[..input.len() - 1].to_vec())
    }
}

impl Default for InputGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_bytes() {
        let gen = InputGenerator::new();
        let bytes = gen.generate_bytes();
        assert!(bytes.len() < 100);
    }

    #[test]
    fn test_generate_numbers() {
        let gen = InputGenerator::new();
        let numbers = gen.generate_numbers();
        assert!(numbers.len() > 0);
    }

    #[test]
    fn test_generate_strings() {
        let gen = InputGenerator::new();
        let strings = gen.generate_strings();
        assert!(strings.len() > 0);
    }

    #[test]
    fn test_shrink() {
        let gen = InputGenerator::new();
        let input = vec![1, 2, 3, 4, 5];
        let shrunk = gen.shrink(&input).unwrap();
        assert_eq!(shrunk.len(), 4);
    }
}
