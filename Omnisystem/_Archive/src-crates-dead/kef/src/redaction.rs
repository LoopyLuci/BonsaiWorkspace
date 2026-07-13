//! PII (Personally Identifiable Information) detection and redaction

use crate::Result;
use regex::Regex;
use std::collections::HashSet;

/// PII redaction engine
pub struct PiiRedactor {
    patterns: PiiPatterns,
    whitelist: HashSet<String>,
}

/// Compiled regex patterns for PII detection
struct PiiPatterns {
    email: Regex,
    phone: Regex,
    credit_card: Regex,
    ssn: Regex,
    date: Regex,
    url: Regex,
    ip_address: Regex,
}

impl PiiPatterns {
    /// Compile all PII patterns
    fn new() -> Self {
        Self {
            // Email pattern
            email: Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}")
                .expect("email regex"),

            // Phone patterns (US format and variations)
            phone: Regex::new(r"(?:\+?1[-.\s]?)?\(?[0-9]{3}\)?[-.\s]?[0-9]{3}[-.\s]?[0-9]{4}\b")
                .expect("phone regex"),

            // Credit card (simple Luhn-compliant pattern)
            credit_card: Regex::new(r"\b(?:\d{4}[-\s]?){3}\d{4}\b").expect("credit card regex"),

            // Social Security Number
            ssn: Regex::new(r"\b\d{3}-\d{2}-\d{4}\b").expect("ssn regex"),

            // Dates in various formats
            date: Regex::new(
                r"\b(?:\d{1,2}[/-]\d{1,2}[/-]\d{2,4}|\d{4}-\d{1,2}-\d{1,2})\b",
            )
            .expect("date regex"),

            // URLs
            url: Regex::new(r"https?://[^\s]+").expect("url regex"),

            // IP addresses
            ip_address: Regex::new(
                r"\b(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\b",
            )
            .expect("ip regex"),
        }
    }
}

impl PiiRedactor {
    /// Create a new PII redactor
    pub fn new() -> Self {
        Self {
            patterns: PiiPatterns::new(),
            whitelist: HashSet::new(),
        }
    }

    /// Add an entity to the whitelist (won't be redacted)
    pub fn whitelist_entity(&mut self, entity: String) {
        self.whitelist.insert(entity);
    }

    /// Redact all PII from text
    pub fn redact(&self, text: &str) -> String {
        let mut redacted = text.to_string();

        // Redact emails
        redacted = self
            .patterns
            .email
            .replace_all(&redacted, "[EMAIL]")
            .to_string();

        // Redact phone numbers
        redacted = self
            .patterns
            .phone
            .replace_all(&redacted, "[PHONE]")
            .to_string();

        // Redact credit cards
        redacted = self
            .patterns
            .credit_card
            .replace_all(&redacted, "[CREDIT_CARD]")
            .to_string();

        // Redact SSNs
        redacted = self
            .patterns
            .ssn
            .replace_all(&redacted, "[SSN]")
            .to_string();

        // Redact IP addresses
        redacted = self
            .patterns
            .ip_address
            .replace_all(&redacted, "[IP_ADDRESS]")
            .to_string();

        // Note: URLs and dates are often useful for knowledge, so we're lenient
        // In strict mode, you might redact these too

        redacted
    }

    /// Detect if text contains PII
    pub fn has_pii(&self, text: &str) -> bool {
        self.patterns.email.is_match(text)
            || self.patterns.phone.is_match(text)
            || self.patterns.credit_card.is_match(text)
            || self.patterns.ssn.is_match(text)
            || self.patterns.ip_address.is_match(text)
    }

    /// Count PII occurrences in text
    pub fn count_pii(&self, text: &str) -> usize {
        let mut count = 0;
        count += self.patterns.email.find_iter(text).count();
        count += self.patterns.phone.find_iter(text).count();
        count += self.patterns.credit_card.find_iter(text).count();
        count += self.patterns.ssn.find_iter(text).count();
        count += self.patterns.ip_address.find_iter(text).count();
        count
    }
}

impl Default for PiiRedactor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redact_email() {
        let redactor = PiiRedactor::new();
        let text = "Contact me at john.doe@example.com for details.";
        let redacted = redactor.redact(text);
        assert!(redacted.contains("[EMAIL]"));
        assert!(!redacted.contains("john.doe@example.com"));
    }

    #[test]
    fn test_redact_phone() {
        let redactor = PiiRedactor::new();
        let text = "Call me at 555-123-4567 tomorrow.";
        let redacted = redactor.redact(text);
        assert!(redacted.contains("[PHONE]"));
        assert!(!redacted.contains("555-123-4567"));
    }

    #[test]
    fn test_redact_credit_card() {
        let redactor = PiiRedactor::new();
        let text = "My card is 4532-1111-2222-3333";
        let redacted = redactor.redact(text);
        assert!(redacted.contains("[CREDIT_CARD]"));
    }

    #[test]
    fn test_redact_ssn() {
        let redactor = PiiRedactor::new();
        let text = "SSN: 123-45-6789";
        let redacted = redactor.redact(text);
        assert!(redacted.contains("[SSN]"));
    }

    #[test]
    fn test_redact_ip_address() {
        let redactor = PiiRedactor::new();
        let text = "Server IP: 192.168.1.1";
        let redacted = redactor.redact(text);
        assert!(redacted.contains("[IP_ADDRESS]"));
    }

    #[test]
    fn test_has_pii() {
        let redactor = PiiRedactor::new();
        assert!(redactor.has_pii("Contact: john@example.com"));
        assert!(!redactor.has_pii("Just a normal text"));
    }

    #[test]
    fn test_count_pii() {
        let redactor = PiiRedactor::new();
        let text = "Email: test@example.com, Phone: 555-123-4567";
        let count = redactor.count_pii(text);
        assert!(count >= 2);
    }
}
