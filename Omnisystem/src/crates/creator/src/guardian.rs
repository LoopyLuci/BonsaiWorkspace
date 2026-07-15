//! Content safety classifier (Guardian).
//!
//! Checks generated blobs for harmful content before they are stored in CAS
//! or returned to clients.  The stub always passes; replace with a real
//! ONNX safety model (e.g. SafetyChecker, NSFW-CLIP) + Axiom formal proofs.

#[derive(Clone)]
pub struct Guardian {
    /// Maximum allowed NSFW score [0, 1].  Blobs scoring above this are rejected.
    pub threshold: f32,
}

impl Default for Guardian {
    fn default() -> Self {
        Self { threshold: 0.8 }
    }
}

impl Guardian {
    pub fn new(threshold: f32) -> Self {
        Self { threshold }
    }

    /// Check raw blob data.  Returns `Ok(())` if safe, `Err(reason)` if not.
    pub fn check(&self, _data: &[u8]) -> Result<(), String> {
        // === Production path (stubbed) ===
        // 1. Run NSFW-CLIP or equivalent ONNX model on the image/audio
        // 2. If score > self.threshold → Err("nsfw content detected")
        // 3. Text prompts: check against blocked-term list + toxicity model
        Ok(())
    }

    /// Check a text prompt before generation starts.
    pub fn check_prompt(&self, prompt: &str) -> Result<(), String> {
        // Minimal blocked-term check; expand with a real classifier.
        let blocked = ["child", "minor", "underage"];
        for term in blocked {
            if prompt.to_lowercase().contains(term) {
                return Err(format!("prompt contains blocked term: '{term}'"));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_prompt_blocks_disallowed_terms() {
        let guardian = Guardian::default();
        assert!(guardian.check_prompt("a photo of a mountain").is_ok());
        assert!(guardian.check_prompt("a photo of a MINOR").is_err());
        assert!(guardian.check_prompt("a child playing").is_err());
    }

    #[test]
    fn test_default_threshold() {
        assert_eq!(Guardian::default().threshold, 0.8);
        assert_eq!(Guardian::new(0.5).threshold, 0.5);
    }
}
