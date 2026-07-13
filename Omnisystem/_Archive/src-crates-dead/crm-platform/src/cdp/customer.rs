//! Customer Data Model for CDP
//!
//! Unified customer representation with flexible attributes and event tracking.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Unique customer identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CustomerId {
    Email(String),
    PhoneNumber(String),
    ExternalId(String),
    AnonymousId(String),
}

impl CustomerId {
    pub fn as_str(&self) -> String {
        match self {
            CustomerId::Email(e) => e.clone(),
            CustomerId::PhoneNumber(p) => p.clone(),
            CustomerId::ExternalId(id) => id.clone(),
            CustomerId::AnonymousId(id) => id.clone(),
        }
    }
}

/// Customer segment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Segment {
    pub name: String,
    pub entered_at: u64,
    pub metadata: HashMap<String, String>,
}

/// Customer event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub event_type: String,
    pub timestamp: u64,
    pub properties: HashMap<String, String>,
}

/// Core customer entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Customer {
    pub primary_id: CustomerId,
    pub secondary_ids: Vec<CustomerId>,
    pub attributes: HashMap<String, String>,
    pub segments: Vec<Segment>,
    pub events: Vec<Event>,
    pub created_at: u64,
    pub updated_at: u64,
    pub lifetime_value: f64,
}

impl Customer {
    pub fn new(id: CustomerId) -> Self {
        let now = current_timestamp();
        Self {
            primary_id: id,
            secondary_ids: Vec::new(),
            attributes: HashMap::new(),
            segments: Vec::new(),
            events: Vec::new(),
            created_at: now,
            updated_at: now,
            lifetime_value: 0.0,
        }
    }

    /// Add or update attribute
    pub fn set_attribute(&mut self, key: String, value: String) {
        self.attributes.insert(key, value);
        self.updated_at = current_timestamp();
    }

    /// Get attribute value
    pub fn get_attribute(&self, key: &str) -> Option<&str> {
        self.attributes.get(key).map(|s| s.as_str())
    }

    /// Add customer to segment
    pub fn add_to_segment(&mut self, segment: Segment) {
        self.segments.push(segment);
        self.updated_at = current_timestamp();
    }

    /// Remove from segment by name
    pub fn remove_from_segment(&mut self, segment_name: &str) {
        self.segments.retain(|s| s.name != segment_name);
        self.updated_at = current_timestamp();
    }

    /// Get current segment names
    pub fn get_segments(&self) -> Vec<String> {
        self.segments.iter().map(|s| s.name.clone()).collect()
    }

    /// Track event
    pub fn track_event(&mut self, event: Event) {
        self.events.push(event);
        self.updated_at = current_timestamp();
    }

    /// Get recent events
    pub fn get_recent_events(&self, count: usize) -> Vec<Event> {
        let mut events = self.events.clone();
        events.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        events.into_iter().take(count).collect()
    }

    /// Check if customer is in segment
    pub fn is_in_segment(&self, segment_name: &str) -> bool {
        self.segments.iter().any(|s| s.name == segment_name)
    }

    /// Add secondary ID (for identity resolution)
    pub fn add_secondary_id(&mut self, id: CustomerId) {
        if !self.secondary_ids.contains(&id) {
            self.secondary_ids.push(id);
            self.updated_at = current_timestamp();
        }
    }

    /// Get all identifiers
    pub fn all_ids(&self) -> Vec<CustomerId> {
        let mut ids = vec![self.primary_id.clone()];
        ids.extend(self.secondary_ids.clone());
        ids
    }

    /// Calculate churn risk (stub)
    pub fn churn_risk(&self) -> f32 {
        // In production: analyze activity patterns, segment, LTV, engagement
        if self.events.is_empty() {
            0.8 // High risk if no recent events
        } else {
            let now = current_timestamp();
            let last_event = self.events.iter().map(|e| e.timestamp).max().unwrap_or(0);
            let days_since_activity = (now - last_event) / 86400;

            if days_since_activity > 90 {
                0.9
            } else if days_since_activity > 30 {
                0.6
            } else {
                0.2
            }
        }
    }

    /// Get customer health score (0.0 - 1.0)
    pub fn health_score(&self) -> f32 {
        let mut score = 0.5;

        // Boost for high LTV
        if self.lifetime_value > 1000.0 {
            score += 0.2;
        }

        // Boost for active engagement
        if !self.events.is_empty() {
            let now = current_timestamp();
            let last_event = self.events.iter().map(|e| e.timestamp).max().unwrap_or(0);
            let days_since = (now - last_event) / 86400;

            if days_since < 7 {
                score += 0.3;
            } else if days_since < 30 {
                score += 0.1;
            }
        }

        // Reduce for segments indicating churn
        if self.is_in_segment("at_risk") {
            score -= 0.2;
        }

        score.max(0.0).min(1.0)
    }
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_customer_creation() {
        let customer = Customer::new(CustomerId::Email("user@example.com".to_string()));
        assert_eq!(customer.primary_id, CustomerId::Email("user@example.com".to_string()));
        assert!(customer.events.is_empty());
    }

    #[test]
    fn test_customer_attributes() {
        let mut customer = Customer::new(CustomerId::Email("user@example.com".to_string()));
        customer.set_attribute("name".to_string(), "John Doe".to_string());
        assert_eq!(customer.get_attribute("name"), Some("John Doe"));
    }

    #[test]
    fn test_segment_management() {
        let mut customer = Customer::new(CustomerId::Email("user@example.com".to_string()));

        let segment = Segment {
            name: "vip".to_string(),
            entered_at: current_timestamp(),
            metadata: HashMap::new(),
        };

        customer.add_to_segment(segment);
        assert!(customer.is_in_segment("vip"));

        customer.remove_from_segment("vip");
        assert!(!customer.is_in_segment("vip"));
    }

    #[test]
    fn test_event_tracking() {
        let mut customer = Customer::new(CustomerId::Email("user@example.com".to_string()));

        let event = Event {
            event_type: "purchase".to_string(),
            timestamp: current_timestamp(),
            properties: HashMap::new(),
        };

        customer.track_event(event);
        assert_eq!(customer.events.len(), 1);
    }

    #[test]
    fn test_health_score() {
        let mut customer = Customer::new(CustomerId::Email("user@example.com".to_string()));
        customer.lifetime_value = 2000.0;

        let score = customer.health_score();
        assert!(score > 0.5);
    }

    #[test]
    fn test_churn_risk() {
        let customer = Customer::new(CustomerId::Email("user@example.com".to_string()));
        let risk = customer.churn_risk();
        assert!(risk > 0.7); // No events = high churn risk
    }

    #[test]
    fn test_identity_resolution() {
        let mut customer = Customer::new(CustomerId::Email("user@example.com".to_string()));
        customer.add_secondary_id(CustomerId::PhoneNumber("555-1234".to_string()));

        let ids = customer.all_ids();
        assert_eq!(ids.len(), 2);
    }
}
