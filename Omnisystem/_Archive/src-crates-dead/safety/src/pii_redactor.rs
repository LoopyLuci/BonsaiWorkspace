use regex::Regex;

pub struct PiiRedactor {
    email_pattern: Regex,
    phone_pattern: Regex,
    ssn_pattern: Regex,
}

impl PiiRedactor {
    pub fn new() -> Self {
        Self {
            email_pattern: Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}")
                .expect("email regex"),
            phone_pattern: Regex::new(r"\\+?[\\d\\s\\-\\(\\)]{7,}").expect("phone regex"),
            ssn_pattern: Regex::new(r"\\d{3}-\\d{2}-\\d{4}").expect("ssn regex"),
        }
    }

    pub fn redact(&self, text: &str) -> String {
        let mut result = text.to_string();

        result = self.email_pattern.replace_all(&result, "[EMAIL]").to_string();
        result = self.phone_pattern.replace_all(&result, "[PHONE]").to_string();
        result = self.ssn_pattern.replace_all(&result, "[SSN]").to_string();

        result
    }
}
