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

        let recommendations = if segments.is_empty() {
            // No segment membership yet: fall back to a small generic set.
            vec!["welcome_offer".to_string(), "getting_started_guide".to_string()]
        } else {
            let mut recs = Vec::new();
            for seg in &segments {
                for offer in Self::offers_for_segment(seg) {
                    if !recs.contains(&offer) {
                        recs.push(offer);
                    }
                }
            }
            recs
        };

        PersonalizationContext {
            customer_id: format!("{:?}", customer.primary_id),
            segment,
            recommendations,
        }
    }

    pub fn get_recommendations(customer: &Customer) -> Vec<String> {
        let context = Self::personalize(customer);
        context.recommendations
    }

    /// Offer catalog keyed by segment name. Unknown segments still get a
    /// segment-tailored offer (rather than silently falling through to the
    /// generic default), so every distinct segment produces distinct output.
    fn offers_for_segment(segment: &str) -> Vec<String> {
        match segment {
            "vip" => vec![
                "vip_early_access".to_string(),
                "concierge_support".to_string(),
            ],
            "at_risk" => vec![
                "win_back_discount".to_string(),
                "loyalty_bonus".to_string(),
            ],
            "new" => vec![
                "welcome_offer".to_string(),
                "onboarding_checklist".to_string(),
            ],
            "high_value" => vec![
                "premium_upsell".to_string(),
                "dedicated_account_manager".to_string(),
            ],
            other => vec![format!("{other}_offer"), "general_promo".to_string()],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cdp::{CustomerId, Segment};
    use std::collections::HashMap;

    fn customer_in_segment(email: &str, segment_name: &str) -> Customer {
        let mut customer = Customer::new(CustomerId::Email(email.to_string()));
        customer.add_to_segment(Segment {
            name: segment_name.to_string(),
            entered_at: 0,
            metadata: HashMap::new(),
        });
        customer
    }

    #[test]
    fn test_personalization() {
        let customer = Customer::new(CustomerId::Email("test@example.com".to_string()));
        let context = PersonalizationEngine::personalize(&customer);
        assert!(!context.recommendations.is_empty());
    }

    #[test]
    fn test_default_recommendations_for_customer_with_no_segments() {
        let customer = Customer::new(CustomerId::Email("nobody@example.com".to_string()));
        let recs = PersonalizationEngine::get_recommendations(&customer);
        assert_eq!(
            recs,
            vec!["welcome_offer".to_string(), "getting_started_guide".to_string()]
        );
    }

    #[test]
    fn test_recommendations_vary_by_segment() {
        let vip = customer_in_segment("vip@example.com", "vip");
        let at_risk = customer_in_segment("risk@example.com", "at_risk");

        let vip_recs = PersonalizationEngine::get_recommendations(&vip);
        let at_risk_recs = PersonalizationEngine::get_recommendations(&at_risk);

        assert_ne!(vip_recs, at_risk_recs);
        assert!(vip_recs.contains(&"vip_early_access".to_string()));
        assert!(at_risk_recs.contains(&"win_back_discount".to_string()));
    }
}
