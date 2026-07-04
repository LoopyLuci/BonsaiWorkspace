use rand::{thread_rng, Rng};

pub enum MutationStrategy {
    BitFlip,
    ByteFlip,
    Interesting,
    Dictionary,
    Havoc,
}

pub struct Mutator;

impl Mutator {
    pub fn apply(input: &[u8], strategy: MutationStrategy) -> Vec<u8> {
        match strategy {
            MutationStrategy::BitFlip => Self::bit_flip(input),
            MutationStrategy::ByteFlip => Self::byte_flip(input),
            MutationStrategy::Interesting => Self::interesting_values(input),
            MutationStrategy::Dictionary => Self::dictionary_mutation(input),
            MutationStrategy::Havoc => Self::havoc(input),
        }
    }

    fn bit_flip(input: &[u8]) -> Vec<u8> {
        let mut rng = thread_rng();
        let mut output = input.to_vec();
        if !output.is_empty() {
            let pos = rng.gen_range(0..output.len());
            let bit = rng.gen_range(0..8);
            output[pos] ^= 1 << bit;
        }
        output
    }

    fn byte_flip(input: &[u8]) -> Vec<u8> {
        let mut rng = thread_rng();
        let mut output = input.to_vec();
        if !output.is_empty() {
            let pos = rng.gen_range(0..output.len());
            output[pos] ^= 0xFF;
        }
        output
    }

    fn interesting_values(input: &[u8]) -> Vec<u8> {
        let mut rng = thread_rng();
        let mut output = input.to_vec();
        if !output.is_empty() {
            let pos = rng.gen_range(0..output.len());
            let interesting = [0x00, 0xFF, 0x7F, 0x80, 0x01, 0xFE];
            output[pos] = interesting[rng.gen_range(0..interesting.len())];
        }
        output
    }

    fn dictionary_mutation(input: &[u8]) -> Vec<u8> {
        let mut rng = thread_rng();
        let mut output = input.to_vec();
        let tokens = [b"GET", b"POST", b"HTTP", b"<!DOCTYPE", b"<script>"];

        if !output.is_empty() && rng.gen_bool(0.5) {
            let token = tokens[rng.gen_range(0..tokens.len())];
            let pos = rng.gen_range(0..output.len().saturating_sub(token.len()).max(1));
            if pos + token.len() <= output.len() {
                output[pos..pos + token.len()].copy_from_slice(token);
            }
        }
        output
    }

    fn havoc(input: &[u8]) -> Vec<u8> {
        let mut rng = thread_rng();
        let mut output = input.to_vec();
        let mutations = rng.gen_range(1..5);

        for _ in 0..mutations {
            let strategy = rng.gen_range(0..5);
            output = match strategy {
                0 => Self::bit_flip(&output),
                1 => Self::byte_flip(&output),
                2 => Self::interesting_values(&output),
                3 => Self::dictionary_mutation(&output),
                _ => output,
            };
        }
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bit_flip() {
        let input = vec![0x00, 0x01, 0x02];
        let output = Mutator::bit_flip(&input);
        assert_eq!(output.len(), input.len());
    }

    #[test]
    fn test_byte_flip() {
        let input = vec![0x00, 0xFF];
        let output = Mutator::byte_flip(&input);
        assert_eq!(output.len(), 2);
    }

    #[test]
    fn test_interesting_values() {
        let input = vec![0x42];
        let output = Mutator::interesting_values(&input);
        assert_eq!(output.len(), 1);
    }

    #[test]
    fn test_dictionary_mutation() {
        let input = b"GET /index HTTP/1.1\r\n".to_vec();
        let output = Mutator::dictionary_mutation(&input);
        assert!(output.len() > 0);
    }

    #[test]
    fn test_havoc() {
        let input = vec![1, 2, 3, 4, 5];
        let output = Mutator::havoc(&input);
        assert!(output.len() > 0);
    }

    #[test]
    fn test_apply() {
        let input = vec![0x42, 0x43];
        let output = Mutator::apply(&input, MutationStrategy::BitFlip);
        assert_eq!(output.len(), input.len());
    }
}
