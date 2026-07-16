use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CostAllocation {
    pub allocation_id: Uuid,
    pub parent_id: String,
    pub child_id: String,
    pub allocation_percent: f32,
    pub amount: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Invoice {
    pub invoice_id: Uuid,
    pub tenant_id: String,
    pub billing_period: String,
    pub total_amount: f64,
    pub line_items: Vec<LineItem>,
    pub generated_at: DateTime<Utc>,
    pub due_date: DateTime<Utc>,
    pub status: InvoiceStatus,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LineItem {
    pub item_id: Uuid,
    pub description: String,
    pub quantity: f64,
    pub unit_price: f64,
    pub total: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum InvoiceStatus {
    Draft,
    Issued,
    Paid,
    Overdue,
    Cancelled,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CostAllocationRule {
    pub rule_id: Uuid,
    pub name: String,
    pub allocation_method: AllocationMethod,
    pub active: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum AllocationMethod {
    Percentage,
    EqualShare,
    ProportionalToUsage,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChargebackStatement {
    pub statement_id: Uuid,
    pub tenant_id: String,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub total_cost: f64,
    pub allocated_costs: Vec<(String, f64)>,
    pub generated_at: DateTime<Utc>,
}
