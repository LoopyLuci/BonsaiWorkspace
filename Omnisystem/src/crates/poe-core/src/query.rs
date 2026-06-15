//! Query interface for knowledge base

pub struct QueryBuilder {
    subject: Option<String>,
    predicate: Option<String>,
    object: Option<String>,
}

impl QueryBuilder {
    pub fn new() -> Self {
        Self {
            subject: None,
            predicate: None,
            object: None,
        }
    }

    pub fn subject(mut self, s: &str) -> Self {
        self.subject = Some(s.to_string());
        self
    }

    pub fn predicate(mut self, p: &str) -> Self {
        self.predicate = Some(p.to_string());
        self
    }

    pub fn object(mut self, o: &str) -> Self {
        self.object = Some(o.to_string());
        self
    }
}
