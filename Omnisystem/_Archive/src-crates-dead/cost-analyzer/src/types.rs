use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PricingModel {
    pub model_id: Uuid,
    pub resource_type: String,
    pub unit_cost: f64,
    pub currency: String,
    pub billing_period: BillingPeriod,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum BillingPeriod {
    Hourly,
    Daily,
    Monthly,
    Annual,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CostRecord {
    pub record_id: Uuid,
    pub tenant_id: String,
    pub resource_type: String,
    pub quantity: f64,
    pub unit_cost: f64,
    pub total_cost: f64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BillingCycle {
    pub cycle_id: Uuid,
    pub tenant_id: String,
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    pub total_cost: f64,
    pub status: CycleStatus,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum CycleStatus {
    Open,
    Closed,
    Invoiced,
    Paid,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CostReport {
    pub report_id: Uuid,
    pub tenant_id: String,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub total_cost: f64,
    pub cost_breakdown: Vec<(String, f64)>,
    pub generated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrendAnalysis {
    pub analysis_id: Uuid,
    pub tenant_id: String,
    pub avg_daily_cost: f64,
    pub peak_daily_cost: f64,
    pub cost_trend: CostTrend,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum CostTrend {
    Increasing,
    Stable,
    Decreasing,
}
