use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;

pub struct Tokenizer {
    vocab: HashMap<String, u32>,
    reverse: HashMap<u32, String>,
    bos_token_id: u32,
    eos_token_id: u32,
}

impl Tokenizer {
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let json: serde_json::Value = serde_json::from_str(&content)?;

        let mut vocab = HashMap::new();
        let mut reverse = HashMap::new();

        // Load vocabulary from tokenizer.json
        if let Some(model_vocab) = json.get("model").and_then(|m| m.get("vocab")) {
            if let Some(vocab_obj) = model_vocab.as_object() {
                for (token, id) in vocab_obj {
                    if let Some(id_num) = id.as_u64() {
                        let id = id_num as u32;
                        vocab.insert(token.clone(), id);
                        reverse.insert(id, token.clone());
                    }
                }
            }
        }

        let bos_token_id = json
            .get("bos_token_id")
            .and_then(|v| v.as_u64())
            .unwrap_or(1) as u32;

        let eos_token_id = json
            .get("eos_token_id")
            .and_then(|v| v.as_u64())
            .unwrap_or(2) as u32;

        tracing::info!(
            "Loaded tokenizer: {} tokens, BOS={}, EOS={}",
            vocab.len(),
            bos_token_id,
            eos_token_id
        );

        Ok(Self {
            vocab,
            reverse,
            bos_token_id,
            eos_token_id,
        })
    }

    pub fn encode(&self, text: &str) -> Vec<u32> {
        let mut tokens = vec![self.bos_token_id];

        // Simple whitespace tokenization (production would use BPE)
        for word in text.split_whitespace() {
            if let Some(&id) = self.vocab.get(word) {
                tokens.push(id);
            } else {
                // Fallback: encode character by character
                for ch in word.chars() {
                    let ch_str = ch.to_string();
                    let id = self.vocab.get(&ch_str).copied().unwrap_or(0);
                    if id != 0 {
                        tokens.push(id);
                    }
                }
            }
        }

        tokens.push(self.eos_token_id);
        tokens
    }

    pub fn decode(&self, tokens: &[u32]) -> String {
        tokens
            .iter()
            .filter_map(|id| self.reverse.get(id))
            .cloned()
            .collect::<Vec<_>>()
            .join("")
    }

    pub fn vocab_size(&self) -> usize {
        self.vocab.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenizer_encode_decode() {
        let tokenizer = Tokenizer {
            vocab: {
                let mut m = HashMap::new();
                m.insert("hello".to_string(), 1);
                m.insert("world".to_string(), 2);
                m
            },
            reverse: {
                let mut m = HashMap::new();
                m.insert(1, "hello".to_string());
                m.insert(2, "world".to_string());
                m
            },
            bos_token_id: 0,
            eos_token_id: 3,
        };

        let tokens = tokenizer.encode("hello world");
        assert_eq!(tokens, vec![0, 1, 2, 3]);

        let decoded = tokenizer.decode(&tokens);
        assert_eq!(decoded, "helloworld");
    }
}
