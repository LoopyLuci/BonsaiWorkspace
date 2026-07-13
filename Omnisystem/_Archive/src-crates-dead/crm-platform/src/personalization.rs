//! Real-time Personalization

use crate::cdp::Customer;

pub struct PersonalizationContext {
    pub customer_id: String,
    pub segment: String,
    pub recommendations: Vec<String>,
}

pub struct PersonalizationEngine;

impl PersonalizationEngine {
    pub fn personalize(customer: &Customer) -> PersonalizationContext {
        let segments = customer.get_segments();
        let segment = segments.first().cloned().unwrap_or_else(|| "default".to_string());

        PersonalizationContext {
            customer_id: format!("{:?}", customer.primary_id),
            segment,
            recommendations: vec!["offer1".to_string(), "offer2".to_string()],
        }
    }

    pub fn get_recommendations(customer: &Customer) -> Vec<String> {
        let context = Self::personalize(customer);
        context.recommendations
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_personalization() {
        let customer = Customer::new(crate::cdp::CustomerId::Email("test@example.com".to_string()));
        let context = PersonalizationEngine::personalize(&customer);
        assert!(!context.recommendations.is_empty());
    }
}
