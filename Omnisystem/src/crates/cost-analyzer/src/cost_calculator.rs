use crate::{PricingModel, CostRecord, CostResult, CostError, CostReport, TrendAnalysis, CostTrend};
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;

pub struct CostCalculator {
    pricing_models: Arc<DashMap<String, PricingModel>>,
    cost_records: Arc<DashMap<Uuid, CostRecord>>,
}

impl CostCalculator {
    pub fn new() -> Self {
        Self {
            pricing_models: Arc::new(DashMap::new()),
            cost_records: Arc::new(DashMap::new()),
        }
    }

    pub async fn register_pricing_model(&self, model: &PricingModel) -> CostResult<()> {
        self.pricing_models.insert(model.resource_type.clone(), model.clone());
        Ok(())
    }

    pub async fn calculate_cost(
        &self,
        tenant_id: &str,
        resource_type: &str,
        quantity: f64,
    ) -> CostResult<f64> {
        if let Some(model) = self.pricing_models.get(resource_type) {
            let total_cost = quantity * model.unit_cost;
            
            let record = CostRecord {
                record_id: Uuid::new_v4(),
                tenant_id: tenant_id.to_string(),
                resource_type: resource_type.to_string(),
                quantity,
                unit_cost: model.unit_cost,
                total_cost,
                timestamp: Utc::now(),
            };

            self.cost_records.insert(record.record_id, record);
            Ok(total_cost)
        } else {
            Err(CostError::PricingNotFound)
        }
    }

    pub async fn generate_report(
        &self,
        tenant_id: &str,
        start_date: chrono::DateTime<Utc>,
        end_date: chrono::DateTime<Utc>,
    ) -> CostResult<CostReport> {
        let mut total_cost = 0.0;
        let mut breakdown: std::collections::HashMap<String, f64> = std::collections::HashMap::new();

        for entry in self.cost_records.iter() {
            let record = entry.value();
            if record.tenant_id == tenant_id && record.timestamp >= start_date && record.timestamp <= end_date {
                total_cost += record.total_cost;
                *breakdown.entry(record.resource_type.clone()).or_insert(0.0) += record.total_cost;
            }
        }

        let cost_breakdown: Vec<(String, f64)> = breakdown.into_iter().collect();

        Ok(CostReport {
            report_id: Uuid::new_v4(),
            tenant_id: tenant_id.to_string(),
            period_start: start_date,
            period_end: end_date,
            total_cost,
            cost_breakdown,
            generated_at: Utc::now(),
        })
    }

    pub async fn analyze_trend(&self, tenant_id: &str) -> CostResult<TrendAnalysis> {
        let mut daily_costs: std::collections::HashMap<String, f64> = std::collections::HashMap::new();

        for entry in self.cost_records.iter() {
            let record = entry.value();
            if record.tenant_id == tenant_id {
                let date = record.timestamp.format("%Y-%m-%d").to_string();
                *daily_costs.entry(date).or_insert(0.0) += record.total_cost;
            }
        }

        let avg_daily_cost = if !daily_costs.is_empty() {
            daily_costs.values().sum::<f64>() / daily_costs.len() as f64
        } else {
            0.0
        };

        let peak_daily_cost = daily_costs.values().copied().fold(0.0, f64::max);

        let trend = if avg_daily_cost > 100.0 {
            CostTrend::Increasing
        } else if avg_daily_cost > 50.0 {
            CostTrend::Stable
        } else {
            CostTrend::Decreasing
        };

        Ok(TrendAnalysis {
            analysis_id: Uuid::new_v4(),
            tenant_id: tenant_id.to_string(),
            avg_daily_cost,
            peak_daily_cost,
            cost_trend: trend,
        })
    }

    pub fn record_count(&self) -> usize {
        self.cost_records.len()
    }
}

impl Default for CostCalculator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BillingPeriod;

    #[tokio::test]
    async fn test_register_pricing() {
        let calculator = CostCalculator::new();
        let model = PricingModel {
            model_id: Uuid::new_v4(),
            resource_type: "cpu".to_string(),
            unit_cost: 0.50,
            currency: "USD".to_string(),
            billing_period: BillingPeriod::Hourly,
        };

        calculator.register_pricing_model(&model).await.unwrap();
    }

    #[tokio::test]
    async fn test_calculate_cost() {
        let calculator = CostCalculator::new();
        let model = PricingModel {
            model_id: Uuid::new_v4(),
            resource_type: "memory".to_string(),
            unit_cost: 0.25,
            currency: "USD".to_string(),
            billing_period: BillingPeriod::Hourly,
        };

        calculator.register_pricing_model(&model).await.unwrap();
        let cost = calculator.calculate_cost("tenant1", "memory", 100.0).await.unwrap();
        assert_eq!(cost, 25.0);
    }

    #[tokio::test]
    async fn test_generate_report() {
        let calculator = CostCalculator::new();
        let model = PricingModel {
            model_id: Uuid::new_v4(),
            resource_type: "storage".to_string(),
            unit_cost: 0.10,
            currency: "USD".to_string(),
            billing_period: BillingPeriod::Monthly,
        };

        calculator.register_pricing_model(&model).await.unwrap();
        calculator.calculate_cost("tenant1", "storage", 500.0).await.unwrap();

        let now = Utc::now();
        let report = calculator.generate_report("tenant1", now - chrono::Duration::days(1), now).await.unwrap();
        assert!(report.total_cost > 0.0);
    }

    #[tokio::test]
    async fn test_analyze_trend() {
        let calculator = CostCalculator::new();
        let model = PricingModel {
            model_id: Uuid::new_v4(),
            resource_type: "network".to_string(),
            unit_cost: 0.15,
            currency: "USD".to_string(),
            billing_period: BillingPeriod::Hourly,
        };

        calculator.register_pricing_model(&model).await.unwrap();
        calculator.calculate_cost("tenant1", "network", 200.0).await.unwrap();

        let analysis = calculator.analyze_trend("tenant1").await.unwrap();
        assert!(analysis.avg_daily_cost >= 0.0);
    }
}
