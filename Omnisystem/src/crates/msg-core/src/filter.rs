use crate::Message;

/// AI-powered spam filter
pub struct SpamFilter;

impl SpamFilter {
    /// Score a message between 0.0 (definitely ham) and 1.0 (definitely spam)
    pub fn score(&self, _msg: &Message) -> f32 {
        // Placeholder: BonsAI V2 classification
        0.0
    }

    /// Check if a message is spam
    pub fn is_spam(&self, msg: &Message, threshold: f32) -> bool {
        self.score(msg) > threshold
    }
}

impl Copy for SpamFilter {}
impl Clone for SpamFilter {
    fn clone(&self) -> Self {
        *self
    }
}
