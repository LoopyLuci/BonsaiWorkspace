use rand::{thread_rng, Rng};
use std::collections::VecDeque;

pub struct Corpus {
    pub inputs: VecDeque<Vec<u8>>,
    pub crash_inputs: Vec<Vec<u8>>,
}

impl Corpus {
    pub fn new() -> Self {
        Self {
            inputs: VecDeque::new(),
            crash_inputs: Vec::new(),
        }
    }

    pub fn add_corpus(&mut self, input: &[u8]) {
        if self.inputs.len() < 10000 {
            self.inputs.push_back(input.to_vec());
        }
    }

    pub fn add_crash(&mut self, input: &[u8]) {
        self.crash_inputs.push(input.to_vec());
    }

    pub fn generate_random_input(&self, size: usize) -> Vec<u8> {
        let mut rng = thread_rng();
        (0..size).map(|_| rng.gen::<u8>()).collect()
    }

    pub fn mutate_existing_input(&self) -> Vec<u8> {
        if self.inputs.is_empty() {
            return self.generate_random_input(64);
        }

        let mut rng = thread_rng();
        let idx = rng.gen_range(0..self.inputs.len());
        let mut input = self.inputs[idx].clone();

        // Mutate: flip random bits
        for _ in 0..rng.gen_range(1..5) {
            if !input.is_empty() {
                let bit_pos = rng.gen_range(0..input.len());
                input[bit_pos] = input[bit_pos].wrapping_add(rng.gen::<u8>());
            }
        }

        input
    }

    pub fn len(&self) -> usize {
        self.inputs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inputs.is_empty()
    }
}

impl Default for Corpus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_corpus_creation() {
        let corpus = Corpus::new();
        assert!(corpus.is_empty());
    }

    #[test]
    fn test_add_corpus() {
        let mut corpus = Corpus::new();
        corpus.add_corpus(&[1, 2, 3]);
        assert_eq!(corpus.len(), 1);
    }

    #[test]
    fn test_add_crash() {
        let mut corpus = Corpus::new();
        corpus.add_crash(&[0xFF, 0xFF]);
        assert_eq!(corpus.crash_inputs.len(), 1);
    }

    #[test]
    fn test_generate_random_input() {
        let corpus = Corpus::new();
        let input = corpus.generate_random_input(64);
        assert_eq!(input.len(), 64);
    }

    #[test]
    fn test_mutate_existing_input() {
        let mut corpus = Corpus::new();
        corpus.add_corpus(&[1, 2, 3, 4, 5]);
        let mutated = corpus.mutate_existing_input();
        assert!(mutated.len() > 0);
    }

    #[test]
    fn test_corpus_size_limit() {
        let mut corpus = Corpus::new();
        for _ in 0..20000 {
            corpus.add_corpus(&[1, 2, 3]);
        }
        assert!(corpus.len() <= 10000);
    }
}
